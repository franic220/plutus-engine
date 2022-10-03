# *Welcome to Plutus*:heavy_dollar_sign:
Nicknamed after the God of wealth, Plutus is a toy payments engine for reading and writing financial transactions to files. These transactions include deposits, withdrawals, disputes, resolutions and chargebacks. Each type of transaction leads to a change in a client account.

- **deposit**: increase the balance
- **withdrawal**: decrease the balance
- **dispute**: decrease the available funds, increase the held funds
- **resolve**: increase the available funds by the amount previously disputed, and decrease the held funds by the amount previously disputed
- **chargeback**: decrease the held and total account funds by the amount previously disputed, immediately freeze (lock) the account
