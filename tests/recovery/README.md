# Recovery and reorg checks

Executed by ../protocol.rs: hundred_payments_distinct_scannable_and_seed_recoverable (also spends restored outputs), independent_account_receipt_and_seed_recovery_range, withdrawal_reorg_and_intrablock_undo_recover_exact_state. Recovery starts with seed, explicit account range, and public blocks; it independently validates those blocks.
