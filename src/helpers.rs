//! Common helper functions for scripts and tests

use std::{collections::BTreeSet, path::PathBuf, sync::Arc};

use cargo_miden::{run, OutputType};
use miden_client::{
    account::{
        component::{AuthRpoFalcon512, BasicWallet, NoAuth},
        Account, AccountId, AccountStorageMode, AccountType, StorageSlot,
    },
    auth::AuthSecretKey,
    builder::ClientBuilder,
    crypto::{FeltRng, SecretKey},
    keystore::FilesystemKeyStore,
    note::{
        Note, NoteExecutionHint, NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag,
        NoteType,
    },
    rpc::{Endpoint, TonicRpcClient},
    utils::Deserializable,
    Client, ClientError,
};
use miden_core::{Felt, FieldElement};
use miden_mast_package::Package;
use miden_objects::account::{
    AccountBuilder, AccountComponent, AccountComponentMetadata, AccountComponentTemplate,
};
use rand::{rngs::StdRng, RngCore};

/// Test setup configuration
pub struct ClientSetup {
    pub client: Client<FilesystemKeyStore<StdRng>>,
    pub keystore: Arc<FilesystemKeyStore<StdRng>>,
}

/// Initialize test infrastructure with client, keystore, and temporary directory
pub async fn setup_client() -> Result<ClientSetup, Box<dyn std::error::Error>> {
    // Initialize RPC connection
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    // Initialize keystore
    let keystore_path = PathBuf::from("./keystore");
    let keystore = Arc::new(FilesystemKeyStore::<StdRng>::new(keystore_path).unwrap());

    let store_path = PathBuf::from("./store.sqlite3");
    let client = ClientBuilder::new()
        .rpc(rpc_api)
        .sqlite_store(store_path.to_str().unwrap())
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .build()
        .await?;

    Ok(ClientSetup { client, keystore })
}

pub fn build_project_in_dir(dir: &std::path::Path, release: bool) -> Package {
    let profile: &str = if release { "--release" } else { "--debug" };
    // Compute manifest path string once
    let manifest_path = dir.join("Cargo.toml");
    let manifest_arg = manifest_path.to_string_lossy().to_string();

    let args = vec![
        "cargo".to_string(),
        "miden".to_string(),
        "build".to_string(),
        profile.to_string(),
        "--manifest-path".to_string(),
        manifest_arg.to_string(),
    ];

    let output = run(args.into_iter(), OutputType::Masm)
        .expect("Failed to compile with the release profile")
        .expect("'cargo miden build --release' should return Some(CommandOutput)");
    let expected_masm_path = match output {
        cargo_miden::CommandOutput::BuildCommandOutput { output } => match output {
            cargo_miden::BuildOutput::Masm { artifact_path } => artifact_path,
            other => panic!("Expected Masm output, got {other:?}"),
        },
        other => panic!("Expected BuildCommandOutput, got {other:?}"),
    };

    let package_bytes = std::fs::read(expected_masm_path).unwrap();
    Package::read_from_bytes(&package_bytes).unwrap()
}

/// Configuration for creating an account with a custom component
#[derive(Clone)]
pub struct AccountCreationConfig {
    pub account_type: AccountType,
    pub storage_mode: AccountStorageMode,
    pub storage_slots: Vec<StorageSlot>,
    pub supported_types: Option<Vec<AccountType>>,
}

impl Default for AccountCreationConfig {
    fn default() -> Self {
        Self {
            account_type: AccountType::RegularAccountImmutableCode,
            storage_mode: AccountStorageMode::Public,
            storage_slots: vec![],
            supported_types: None,
        }
    }
}

pub fn account_component_from_package(
    package: Arc<Package>,
    config: &AccountCreationConfig,
) -> AccountComponent {
    let account_component = match package.account_component_metadata_bytes.as_deref() {
        None => panic!("no account component metadata present"),
        Some(bytes) => {
            let metadata = AccountComponentMetadata::read_from_bytes(bytes).unwrap();
            let template =
                AccountComponentTemplate::new(metadata, package.unwrap_library().as_ref().clone());

            let component =
                AccountComponent::new(template.library().clone(), config.storage_slots.clone())
                    .unwrap();

            // Use supported types from config if provided, otherwise default to RegularAccountImmutableCode
            let supported_types = if let Some(types) = &config.supported_types {
                BTreeSet::from_iter(types.clone())
            } else {
                BTreeSet::from_iter([AccountType::RegularAccountImmutableCode])
            };

            component.with_supported_types(supported_types)
        }
    };
    account_component
}

/// Helper to create an account with a custom component from a package
pub async fn create_account_from_package(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    package: Arc<Package>,
    config: AccountCreationConfig,
) -> Result<Account, ClientError> {
    let account_component: AccountComponent = account_component_from_package(package, &config);

    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    // Sync client state to get latest block info
    let _sync_summary = client.sync_state().await.unwrap();

    let (account, seed) = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_component(account_component)
        .with_auth_component(NoAuth)
        .build()
        .unwrap();

    println!("Account ID: {:?}", account.id());

    client.add_account(&account, Some(seed), false).await?;

    Ok(account)
}

/// Configuration for creating a note
pub struct NoteCreationConfig {
    pub note_type: NoteType,
    pub tag: NoteTag,
    pub assets: miden_client::note::NoteAssets,
    pub inputs: Vec<Felt>,
    pub execution_hint: NoteExecutionHint,
    pub aux: Felt,
}

impl Default for NoteCreationConfig {
    fn default() -> Self {
        Self {
            note_type: NoteType::Public,
            tag: NoteTag::for_local_use_case(0, 0).unwrap(),
            assets: Default::default(),
            inputs: Default::default(),
            execution_hint: NoteExecutionHint::always(),
            aux: Felt::ZERO,
        }
    }
}

/// Helper to create a note from a compiled package
pub fn create_note_from_package(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    package: Arc<Package>,
    sender_id: AccountId,
    config: NoteCreationConfig,
) -> Note {
    let note_program = package.unwrap_program();
    let note_script = NoteScript::from_parts(
        note_program.mast_forest().clone(),
        note_program.entrypoint(),
    );

    let serial_num = client.rng().draw_word();
    let note_inputs = NoteInputs::new(config.inputs).unwrap();
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    let metadata = NoteMetadata::new(
        sender_id,
        config.note_type,
        config.tag,
        config.execution_hint,
        config.aux,
    )
    .unwrap();

    Note::new(config.assets, metadata, recipient)
}

pub async fn create_basic_wallet_account(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    keystore: Arc<FilesystemKeyStore<StdRng>>,
    config: AccountCreationConfig,
) -> Result<Account, ClientError> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = SecretKey::with_rng(client.rng());

    // Sync client state to get latest block info
    let _sync_summary = client.sync_state().await.unwrap();

    let builder = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet);

    let (account, seed) = builder.build().unwrap();
    client.add_account(&account, Some(seed), false).await?;
    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
        .unwrap();

    Ok(account)
}
