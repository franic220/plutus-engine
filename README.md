# *Welcome to Plutus*:heavy_dollar_sign:
Nicknamed after the God of wealth, Plutus is a toy payments engine for reading and writing financial transactions to files. These transactions include deposits, withdrawals, disputes, resolutions and chargebacks. Each type of transaction leads to a change in a client account:

- **deposit**: increase the balance
- **withdrawal**: decrease the balance
- **dispute**: decrease the available funds, increase the held funds
- **resolve**: increase the available funds by the amount previously disputed, and decrease the held funds by the amount previously disputed
- **chargeback**: decrease the held and total account funds by the amount previously disputed, immediately freeze (lock) the account

# **File Structure**:
![plutus-direcory-screenshot](https://user-images.githubusercontent.com/52143693/193697394-6bf10898-97cd-42a9-943f-a79b25ae46ed.png)

**main.rs**
> Executes `run`(found in `reader.rs`) to trigger the application. It also terminates execution when errors occur.
---
**mapper.rs**
> Contains all of the relevant enums and structs. The enums are used to define custom error types (`ReaderError`) and transaction types (`TransactionType`). The structs are used for defining the structure of the account data.
---
**reader.rs**
> Contains all of the logic for reading and writing to files. The types defined in `mapper.rs` are utilized in this file to process transactions. Any tests associated with processing transaction data, are contained within this file.
---
**test-helpers.rs**
> Defines several reusable helper functions, for improving the readability of various test functions.
---
**transactions.csv**
> Sample transaction data for the application to read. It includes rows with whitespace and rows with missing values.

# **Running Plutus Engine**:
Executing `cargo run -- transactions.csv > accounts.csv` in the plutus-engine directory will run the program and redirect output to `accounts.csv`. To view the output directly in the terminal, run `cargo run -- transactions.csv`. **The output in the terminal should look like so**:

![plutus-output-screenshot](https://user-images.githubusercontent.com/52143693/193699004-58b50ead-bda2-4b13-9f47-cb03a8329538.png)

# **Assumptions**:
When it comes to handling `dispute`, `resolve`, or `chargeback` transactions, it's assumed that the manner in which the account should be updated will always be the same. It could be argued, that the account should be updated based on the type of transaction it was originally. For example, if a client disputes a withdrawal, should the available funds remain as is and only the held funds be increased? 

If we did want to update the account based on what the original transaction type was, one way would be to extend the `Transaction` struct to have an `original_state` field. This field would store the original transaction type of the transaction. We could then access this field and use it to conditionally update the account.

# **Improvements**:
There are two account related structs; `Account` and `AccountRecord`. The `Account` struct is used to store the account information after we've deserialized it. `AccountRecord` is used to serialize the account data when writing to the file. One theoretical improvement could be to use only the `Account` struct. We could make `successful_transactions` optional. Then we can make use of serde's `rename` and `skip_serializing` field attributes.

Another improvement would be to add additional tests for `read_transactions_from_csv`. As well as, adding tests for `write_accounts_to_csv`, since there are none at the moment.

Presently we terminate execution whenever any error occurs. We could add logic to handle these errors when they propagate. This would allow us to not necessarily terminate execution, every time.
