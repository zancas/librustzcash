use super::{
    components::{Amount, Script},
    sighash::signature_hash,
    Transaction,
};

mod data;


#[test]
fn tx_read_write() {
    let data = &self::data::tx_read_write::TX_READ_WRITE;
    let tx = Transaction::read(&data[..]).unwrap();
    let mut encoded = Vec::with_capacity(data.len());
    tx.write(&mut encoded).unwrap();
    assert_eq!(&data[..], &encoded[..]);
}


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
