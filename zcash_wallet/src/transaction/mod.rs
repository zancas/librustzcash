use pairing::bls12_381::Bls12;
use sapling_crypto::primitives::{Diversifier, Note};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};
use zcash_primitives::transaction::{Transaction, TxId};
use zcash_proofs::sapling::CommitmentTreeWitness;

mod builder;

pub use self::builder::Builder;

pub struct Memo([u8; 512]);

impl Default for Memo {
    fn default() -> Self {
        // Empty memo field indication per ZIP 302
        let mut memo = [0u8; 512];
        memo[0] = 0xF6;
        Memo(memo)
    }
}

/// The status of a transaction in the wallet.
#[derive(Debug, PartialEq)]
enum TxStatus {
    /// Not yet sent.
    Pending { expires: u32 },

    /// Not yet sent, and expiry height has passed.
    PendingExpired { expires: u32 },

    /// Sent and no internet, no server, no funds etc.
    /// TODO: Does this need its own state, or can we just use the Pending
    /// state plus external indicators?
    Failed,

    /// Sent and in mempool (0 confirmations).
    InMemPool { expires: u32 },

    /// Sent and network received, but not mined before expiry height.
    Expired { expires: u32 },

    /// Sent and mined, with 1-9 confirmations.
    Mined { expires: u32, mined: u32 },

    /// Sent and mined, with 10+ confirmations.
    Verified { expires: u32, mined: u32 },
}

pub struct WalletTx {
    txid: TxId,
    created_time: u32,
    status: TxStatus,

    /// Map from vShieldedOutput index to WalletNote
    pub notes: HashMap<usize, Rc<WalletNote>>,

    /// Present if the wallet created the transaction, or has requested it from
    /// the server.
    tx: Option<Transaction>,
}

impl WalletTx {
    pub fn new(txid: TxId, created_time: u32, expiry_height: u32) -> Self {
        WalletTx {
            txid,
            created_time,
            status: TxStatus::Pending {
                expires: expiry_height,
            },
            notes: HashMap::new(),
            tx: None,
        }
    }

    pub fn from_block(txid: TxId, created_time: u32, expiry_height: u32, mined: u32) -> Self {
        WalletTx {
            txid,
            created_time,
            status: TxStatus::Mined {
                expires: expiry_height,
                mined,
            },
            notes: HashMap::new(),
            tx: None,
        }
    }

    // pub fn from_tx(tx: Transaction, created_time: u32) -> Self {
    //     WalletTx {
    //         txid: tx.txid(),
    //         created_time,
    //         status: TxStatus::Pending {
    //             expires: tx.expiry_height,
    //         },
    //         notes: HashMap::new(),
    //         tx: Some(tx),
    //     }
    // }

    pub fn is_verified(&self) -> bool {
        match self.status {
            TxStatus::Verified {
                expires: _,
                mined: _,
            } => true,
            _ => false,
        }
    }

    pub fn sent(&mut self) {
        self.status = match self.status {
            TxStatus::Pending { expires } => TxStatus::InMemPool { expires },
            _ => panic!("Can only send pending transactions"),
        }
    }

    pub fn mined(&mut self, mined: u32) {
        self.status = match self.status {
            TxStatus::InMemPool { expires } if expires == 0 || mined <= expires => {
                TxStatus::Mined { expires, mined }
            }
            TxStatus::InMemPool { expires: _ } => {
                panic!("Can only mine transactions that are not expired")
            }
            _ => panic!("Can only mine transactions in the mempool"),
        }
    }

    /// Transition to the correct status for the given chain tip height.
    /// This function can be called sequentially with large jumps in height,
    /// but must be called separately for increases and decreases in chain tip
    /// height (during chain reorgs).
    pub fn chain_tip(&mut self, height: u32) {
        if let Some(status) = match &self.status {
            // Pending <--> PendingExpired
            &TxStatus::Pending { expires } if expires != 0 && height > expires => {
                Some(TxStatus::PendingExpired { expires })
            }
            &TxStatus::PendingExpired { expires } if height <= expires => {
                Some(TxStatus::Pending { expires })
            }

            // InMemPool <--> Expired
            &TxStatus::InMemPool { expires } if expires != 0 && height > expires => {
                Some(TxStatus::Expired { expires })
            }
            &TxStatus::Expired { expires } if height <= expires => {
                Some(TxStatus::InMemPool { expires })
            }

            // Mined --> InMemPool
            &TxStatus::Mined { expires, mined } if height < mined => {
                Some(TxStatus::InMemPool { expires })
            }

            // Verified --> InMemPool
            &TxStatus::Verified { expires, mined } if height < mined => {
                Some(TxStatus::InMemPool { expires })
            }

            // Mined <--> Verified
            &TxStatus::Mined { expires, mined } if height - mined >= 10 => {
                Some(TxStatus::Verified { expires, mined })
            }
            &TxStatus::Verified { expires, mined } if height - mined < 10 => {
                Some(TxStatus::Mined { expires, mined })
            }

            // All other cases, which are unaffected by changes in chain height
            _ => None,
        } {
            self.status = status;
        }
    }
}

