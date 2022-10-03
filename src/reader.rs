use crate::mapper::{
    Account, AccountRecord, ReaderError, ReaderResult, Record, TransactionType,
    VALID_FILE_EXTENSION,
};
use anyhow::Result;
use csv::{ReaderBuilder, Trim};
use std::collections::HashMap;
use std::path::Path;
use std::{env, io};

/// Executes all of the logic for the payment engine. Reads data from a file, maps this data
/// to client's and their accounts, then prints to std out.
pub(crate) fn run() -> Result<()> {
    // read data from a csv
    let file_path = get_file_path(env::args().collect())?;
    let client_id_and_account_map: HashMap<u16, Account> = read_transactions_from_csv(&file_path)?;

    // write data to std out
    write_accounts_to_csv(client_id_and_account_map)?;

    Ok(())
}

/// Retrieves the file path from the provided command line arguments
fn get_file_path(args: Vec<String>) -> ReaderResult<String> {
    // error when an argument for file path wasn't provided
    if args.len() < 2 {
        return Err(ReaderError::MissingArgError);
    }

    let path = Path::new(&args[1]);

    // error when the file extension is incorrect
    match path.extension() {
        // if a file extension was provided, check that it's valid
        Some(extension) => {
            // non csv files are considered invalid
            if extension != VALID_FILE_EXTENSION {
                return Err(ReaderError::InvalidExtensionError);
            }
        }
        None => return Err(ReaderError::InvalidExtensionError),
    };

    // error when the file doesn't exist
    if !path.exists() {
        return Err(ReaderError::NonExistentFileError(args[1].to_string()));
    }

    Ok(args[1].to_string())
}

/// Reads transaction data from a csv and returns a HashMap of client_id -> Account
fn read_transactions_from_csv(file_path: &String) -> Result<HashMap<u16, Account>> {
    // build a CSV reader that accounts for whitespace, and missing values
    let mut reader = ReaderBuilder::new()
        .trim(Trim::Fields)
        .flexible(true)
        .from_path(file_path)?;

    // Iterate through the records. For each record, add an entry (Account) in the HashMap. If the entry
    // already exists, update its values using the record data
    let transactions_map = reader.deserialize().fold(
        HashMap::new(),
        |mut id_to_account_map_accum: HashMap<u16, Account>, result| {
            let record: Record = result
                .expect("Record should be structured like this: deposit,33,52,5492.9228 or this: resolve,21,2,");

            // if the Account isn't already in our HashMap, add it using Account::default()
            let entry = id_to_account_map_accum
                .entry(record.client_id)
                .or_insert_with(|| Account::default());

            process_transaction_record(&record, entry)
                .expect("failed to process transaction");

            id_to_account_map_accum
        },
    );

    Ok(transactions_map)
}

/// Triggers the relevant logic for updating a client's account, using a record (Record)
fn process_transaction_record(record: &Record, account: &mut Account) -> Result<(), anyhow::Error> {
    match record.transaction_type {
        TransactionType::Deposit => {
            // the amount field is optional, only process it when it's been defined
            if let Some(amount) = record.amount {
                account.deposit(amount, record.transaction_id)
            }
        }
        TransactionType::Withdrawal => {
            // the amount field is optional, only process it when it's been defined
            if let Some(amount) = record.amount {
                account.withdraw(amount, record.transaction_id)?;
            }
        }
        TransactionType::Dispute => account.dispute(record.transaction_id),
        TransactionType::Resolve => account.resolve(record.transaction_id),
        TransactionType::Chargeback => account.chargeback(record.transaction_id),
    }

    Ok(())
}

