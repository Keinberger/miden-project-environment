//! Deployment script for counter contracts
//! 🎉 Ultra-clean: Just a file in scripts/ folder, no nested directories!

use helpers::setup_script;

use miden_client::{
    account::{AccountIdAddress, Address, AddressInterface, StorageMap, StorageSlot},
    asset::FungibleAsset,
    builder::ClientBuilder,
    crypto::FeltRng,
    keystore::FilesystemKeyStore,
    note::{
        create_p2id_note, Note, NoteAssets, NoteExecutionHint, NoteInputs, NoteMetadata,
        NoteRecipient, NoteTag, NoteType,
    },
    rpc::{Endpoint, TonicRpcClient},
    transaction::{OutputNote, TransactionKernel, TransactionRequestBuilder},
    Felt, Word,
};
use miden_lib::utils::ScriptBuilder;
use miden_objects::{
    account::{AccountComponent, NetworkId},
    assembly::Assembler,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // instantiate client
    let client = setup_script()?;
    // create counter account

    // build increment note

    // craft transaction to consume note
    Ok(())
}
