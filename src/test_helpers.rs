use crate::mapper::{Account, Record, TransactionType};
use approx::assert_relative_eq;
use std::fs::File;
use std::io::{Error, Write};
use tempfile::{tempdir, TempDir};

/// Helper for validating relevant fields for a basic account test
#[allow(dead_code)]
pub fn assert_account(
    account: &Account,
    available_funds: f32,
    total_funds: f32,
    is_map_empty: bool,
) {
    assert_relative_eq!(account.available_funds, available_funds);
    assert_relative_eq!(account.total_funds, total_funds);
    assert!(is_map_empty);
}

/// Helper for validating the results of a chargeback test
#[allow(dead_code)]
pub fn assert_chargeback(
    account: &Account,
    held_funds: f32,
    total_funds: f32,
    is_locked: bool,
    transaction_id: u32,
    current_state: TransactionType,
) {
    assert_relative_eq!(account.held_funds, held_funds);
    assert_relative_eq!(account.total_funds, total_funds);
    assert!(is_locked);
    assert_eq!(
        account
            .successful_transactions
            .get(&transaction_id)
            .unwrap()
            .current_state,
        current_state
    );
}

/// Helper for validating the results of a dispute or resolve test
#[allow(dead_code)]
pub fn assert_dispute_or_resolve(
    account: &Account,
    transaction_id: u32,
    available_funds: f32,
    held_funds: f32,
    transaction_type: TransactionType,
) {
    assert_relative_eq!(account.available_funds, available_funds);
    assert_relative_eq!(account.held_funds, held_funds);
    assert_eq!(
        account
            .successful_transactions
            .get(&transaction_id)
            .unwrap()
            .current_state,
        transaction_type
    );
}

/// Helper for creating a Record
#[allow(dead_code)]
pub fn dummy_record(transaction_type: TransactionType, amount: Option<f32>) -> Record {
    Record {
        transaction_type,
        client_id: 0,
        transaction_id: 0,
        amount,
    }
}

/// Helper for creating a temporary file inside of `std::env::temp_dir()`
#[allow(dead_code)]
pub fn create_temp_file(file_name: &str) -> Result<(String, TempDir, File), Error> {
    // create a directory, add a temp file to it
    let dir = tempdir()?;
    let file_path = dir.path().join(file_name);
    let file = File::create(&file_path)?;

    Ok((file_path.into_os_string().into_string().unwrap(), dir, file))
}

/// Helper for adding transactions to a temporary file. Note that prior to writing the transactions,
/// a header row will be written to the file.
#[allow(dead_code)]
pub fn add_transactions_to_temp_file(
    transactions: Vec<&str>,
    file: &mut File,
) -> Result<(), Error> {
    // write headers to the file
    writeln!(file, "type,client,tx,amount")?;

    // write the transaction data to the file
    for transaction in transactions.into_iter() {
        writeln!(file, "{}", transaction)?;
    }

    Ok(())
}