pub struct WalletNote {
    /// The transaction in which this note was received
    tx_received: Weak<RefCell<WalletTx>>,

    /// The transaction in which this note was spent
    pub tx_spent: Option<Weak<RefCell<WalletTx>>>,

    pub diversifier: Diversifier,

    pub note: Note<Bls12>,

    memo: Option<Memo>,

    pub(crate) witness: Option<CommitmentTreeWitness>,
}

impl WalletNote {
    pub fn new(
        tx_received: Weak<RefCell<WalletTx>>,
        diversifier: Diversifier,
        note: Note<Bls12>,
        witness: CommitmentTreeWitness,
    ) -> Self {
        WalletNote {
            tx_received,
            tx_spent: None,
            diversifier,
            note,
            memo: None,
            witness: Some(witness),
        }
    }

    pub fn is_spendable(&self) -> bool {
        match self.tx_received.upgrade() {
            Some(tx) => tx.borrow().is_verified(),
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use zcash_primitives::transaction::TxId;

    use super::{TxStatus, WalletTx};

    #[test]
    fn state_machine() {
        let mut tx = WalletTx::new(TxId([0u8; 32]), 12345, 120);
        assert_eq!(tx.status, TxStatus::Pending { expires: 120 });

        // Pending <--> PendingExpired
        tx.chain_tip(120);
        assert_eq!(tx.status, TxStatus::Pending { expires: 120 });
        tx.chain_tip(121);
        assert_eq!(tx.status, TxStatus::PendingExpired { expires: 120 });
        tx.chain_tip(115);
        assert_eq!(tx.status, TxStatus::Pending { expires: 120 });

        // Pending --> InMemPool
        tx.sent();
        assert_eq!(tx.status, TxStatus::InMemPool { expires: 120 });

        // InMemPool <--> Expired
        tx.chain_tip(120);
        assert_eq!(tx.status, TxStatus::InMemPool { expires: 120 });
        tx.chain_tip(121);
        assert_eq!(tx.status, TxStatus::Expired { expires: 120 });
        tx.chain_tip(110);
        assert_eq!(tx.status, TxStatus::InMemPool { expires: 120 });

        // InMemPool --> Mined
        tx.mined(115);
        assert_eq!(
            tx.status,
            TxStatus::Mined {
                expires: 120,
                mined: 115
            }
        );

        // Mined <--> Verified
        tx.chain_tip(121);
        assert!(!tx.is_verified());
        assert_eq!(
            tx.status,
            TxStatus::Mined {
                expires: 120,
                mined: 115
            }
        );
        tx.chain_tip(125);
        assert!(tx.is_verified());
        assert_eq!(
            tx.status,
            TxStatus::Verified {
                expires: 120,
                mined: 115
            }
        );
        tx.chain_tip(124);
        assert!(!tx.is_verified());
        assert_eq!(
            tx.status,
            TxStatus::Mined {
                expires: 120,
                mined: 115
            }
        );

        // Mined --> InMemPool
        tx.chain_tip(115);
        assert_eq!(
            tx.status,
            TxStatus::Mined {
                expires: 120,
                mined: 115
            }
        );
        tx.chain_tip(114);
        assert_eq!(tx.status, TxStatus::InMemPool { expires: 120 });

        // InMemPool --> Mined --> Verified
        tx.mined(115);
        assert!(!tx.is_verified());
        assert_eq!(
            tx.status,
            TxStatus::Mined {
                expires: 120,
                mined: 115
            }
        );
        tx.chain_tip(130);
        assert!(tx.is_verified());
        assert_eq!(
            tx.status,
            TxStatus::Verified {
                expires: 120,
                mined: 115
            }
        );

        // Verified --> InMemPool
        tx.chain_tip(110);
        assert!(!tx.is_verified());
        assert_eq!(tx.status, TxStatus::InMemPool { expires: 120 });
    }
}