/// Writes client account data to a csv
fn write_accounts_to_csv(account_map: HashMap<u16, Account>) -> Result<()> {
    let mut writer = csv::Writer::from_writer(io::stdout());

    for (client_id, account) in account_map {
        // serialize AccountRecord as CSV record
        writer.serialize(AccountRecord {
            client: client_id,
            available: account.available_funds,
            held: account.held_funds,
            total: account.total_funds,
            locked: account.is_locked,
        })?;
    }

    writer.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::mapper::{Account, ReaderError, Transaction, TransactionType};
    use crate::reader::{get_file_path, process_transaction_record, read_transactions_from_csv};
    use crate::test_helpers::*;
    use approx::assert_relative_eq;
    use std::io::Error;

    // Tests that available_funds, total_funds and successful_transactions are increased as expected
    #[test]
    fn test_deposit() {
        let amount = 325.88;
        let transaction_id = 22;

        let expected_transaction = Transaction {
            amount,
            current_state: TransactionType::Deposit,
        };

        let mut account = Account::default();
        account.deposit(amount, transaction_id);

        assert_account(
            &account,
            amount,
            amount,
            !account.successful_transactions.is_empty(),
        );
        assert_eq!(
            account.successful_transactions.get(&transaction_id),
            Some(&expected_transaction)
        );
    }

    // Tests that attempting to withdraw an amount greater than the available funds triggers the appropriate error
    #[test]
    fn test_withdraw_greater_than_available() {
        let withdrawal_amount = 800.3196;
        let available_amount = 800.3195;

        let mut account = Account::default();
        account.available_funds = available_amount;

        let result = account.withdraw(800.3196, 0).unwrap_err();
        let expected_reader_error =
            ReaderError::InsufficientFundsError(withdrawal_amount, available_amount);

        assert_eq!(result, expected_reader_error);
        assert_eq!(account.available_funds, available_amount);
    }

    // Tests that available_funds, total_funds and successful_transactions are decreased as expected
    #[test]
    fn test_valid_withdraw() {
        let available_amount = 100.91;
        let total_funds_amount = 275.68;
        let decrease_amount = 50.0;
        let transaction_id = 1;

        let expected_available_funds = available_amount - decrease_amount;
        let expected_total_funds = total_funds_amount - decrease_amount;
        let expected_transaction = Transaction {
            amount: decrease_amount,
            current_state: TransactionType::Withdrawal,
        };

        let mut account = Account::default();
        account.available_funds = available_amount;
        account.total_funds = total_funds_amount;

        account
            .withdraw(decrease_amount, transaction_id)
            .expect("ok");

        assert_account(
            &account,
            expected_available_funds,
            expected_total_funds,
            !account.successful_transactions.is_empty(),
        );

        assert_eq!(
            account.successful_transactions.get(&transaction_id),
            Some(&expected_transaction)
        );
    }

    // Tests that available_funds and held_funds are left unchanged when a transaction is currently
    // being disputed
    #[test]
    fn test_add_existing_dispute() {
        let available_funds = 500.0;
        let held_funds = 74.25;
        let transaction_id = 5;

        let mut account = Account::default();
        account.available_funds = available_funds;
        account.held_funds = held_funds;
        account.successful_transactions.insert(
            transaction_id,
            Transaction {
                amount: 150.0,
                current_state: TransactionType::Dispute,
            },
        );

        account.dispute(transaction_id);

        // account should remain unchanged, since the transaction was already being disputed prior
        // to us executing add_dispute
        assert_dispute_or_resolve(
            &account,
            transaction_id,
            available_funds,
            held_funds,
            TransactionType::Dispute,
        )
    }

    // Tests that available_funds and held_funds are updated correctly, when a transaction is disputed
    #[test]
    fn test_valid_dispute() {
        let deposit_amount = 4_028.58;
        let transaction_id = 10;

        let mut account = Account::default();
        account.deposit(deposit_amount, transaction_id);

        account.dispute(transaction_id);

        assert_dispute_or_resolve(
            &account,
            transaction_id,
            0.0,
            deposit_amount,
            TransactionType::Dispute,
        )
    }

    // Tests that held_funds and available_funds are left unchanged when a transaction is not currently
    // being disputed
    #[test]
    fn test_resolve_non_disputed_transaction() {
        let deposit_amount = 1_000.0;
        let transaction_id = 10;

        let mut account = Account::default();
        account.deposit(deposit_amount, transaction_id);

        account.resolve(transaction_id);

        assert_dispute_or_resolve(
            &account,
            transaction_id,
            deposit_amount,
            0.0,
            TransactionType::Deposit,
        )
    }

    // Tests that held_funds and available_funds are updated correctly, when a previously disputed
    // transaction is resolved
    #[test]
    fn test_valid_resolve() {
        let deposit_amount = 1_000.0;
        let transaction_id = 10;

        let mut account = Account::default();
        account.deposit(deposit_amount, transaction_id);
        account.dispute(transaction_id);

        account.resolve(transaction_id);

        assert_dispute_or_resolve(
            &account,
            transaction_id,
            deposit_amount,
            0.0,
            TransactionType::Resolve,
        )
    }

    // Tests that an account is unchanged when a chargeback is attempted for a transaction that is
    // not currently being disputed
    #[test]
    fn test_chargeback_non_disputed_transaction() {
        let initial_amount = 1_000.94565;
        let increase_amount = 100.28313;
        let transaction_id = 8;

        let expected_amount = initial_amount + increase_amount;

        let mut account = Account::default();
        account.deposit(initial_amount, 0);
        account.deposit(increase_amount, transaction_id);

        account.chargeback(transaction_id);

        assert_relative_eq!(account.available_funds, expected_amount);
        assert_chargeback(
            &account,
            0.0,
            expected_amount,
            !account.is_locked,
            transaction_id,
            TransactionType::Deposit,
        );
    }

    // Tests that an account is correctly updated when a chargeback occurs
    #[test]
    fn test_valid_chargeback() {
        let initial_amount = 1_000.0;
        let increase_amount = 100.0;
        let transaction_id = 8;

        let mut account = Account::default();
        account.deposit(initial_amount, 0);
        account.deposit(increase_amount, transaction_id);
        account.dispute(transaction_id);

        account.chargeback(transaction_id);

        assert_chargeback(
            &account,
            0.0,
            initial_amount,
            account.is_locked,
            transaction_id,
            TransactionType::Chargeback,
        );
    }

    // Tests that the expected error is returned when the file path argument has not been provided
    #[test]
    fn test_get_file_path_missing_arg() {
        let env_args = vec![vec![], vec!["".to_string()]];

        for args in env_args.into_iter() {
            let result = get_file_path(args).unwrap_err();
            let expected_reader_error = ReaderError::MissingArgError;

            assert_eq!(result, expected_reader_error);
        }
    }

    // Tests that the expected error is returned when the file path leads to a non csv file
    #[test]
    fn test_get_file_path_invalid_extension() {
        let args = vec!["".to_string(), "someFile.txt".to_string()];
        let result = get_file_path(args).unwrap_err();

        let expected_reader_error = ReaderError::InvalidExtensionError;

        assert_eq!(result, expected_reader_error);
    }

    // Tests that the expected error is returned when the file path leads to a non existent file
    #[test]
    fn test_get_file_path_non_existent_file() {
        let non_existent_file = "nonExistentFile.csv";
        let args = vec!["".to_string(), non_existent_file.to_string()];
        let result = get_file_path(args).unwrap_err();

        let expected_reader_error =
            ReaderError::NonExistentFileError(non_existent_file.to_string());

        assert_eq!(result, expected_reader_error);
    }

    // Tests that get_file_path returns the correct file path, for an existing .csv file
    #[test]
    fn test_get_file_path() -> Result<(), Error> {
        // create a temporary file in a directory
        let file_name = "mock-transactions.csv";
        let (file_path_str, dir, file) = create_temp_file(file_name)?;

        let args = vec!["".to_string(), file_path_str];
        let result = get_file_path(args).unwrap();

        // we expect the result to end with the file name
        assert!(result.ends_with(file_name));

        drop(file);
        dir.close()?;

        Ok(())
    }

    // Tests that account data is correctly being read in from a file, for two different client accounts
    #[test]
    fn test_read_valid_transactions_from_csv_for_clients() -> Result<(), Error> {
        // create a temporary file in a directory
        let file_name = "transactions.csv";
        let (file_path_str, dir, mut file) = create_temp_file(file_name)?;

        // the transactions to add to our temporary file (type,client,tx,amount), there are 6
        // transactions for client id 24 and 6 transactions for client id 4
        let transactions = vec![
            "deposit,24,     1,    100.8453",
            "deposit,24,10,   250.21",
            "deposit,4,11,76.984",
            "withdrawal,4,     5,21.56",
            "deposit,24,8,13.612",
            "withdrawal,24,50, 50.0",
            "deposit,4,52,79.23",
            "deposit,4,53,31.84",
            "withdrawal,24,100,24.98",
            "withdrawal,24,57,       80.11",
            "withdrawal,4,3     ,47.81",
            "deposit,4,83,8.0",
        ];
        add_transactions_to_temp_file(transactions, &mut file)?;

        // By manually summing up the amounts from each element in the transactions array above, we
        // get the expected account balances for each client id (24 and 4)
        let expected_client_ids = [24, 4];
        let expected_account_funds = [209.5773, 126.684];

        // the transaction ids, transaction types and transaction amounts for each client. The first
        // element contains all the transaction ids for the first client account and the second element
        // contains all the transactions for the second client account
        let transaction_ids: [[u32; 6]; 2] = [[1, 10, 8, 50, 100, 57], [11, 5, 52, 53, 3, 83]];
        let transaction_types = [
            [
                TransactionType::Deposit,
                TransactionType::Deposit,
                TransactionType::Deposit,
                TransactionType::Withdrawal,
                TransactionType::Withdrawal,
                TransactionType::Withdrawal,
            ],
            [
                TransactionType::Deposit,
                TransactionType::Withdrawal,
                TransactionType::Deposit,
                TransactionType::Deposit,
                TransactionType::Withdrawal,
                TransactionType::Deposit,
            ],
        ];
        let transaction_amounts = [
            [100.8453, 250.21, 13.612, 50.0, 24.98, 80.11],
            [76.984, 21.56, 79.23, 31.84, 47.81, 8.0],
        ];

        let client_account_map = read_transactions_from_csv(&file_path_str).unwrap();

        for (index, expected_client_id) in expected_client_ids.iter().enumerate() {
            let account = client_account_map.get(expected_client_id).unwrap();
            let expected_funds = expected_account_funds[index];

            assert_account(
                &account,
                expected_funds,
                expected_funds,
                !account.successful_transactions.is_empty(),
            );

            // confirm that account transaction data has been correctly stored
            for (i, transaction_id) in transaction_ids[index].iter().enumerate() {
                let account_transaction =
                    account.successful_transactions.get(transaction_id).unwrap();

                let transaction_amount = transaction_amounts[index][i];
                let transaction_type = transaction_types[index][i];

                let expected_account_transaction = Transaction {
                    amount: transaction_amount,
                    current_state: transaction_type,
                };

                assert_eq!(*account_transaction, expected_account_transaction);
            }
        }

        drop(file);
        dir.close()?;

        Ok(())
    }

    // Tests that processing a deposit correctly updates an account
    #[test]
    fn test_process_deposit_transaction() {
        let amount = 1_500.90;
        let record = dummy_record(TransactionType::Deposit, Some(amount));

        let expected_transaction = Transaction {
            amount,
            current_state: TransactionType::Deposit,
        };

        let mut account = Account::default();

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            amount,
            amount,
            !account.successful_transactions.is_empty(),
        );
        assert_eq!(
            account.successful_transactions.get(&0),
            Some(&expected_transaction)
        );
    }

    // Tests that processing a deposit that does not contain an amount, does not update an account
    #[test]
    fn test_process_deposit_transaction_no_amount() {
        let record = dummy_record(TransactionType::Deposit, None);
        let mut account = Account::default();

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            0.0,
            0.0,
            account.successful_transactions.is_empty(),
        );
    }

    // Tests that processing a withdrawal correctly updates an account
    #[test]
    fn test_process_withdrawal_transaction() {
        let initial_balance = 200.0;
        let amount = 135.0;
        let record = dummy_record(TransactionType::Withdrawal, Some(amount));

        let expected_funds = initial_balance - amount;
        let expected_transaction = Transaction {
            amount,
            current_state: TransactionType::Withdrawal,
        };

        let mut account = Account::default();
        account.deposit(initial_balance, 1);

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            expected_funds,
            expected_funds,
            !account.successful_transactions.is_empty(),
        );
        assert_eq!(
            account.successful_transactions.get(&0),
            Some(&expected_transaction)
        );
    }

    // Tests that processing a withdrawal that does not contain an amount, does not update an account
    #[test]
    fn test_process_withdrawal_transaction_no_amount() {
        let record = dummy_record(TransactionType::Withdrawal, None);
        let mut account = Account::default();

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            0.0,
            0.0,
            account.successful_transactions.is_empty(),
        );
    }

    // Tests that processing a dispute correctly updates an account
    #[test]
    fn test_process_dispute_transaction() {
        let initial_balance = 200.0;
        let record = dummy_record(TransactionType::Dispute, None);

        let expected_transaction = Transaction {
            amount: initial_balance,
            current_state: TransactionType::Dispute,
        };

        let mut account = Account::default();
        account.deposit(initial_balance, 0);

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            0.0,
            initial_balance,
            !account.successful_transactions.is_empty(),
        );
        assert_eq!(account.held_funds, initial_balance);
        assert_eq!(
            account.successful_transactions.get(&0),
            Some(&expected_transaction)
        );
    }

    // Tests that processing a resolve correctly updates an account
    #[test]
    fn test_process_resolve_transaction() {
        let initial_balance = 200.0;
        let record = dummy_record(TransactionType::Resolve, None);

        let expected_transaction = Transaction {
            amount: initial_balance,
            current_state: TransactionType::Resolve,
        };

        let mut account = Account::default();
        account.deposit(initial_balance, 0);
        account.dispute(0);

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            initial_balance,
            initial_balance,
            !account.successful_transactions.is_empty(),
        );
        assert_eq!(account.held_funds, 0.0);
        assert_eq!(
            account.successful_transactions.get(&0),
            Some(&expected_transaction)
        );
    }

    // Tests that processing a chargeback correctly updates an account
    #[test]
    fn test_process_chargeback_transaction() {
        let initial_balance = 200.0;
        let record = dummy_record(TransactionType::Chargeback, None);

        let expected_transaction = Transaction {
            amount: initial_balance,
            current_state: TransactionType::Chargeback,
        };

        let mut account = Account::default();
        account.deposit(initial_balance, 0);
        account.dispute(0);

        process_transaction_record(&record, &mut account).expect("ok");

        assert_account(
            &account,
            0.0,
            0.0,
            !account.successful_transactions.is_empty(),
        );

        assert_eq!(account.held_funds, 0.0);
        assert!(account.is_locked);
        assert_eq!(
            account.successful_transactions.get(&0),
            Some(&expected_transaction)
        );
    }
}