#[path = "../../../../tests/common/mod.rs"]
mod common;
fn main() {
    if std::env::args().nth(1).as_deref() == Some("payment") {
        let (_, _, input) = common::confidential();
        let tx = confidential_tx::build(
            &[input],
            &[
                confidential_tx::Payment::Confidential(
                    address::Address::from_account(&common::account()),
                    3 * common::BTC,
                ),
                confidential_tx::Payment::Confidential(
                    address::Address::from_account(&common::account()),
                    699_999_000,
                ),
            ],
            1000,
            &mut common::rng(),
        )
        .unwrap();
        print!(
            "txid={}\nwtxid={}\nbase={}\nbytes={}\n",
            hex::encode(tx.txid()),
            hex::encode(tx.wtxid()),
            hex::encode(tx.base_bytes()),
            tx.bytes().len()
        );
        return;
    }
    let a = keys::Account::derive(&[1; 32], 0).unwrap();
    let addr = address::Address::from_account(&a);
    print!(
        "address={}\nscan={}\nspend={}\ngenerator={}\n",
        addr.encode(),
        hex::encode(addr.scan.serialize()),
        hex::encode(addr.spend.serialize()),
        hex::encode(commitments::generator().serialize())
    );
}
