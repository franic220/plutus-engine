# *Welcome to Plutus*:heavy_dollar_sign:
Nicknamed after the God of wealth, Plutus is a toy payments engine for reading and writing financial transactions to files. These transactions include deposits, withdrawals, disputes, resolutions and chargebacks. Each type of transaction leads to a change in a client account:

- **deposit**: increase the balance
- **withdrawal**: decrease the balance
- **dispute**: decrease the available funds, increase the held funds
- **resolve**: increase the available funds by the amount previously disputed, and decrease the held funds by the amount previously disputed
- **chargeback**: decrease the held and total account funds by the amount previously disputed, immediately freeze (lock) the account

# **File Structure**:
**mapper.rs**
> Contains all of the relevant enums and structs. The enums are used to define custom error types (`ReaderError`) and transactions types (`TransactionType`). The structs are used for defining the structure of the account data.
---
**reader.rs**
> Contains all of the logic for reading and writing to files. The types defined in `mapper.rs` are utilized in this file to process transactions. Any tests associated with processing transaction data, are contained within this file.
---
**test-helpers.rs**
> Defines several reusable helper functions, for improving the readability of various test functions.
