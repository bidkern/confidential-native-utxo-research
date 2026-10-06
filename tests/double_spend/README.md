# Outpoint lifecycle checks

Executed by ../protocol.rs: explicit_double_spends_and_duplicate_inputs_rejected and malformed_proof_cannot_mutate_utxo_state. Tests reject same-transaction duplicate inputs, same-block repeated spends and later replay; block rejection preserves state.
