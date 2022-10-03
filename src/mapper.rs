use round::round;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use thiserror::Error;

/// We should only be reading data from .csv files
pub const VALID_FILE_EXTENSION: &str = "csv";

/// A generic result type for ReaderError variants
pub type ReaderResult<T> = anyhow::Result<T, ReaderError>;

/// Custom error that wraps relevant reader errors
#[derive(Debug, Error, PartialEq)]
pub enum ReaderError {
    /// The file does not have a csv extension (.csv)
    #[error("The file must have a csv extension")]
    InvalidExtensionError,

    /// Withdrawal amount is bigger than available funds
    #[error("Failed withdrawal, amount: {0} is greater than available funds: {1}")]
    InsufficientFundsError(f32, f32),

    /// A file path to read transaction data from, wasn't provided
    #[error("An argument for file path must be provided, like so: cargo run -- some_file_path")]
    MissingArgError,

    /// The file doesn't exist
    #[error("Incorrect file path argument provided: {0}")]
    NonExistentFileError(String),
}

/// The various types of transactions
#[derive(Debug, Deserialize, Clone, Copy, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    /// A credit to the client's asset account
    Deposit,

    /// A debit to the client's asset account
    Withdrawal,

    /// A client's claim that a transaction was erroneous and should be reversed
    Dispute,

    /// A resolution to a dispute, releasing the associated held funds
    Resolve,

    /// The final state of a dispute and represents the client reversing a transaction
    Chargeback,
}

/// The relevant details of a transaction
#[derive(Debug, PartialEq)]
pub struct Transaction {
    /// A decimal value with a precision of up to four places past the decimal
    pub amount: f32,

    /// The type of transaction (e.g. dispute)
    pub current_state: TransactionType,
}

/// The structure of each row of data in the file
#[derive(Debug, Deserialize)]
pub struct Record {
    /// The type of transaction that occurred (e.g. deposit)
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,

    /// The unique identifier of the client
    #[serde(rename = "client")]
    pub client_id: u16,

    /// The unique identifier of the transaction
    #[serde(rename = "tx")]
    pub transaction_id: u32,

    /// A decimal value with a precision of up to four places past the decimal
    #[serde(default)]
    pub amount: Option<f32>,
}

/// The details of the client account that's output to std out
#[derive(Debug, Serialize)]
pub struct AccountRecord {
    /// The unique ID of the client
    pub client: u16,

    /// The available funds in the account
    #[serde(serialize_with = "serialize_with_precision")]
    pub available: f32,

    /// The held funds in the account
    #[serde(serialize_with = "serialize_with_precision")]
    pub held: f32,

    /// The total funds in the account
    #[serde(serialize_with = "serialize_with_precision")]
    pub total: f32,

    /// Whether the account is locked
    pub locked: bool,
}

/// The details of a client's account
#[derive(Debug, Default, PartialEq)]
pub struct Account {
    /// The total funds that are available for trading, staking, withdrawal, etc
    pub available_funds: f32,

    /// The total funds that are held for dispute
    pub held_funds: f32,

    /// The total funds that are available or held
    pub total_funds: f32,

    /// Whether the account is locked
    pub is_locked: bool,

    /// Data about the transactions that have been successfully executed (id, amount, current state)
    pub successful_transactions: HashMap<u32, Transaction>,
}

impl Account {
    /// Updates a client account when a deposit transaction occurs
    pub fn deposit(&mut self, amount: f32, transaction_id: u32) {
        self.available_funds += amount;
        self.total_funds += amount;
        self.successful_transactions.insert(
            transaction_id,
            Transaction {
                amount,
                current_state: TransactionType::Deposit,
            },
        );
    }

    /// Updates a client account when a withdrawal transaction occurs
    pub fn withdraw(&mut self, amount: f32, transaction_id: u32) -> ReaderResult<()> {
        // if a client account contains insufficient available funds, ensure the withdrawal fails
        if amount > self.available_funds {
            return Err(ReaderError::InsufficientFundsError(
                amount,
                self.available_funds,
            ));
        }

        self.available_funds -= amount;
        self.total_funds -= amount;
        self.successful_transactions.insert(
            transaction_id,
            Transaction {
                amount,
                current_state: TransactionType::Withdrawal,
            },
        );

        Ok(())
    }

    /// Updates a client account when a dispute transaction occurs
    pub fn dispute(&mut self, transaction_id: u32) {
        if let Some(transaction) = self.successful_transactions.get_mut(&transaction_id) {
            // we only want to update the account if the transaction hasn't been disputed yet
            if TransactionType::Dispute == transaction.current_state {
                return;
            }

            self.available_funds -= transaction.amount;
            self.held_funds += transaction.amount;
            transaction.current_state = TransactionType::Dispute;
        }
    }

    /// Updates a client account when a resolve transaction occurs
    pub fn resolve(&mut self, transaction_id: u32) {
        if let Some(transaction) = self.successful_transactions.get_mut(&transaction_id) {
            // we only want to update the account if the transaction is currently being disputed
            if TransactionType::Dispute == transaction.current_state {
                self.held_funds -= transaction.amount;
                self.available_funds += transaction.amount;
                transaction.current_state = TransactionType::Resolve;
            }
        }
    }

    /// Updates a client account when a chargeback transaction occurs
    pub fn chargeback(&mut self, transaction_id: u32) {
        if let Some(transaction) = self.successful_transactions.get_mut(&transaction_id) {
            // we only want to update the account if the transaction is currently being disputed
            if TransactionType::Dispute == transaction.current_state {
                self.held_funds -= transaction.amount;
                self.total_funds -= transaction.amount;
                // for chargebacks, immediately freeze the account
                self.is_locked = true;
                transaction.current_state = TransactionType::Chargeback;
            }
        }
    }
}

/// Ensures that f32 values are serialized with 4 decimals of precision
fn serialize_with_precision<S>(val: &f32, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
{
    s.serialize_f64(round(*val as f64, 4))
}