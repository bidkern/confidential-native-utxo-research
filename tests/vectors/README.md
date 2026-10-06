# Frozen regression vectors

account.txt uses master seed [1;32], account 0, and the documented hardened path. payment.txt is the deterministic 10 BTC confidential input split into 3 BTC and 6.99999 BTC, fee 1,000 sats. tests/common/mod.rs specifies the synthetic coinbase ancestry; RNG is ChaCha20 seeded with [42;32].

Run the vectors binary for account data, or pass `payment` directly to that binary for transaction data. Fixtures are UTF-8 with LF newlines. They freeze this implementation's outputs and are not independently sourced BIP352 vectors or a privacy/security proof. Real wallets must never use these public seeds.
