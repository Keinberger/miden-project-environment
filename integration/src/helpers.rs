//! Common helper functions for scripts and tests

use std::{collections::BTreeSet, path::Path, sync::Arc};

use anyhow::{bail, Context, Result};
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
    Client,
};
use miden_core::{Felt, FieldElement};
use miden_mast_package::Package;
use miden_objects::account::{
    AccountBuilder, AccountComponent, AccountComponentMetadata, AccountComponentTemplate,
};
use rand::{rngs::StdRng, RngCore};

/// Test setup configuration containing initialized client and keystore
pub struct ClientSetup {
    pub client: Client<FilesystemKeyStore<StdRng>>,
    pub keystore: Arc<FilesystemKeyStore<StdRng>>,
}

/// Initializes test infrastructure with client and keystore
///
/// # Returns
/// A `ClientSetup` containing the initialized client and keystore
///
/// # Errors
/// Returns an error if RPC connection fails, keystore initialization fails,
/// or client building fails
pub async fn setup_client() -> Result<ClientSetup> {
    // Initialize RPC connection
    let endpoint = Endpoint::testnet();
    let timeout_ms = 10_000;
    let rpc_api = Arc::new(TonicRpcClient::new(&endpoint, timeout_ms));

    // Initialize keystore
    let keystore_path = std::path::PathBuf::from("../keystore");

    let keystore = Arc::new(
        FilesystemKeyStore::<StdRng>::new(keystore_path)
            .context("Failed to initialize keystore")?,
    );

    let store_path = std::path::PathBuf::from("../store.sqlite3");
    let store_path_str = store_path
        .to_str()
        .context("Store path contains invalid UTF-8")?;

    let client = ClientBuilder::new()
        .rpc(rpc_api)
        .sqlite_store(store_path_str)
        .authenticator(keystore.clone())
        .in_debug_mode(true.into())
        .build()
        .await
        .context("Failed to build Miden client")?;

    Ok(ClientSetup { client, keystore })
}

/// Builds a Miden project in the specified directory
///
/// # Arguments
/// * `dir` - Path to the directory containing the Cargo.toml
/// * `release` - Whether to build in release mode
///
/// # Returns
/// The compiled `Package`
///
/// # Errors
/// Returns an error if compilation fails or if the output is not in the expected format
pub fn build_project_in_dir(dir: &Path, release: bool) -> Result<Package> {
    let profile = if release { "--release" } else { "--debug" };
    let manifest_path = dir.join("Cargo.toml");
    let manifest_arg = manifest_path.to_string_lossy();

    let args = vec![
        "cargo",
        "miden",
        "build",
        profile,
        "--manifest-path",
        &manifest_arg,
    ];

    let output = run(args.into_iter().map(String::from), OutputType::Masm)
        .context("Failed to compile project")?
        .context("Cargo miden build returned None")?;

    let artifact_path = match output {
        cargo_miden::CommandOutput::BuildCommandOutput { output } => match output {
            cargo_miden::BuildOutput::Masm { artifact_path } => artifact_path,
            other => bail!("Expected Masm output, got {:?}", other),
        },
        other => bail!("Expected BuildCommandOutput, got {:?}", other),
    };

    let package_bytes = std::fs::read(&artifact_path).context(format!(
        "Failed to read compiled package from {}",
        artifact_path.display()
    ))?;

    Package::read_from_bytes(&package_bytes).context("Failed to deserialize package from bytes")
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

/// Creates an account component from a compiled package
///
/// # Arguments
/// * `package` - The compiled package containing account component metadata
/// * `config` - Configuration for account creation
///
/// # Returns
/// An `AccountComponent` configured according to the provided config
///
/// # Errors
/// Returns an error if the package doesn't contain account component metadata or deserialization fails
pub fn account_component_from_package(
    package: Arc<Package>,
    config: &AccountCreationConfig,
) -> Result<AccountComponent> {
    let bytes = package
        .account_component_metadata_bytes
        .as_deref()
        .context("Package missing account component metadata")?;

    let metadata = AccountComponentMetadata::read_from_bytes(bytes)
        .context("Failed to deserialize account component metadata")?;

    let library = package.unwrap_library();
    let template = AccountComponentTemplate::new(metadata, library.as_ref().clone());

    let component = AccountComponent::new(template.library().clone(), config.storage_slots.clone())
        .context("Failed to create account component")?;

    // Use supported types from config if provided, otherwise default to RegularAccountImmutableCode
    let supported_types = if let Some(types) = &config.supported_types {
        BTreeSet::from_iter(types.clone())
    } else {
        BTreeSet::from_iter([AccountType::RegularAccountImmutableCode])
    };

    Ok(component.with_supported_types(supported_types))
}

/// Creates an account with a custom component from a compiled package
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `package` - The compiled package containing the account component
/// * `config` - Configuration for account creation
///
/// # Returns
/// The created `Account`
///
/// # Errors
/// Returns an error if account creation or client operations fail
pub async fn create_account_from_package(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    package: Arc<Package>,
    config: AccountCreationConfig,
) -> Result<Account> {
    let account_component = account_component_from_package(package, &config)
        .context("Failed to create account component from package")?;

    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let (account, seed) = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_component(account_component)
        .with_auth_component(NoAuth)
        .build()
        .context("Failed to build account")?;

    println!("Account ID: {:?}", account.id());

    client
        .add_account(&account, Some(seed), false)
        .await
        .context("Failed to add account to client")?;

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
            // Note: This should never fail for valid inputs (0, 0)
            tag: NoteTag::for_local_use_case(0, 0)
                .expect("Failed to create default note tag with (0, 0)"),
            assets: Default::default(),
            inputs: Default::default(),
            execution_hint: NoteExecutionHint::always(),
            aux: Felt::ZERO,
        }
    }
}

