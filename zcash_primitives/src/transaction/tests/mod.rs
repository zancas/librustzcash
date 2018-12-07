use pairing::bls12_381::Bls12;
use rand::{thread_rng, Rng};
use sapling_crypto::{jubjub::FixedGenerators, redjubjub::PrivateKey};

use super::{
    components::{Amount, Script},
    sighash::signature_hash,
    Transaction, TransactionData,
};
use JUBJUB;

#[test]
fn tx_read_write() {
    let data = &self::data::tx_read_write::TX_READ_WRITE;
    let tx = Transaction::read(&data[..]).unwrap();

    let mut encoded = Vec::with_capacity(data.len());
    tx.write(&mut encoded).unwrap();
    assert_eq!(&data[..], &encoded[..]);
}

#[test]
fn tx_write_rejects_unexpected_joinsplit_pubkey() {
    // Succeeds without a JoinSplit pubkey
    {
        let tx = TransactionData::new().freeze();
        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_ok());
    }

    // Fails with an unexpected JoinSplit pubkey
    {
        let mut tx = TransactionData::new();
        tx.joinsplit_pubkey = Some([0; 32]);
        let tx = tx.freeze();

        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_err());
    }
}

#[test]
fn tx_write_rejects_unexpected_joinsplit_sig() {
    // Succeeds without a JoinSplit signature
    {
        let tx = TransactionData::new().freeze();
        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_ok());
    }

    // Fails with an unexpected JoinSplit signature
    {
        let mut tx = TransactionData::new();
        tx.joinsplit_sig = Some([0; 64]);
        let tx = tx.freeze();

        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_err());
    }
}

#[test]
fn tx_write_rejects_unexpected_binding_sig() {
    // Succeeds without a binding signature
    {
        let tx = TransactionData::new().freeze();
        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_ok());
    }

    // Fails with an unexpected binding signature
    {
        let rng = &mut thread_rng();
        let sk = PrivateKey::<Bls12>(rng.gen());
        let sig = sk.sign(
            b"Foo bar",
            rng,
            FixedGenerators::SpendingKeyGenerator,
            &JUBJUB,
        );

        let mut tx = TransactionData::new();
        tx.binding_sig = Some(sig);
        let tx = tx.freeze();

        let mut encoded = Vec::new();
        assert!(tx.write(&mut encoded).is_err());
    }
}

mod data;
#[test]
fn zip_0143() {
    for tv in self::data::zip_0143::make_test_vectors() {
        let tx = Transaction::read(&tv.tx[..]).unwrap();
        let transparent_input = if let Some(n) = tv.transparent_input {
            Some((n as usize, Script(tv.script_code), Amount(tv.amount)))
        } else {
            None
        };

        assert_eq!(
            signature_hash(&tx, tv.consensus_branch_id, tv.hash_type, transparent_input,),
            tv.sighash
        );
    }
}

#[test]
fn zip_0243() {
    for tv in self::data::zip_0243::make_test_vectors() {
        let tx = Transaction::read(&tv.tx[..]).unwrap();
        let transparent_input = if let Some(n) = tv.transparent_input {
            Some((n as usize, Script(tv.script_code), Amount(tv.amount)))
        } else {
            None
        };

        assert_eq!(
            signature_hash(&tx, tv.consensus_branch_id, tv.hash_type, transparent_input,),
            tv.sighash
        );
    }
}
