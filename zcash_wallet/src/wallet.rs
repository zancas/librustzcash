//! Implementation of a Zcash light wallet.

use types::{ChainState, KeyStore, TxProver, TxSender};

pub struct Wallet {
    ks: Box<KeyStore>,
    cs: Box<ChainState>,
    prover: Box<TxProver>,
    sender: Box<TxSender>,
}

impl Wallet {
    pub fn new(
        ks: Box<KeyStore>,
        cs: Box<ChainState>,
        prover: Box<TxProver>,
        sender: Box<TxSender>,
    ) -> Self {
        Wallet {
            ks,
            cs,
            prover,
            sender,
        }
    }
}