/// Creates a note from a compiled package
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `package` - The compiled package containing the note script
/// * `sender_id` - The ID of the account sending the note
/// * `config` - Configuration for note creation
///
/// # Returns
/// The created `Note`
///
/// # Errors
/// Returns an error if note creation fails
pub fn create_note_from_package(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    package: Arc<Package>,
    sender_id: AccountId,
    config: NoteCreationConfig,
) -> Result<Note> {
    let note_program = package.unwrap_program();
    let note_script = NoteScript::from_parts(
        note_program.mast_forest().clone(),
        note_program.entrypoint(),
    );

    let serial_num = client.rng().draw_word();
    let note_inputs = NoteInputs::new(config.inputs).context("Failed to create note inputs")?;
    let recipient = NoteRecipient::new(serial_num, note_script, note_inputs);

    let metadata = NoteMetadata::new(
        sender_id,
        config.note_type,
        config.tag,
        config.execution_hint,
        config.aux,
    )
    .context("Failed to create note metadata")?;

    Ok(Note::new(config.assets, metadata, recipient))
}

/// Creates a basic wallet account with authentication
///
/// # Arguments
/// * `client` - The Miden client instance
/// * `keystore` - The keystore for storing authentication keys
/// * `config` - Configuration for account creation
///
/// # Returns
/// The created `Account` with basic wallet functionality
///
/// # Errors
/// Returns an error if account creation, key generation, or keystore operations fail
pub async fn create_basic_wallet_account(
    client: &mut Client<FilesystemKeyStore<StdRng>>,
    keystore: Arc<FilesystemKeyStore<StdRng>>,
    config: AccountCreationConfig,
) -> Result<Account> {
    let mut init_seed = [0_u8; 32];
    client.rng().fill_bytes(&mut init_seed);

    let key_pair = SecretKey::with_rng(client.rng());

    let builder = AccountBuilder::new(init_seed)
        .account_type(config.account_type)
        .storage_mode(config.storage_mode)
        .with_auth_component(AuthRpoFalcon512::new(key_pair.public_key()))
        .with_component(BasicWallet);

    let (account, seed) = builder
        .build()
        .context("Failed to build basic wallet account")?;

    client
        .add_account(&account, Some(seed), false)
        .await
        .context("Failed to add account to client")?;

    keystore
        .add_key(&AuthSecretKey::RpoFalcon512(key_pair))
        .context("Failed to add key to keystore")?;

    Ok(account)
}
