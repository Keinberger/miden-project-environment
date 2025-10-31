use integration::helpers::{
    AccountCreationConfig, ClientSetup, NoteCreationConfig, build_project_in_dir,
    create_account_from_package, create_basic_wallet_account, create_note_from_package,
    setup_client,
};

use miden_client::{
    Felt, Word,
    account::StorageMap,
    transaction::{OutputNote, TransactionRequestBuilder},
};
use std::path::Path;
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // instantiate client
    let ClientSetup {
        mut client,
        keystore,
    } = setup_client().await.unwrap();

    let sync_summary = client.sync_state().await.unwrap();
    println!("Latest block: {}", sync_summary.block_num);

    let contract_package = Arc::new(build_project_in_dir(
        Path::new("../contracts/counter-account"),
        true,
    ));
    let note_package = Arc::new(build_project_in_dir(
        Path::new("../contracts/increment-note"),
        true,
    ));

    // Create the counter account with initial storage and no-auth auth component
    let count_storage_key = Word::from([Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(1)]);
    let initial_count = Word::from([Felt::new(0), Felt::new(0), Felt::new(0), Felt::new(0)]);
    let counter_cfg = AccountCreationConfig {
        storage_slots: vec![miden_client::account::StorageSlot::Map(
            StorageMap::with_entries([(count_storage_key, initial_count)]).unwrap(),
        )],
        ..Default::default()
    };

    // create counter account
    let counter_account =
        create_account_from_package(&mut client, contract_package.clone(), counter_cfg)
            .await
            .unwrap();

    // Create a separate sender account using only the BasicWallet component
    let sender_cfg = AccountCreationConfig::default();
    let sender_account = create_basic_wallet_account(&mut client, keystore.clone(), sender_cfg)
        .await
        .unwrap();
    println!("Sender account ID: {:?}", sender_account.id().to_hex());

    // build increment note
    let counter_note = create_note_from_package(
        &mut client,
        note_package.clone(),
        sender_account.id(),
        NoteCreationConfig::default(),
    );
    println!("Counter note hash: {:?}", counter_note.id().to_hex());

    // build and submit transaction to publish note
    let note_publish_request = TransactionRequestBuilder::new()
        .own_output_notes(vec![OutputNote::Full(counter_note.clone())])
        .build()
        .unwrap();
    let note_publish_tx_result = client
        .new_transaction(sender_account.id(), note_publish_request)
        .await
        .unwrap();
    client
        .submit_transaction(note_publish_tx_result.clone())
        .await?;
    client.sync_state().await?;

    println!(
        "Note publish transaction ID: {:?}",
        note_publish_tx_result.executed_transaction().id().to_hex()
    );

    let consume_note_request = TransactionRequestBuilder::new()
        .unauthenticated_input_notes([(counter_note.clone(), None)])
        .build()
        .unwrap();

    let consume_tx_result = client
        .new_transaction(counter_account.id(), consume_note_request)
        .await
        .unwrap();

    client
        .submit_transaction(consume_tx_result.clone())
        .await?;

    println!(
        "Consume transaction ID: {:?}",
        consume_tx_result.executed_transaction().id().to_hex()
    );

    println!(
        "Account delta: {:?}",
        consume_tx_result.executed_transaction().account_delta()
    );

    Ok(())
}
