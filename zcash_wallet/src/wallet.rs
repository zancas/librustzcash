//! Implementation of a Zcash light wallet.

use failure::Error;
use pairing::bls12_381::Bls12;
use rand::{OsRng, Rand};
use sapling_crypto::primitives::{Diversifier, PaymentAddress};
use sapling_crypto::{jubjub::fs::Fs, primitives::Note};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use zcash_primitives::{
    merkle_tree::IncrementalWitness,
    transaction::{components::Amount, TxId},
};

use account::{Account, AccountId};
use transaction::{self, Memo, WalletNote, WalletTx};
use types::{ChainState, KeyStore, SendResult, TxProver, TxSender};

pub struct Wallet {
    coin_type: u32,
    ks: Box<KeyStore>,
    cs: Box<ChainState>,
    prover: Box<TxProver>,
    sender: Box<TxSender>,
    accounts: Vec<Account>,
    transactions: HashMap<TxId, Rc<RefCell<WalletTx>>>,
}

impl Wallet {
    pub fn new(
        coin_type: u32,
        ks: Box<KeyStore>,
        cs: Box<ChainState>,
        prover: Box<TxProver>,
        sender: Box<TxSender>,
    ) -> Self {
        Wallet {
            coin_type,
            ks,
            cs,
            prover,
            sender,
            accounts: vec![],
            transactions: HashMap::new(),
        }
    }

    /// Creates a new account inside the wallet.
    pub fn create_account(&mut self, label: String) -> AccountId {
        // Generate accounts in-order
        let account_id = self.accounts.len() as u32;

        self.accounts.push(Account::new(
            label,
            self.ks.xfvk(self.coin_type, account_id),
        ));

        AccountId(account_id)
    }

    /// Returns a list of the accounts in this wallet.
    pub fn accounts(&self) -> &Vec<Account> {
        &self.accounts
    }

    /// Sends funds from an account to a recipient, with an optional memo.
    pub fn make_payment(
        &self,
        from: AccountId,
        to: PaymentAddress<Bls12>,
        value: Amount,
        memo: Option<Memo>,
    ) -> Result<SendResult, Error> {
        let mut rng = OsRng::new().expect("should be able to construct RNG");

        // Fetch the account
        let account = match self.accounts().get(from.0 as usize) {
            Some(account) => account,
            None => return Err(format_err!("Unknown account: {}", from.0)),
        };

        // Select notes to spend
        let notes = account.select_notes(value)?;

        // Create the transaction
        let mut builder = transaction::Builder::new(self.coin_type);
        for (diversifier, note, witness) in notes {
            let ar = Fs::rand(&mut rng);
            builder.add_sapling_spend(from, diversifier, note, ar, witness)?;
        }
        builder.add_sapling_output(account.xfvk.fvk.ovk, to, value, Fs::rand(&mut rng), memo)?;
        let tx = builder.build(self.cs.consensus_branch_id(), &self.ks, &self.prover)?;

        // Add transaction to wallet
        // TODO (for now, we'll see it arrive via self.zcs)

        // Send the transaction
        self.sender.send(&tx)
    }

    //
    // Internal helper functions
    //

    fn incoming_viewing_keys(&self) -> Vec<(AccountId, Fs)> {
        self.accounts
            .iter()
            .enumerate()
            .map(|(i, a)| (AccountId(i as u32), a.ivk()))
            .collect()
    }

    fn received_tx(
        &mut self,
        block_height: u32,
        txid: TxId,
        created_time: u32,
        expiry_height: u32,
        notes: Vec<(
            AccountId,
            usize,
            Diversifier,
            Note<Bls12>,
            IncrementalWitness,
        )>,
    ) {
        let mut tx = WalletTx::from_block(txid, created_time, expiry_height, block_height);
        tx.notes.reserve(notes.len());

        let tx = Rc::new(RefCell::new(tx));
        notes.into_iter().for_each(|(id, i, d, n, w)| {
            let note = Rc::new(WalletNote::new(Rc::downgrade(&tx), d, n, w));
            self.accounts
                .get_mut(id.0 as usize)
                .unwrap()
                .notes
                .push(note.clone());
            tx.borrow_mut().notes.insert(i, note);
        });

        self.transactions.insert(txid, tx);
    }

    /// Update transactions in the wallet to the correct status for the given
    /// chain tip height.
    ///
    /// This function can be called sequentially with large jumps in height,
    /// but must be called separately for increases and decreases in chain tip
    /// height (during chain reorgs).
    fn chain_tip(&mut self, height: u32) {
        self.transactions
            .values_mut()
            .for_each(|tx| tx.borrow_mut().chain_tip(height))
    }
}

#[cfg(test)]
mod tests {
    use pairing::{bls12_381::Bls12, PrimeField};
    use rand::{SeedableRng, XorShiftRng};
    use sapling_crypto::{
        jubjub::edwards,
        primitives::{Diversifier, PaymentAddress},
    };
    use std::path::Path;
    use zcash_primitives::{
        merkle_tree::{CommitmentTree, IncrementalWitness, Node},
        transaction::{components::Amount, TxId},
        JUBJUB,
    };

    use super::{AccountId, Wallet};
    use address::encode_payment_address;
    use chain::{sync::MockChainSync, ChainManager};
    use constants;
    use keystore::LocalKeyStore;
    use prover::{LocalTxProver, MockTxProver};
    use sender::MockTxSender;
    use tests::{fake_note, fake_witness};
    use types::SendResult;

    #[test]
    fn create_account() {
        let mut wallet = Wallet::new(
            constants::COIN_TYPE_TEST,
            Box::new(LocalKeyStore::from_seed(&[0u8; 32])),
            Box::new(ChainManager::new(Box::new(MockChainSync {}))),
            Box::new(MockTxProver {}),
            Box::new(MockTxSender {}),
        );

        assert_eq!(wallet.accounts().len(), 0);
        assert_eq!(wallet.create_account("test".to_owned()), AccountId(0));
        assert_eq!(wallet.accounts().len(), 1);
        assert_eq!(wallet.accounts()[0].label, "test");
        assert_eq!(
            encode_payment_address(
                constants::HRP_SAPLING_EXTENDED_SPENDING_KEY_TEST,
                &wallet.accounts()[0].default_address()
            ),
            "ztestsapling1cfwr43u8m9k4yfc94qfq4pu44avsr5jx2mm9sj40fu4adeep3ghamvdhg6kftv62t0ys7w3tank"
        );
    }

    #[test]
    fn balance() {
        let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);

        let mut wallet = Wallet::new(
            constants::COIN_TYPE_TEST,
            Box::new(LocalKeyStore::from_seed(&[0u8; 32])),
            Box::new(ChainManager::new(Box::new(MockChainSync {}))),
            Box::new(MockTxProver {}),
            Box::new(MockTxSender {}),
        );
        let a1 = wallet.create_account("test1".to_owned());
        let a2 = wallet.create_account("test2".to_owned());
        let ivk1 = wallet.accounts()[a1.0 as usize].ivk();
        let ivk2 = wallet.accounts()[a2.0 as usize].ivk();

        // Receive a transaction at height 100
        let d = Diversifier([0u8; 11]);
        wallet.received_tx(
            100,
            TxId([0u8; 32]),
            12345,
            0,
            vec![
                (
                    a1,
                    0,
                    d.clone(),
                    fake_note(ivk1, &d, 6, rng),
                    fake_witness(),
                ),
                (
                    a2,
                    1,
                    d.clone(),
                    fake_note(ivk2, &d, 4, rng),
                    fake_witness(),
                ),
            ],
        );

        // Receive a transaction at height 102
        wallet.received_tx(
            102,
            TxId([1u8; 32]),
            34567,
            0,
            vec![(
                a1,
                0,
                d.clone(),
                fake_note(ivk1, &d, 5, rng),
                fake_witness(),
            )],
        );

        // All value is currently pending
        assert_eq!(wallet.accounts()[0].balances(), (Amount(0), Amount(11)));
        assert_eq!(wallet.accounts()[1].balances(), (Amount(0), Amount(4)));

        // At height 105, all value is still pending
        wallet.chain_tip(105);
        assert_eq!(wallet.accounts()[0].balances(), (Amount(0), Amount(11)));
        assert_eq!(wallet.accounts()[1].balances(), (Amount(0), Amount(4)));

        // At height 110, the value from the first transaction is spendable
        wallet.chain_tip(110);
        assert_eq!(wallet.accounts()[0].balances(), (Amount(6), Amount(5)));
        assert_eq!(wallet.accounts()[1].balances(), (Amount(4), Amount(0)));

        // A rollback due to reorg makes the value pending again
        wallet.chain_tip(109);
        assert_eq!(wallet.accounts()[0].balances(), (Amount(0), Amount(11)));
        assert_eq!(wallet.accounts()[1].balances(), (Amount(0), Amount(4)));

        // At height 112, all value is spendable
        wallet.chain_tip(112);
        assert_eq!(wallet.accounts()[0].balances(), (Amount(11), Amount(0)));
        assert_eq!(wallet.accounts()[1].balances(), (Amount(4), Amount(0)));
    }

    #[test]
    fn make_payment() {
        let rng = &mut XorShiftRng::from_seed([0x3dbe6259, 0x8d313d76, 0x3237db17, 0xe5bc0654]);

        let mut wallet = Wallet::new(
            constants::COIN_TYPE_TEST,
            Box::new(LocalKeyStore::from_seed(&[0u8; 32])),
            Box::new(ChainManager::new(Box::new(MockChainSync {}))),
            Box::new(LocalTxProver::new(
                Path::new("/home/str4d/.zcash-params/sapling-spend.params"),
                "8270785a1a0d0bc77196f000ee6d221c9c9894f55307bd9357c3f0105d31ca63991ab91324160d8f53e2bbd3c2633a6eb8bdf5205d822e7f3f73edac51b2b70c",
                Path::new("/home/str4d/.zcash-params/sapling-output.params"),
                "657e3d38dbb5cb5e7dd2970e8b03d69b4787dd907285b5a7f0790dcc8072f60bf593b32cc2d1c030e00ff5ae64bf84c5c3beb84ddc841d48264b4a171744d028",
            )),
            // Box::new(MockTxProver {}),
            Box::new(MockTxSender {}),
        );
        let account = wallet.create_account("test1".to_owned());
        let ivk = wallet.accounts()[account.0 as usize].ivk();

        let d = Diversifier([0u8; 11]);
        let mut tree = CommitmentTree::new();

        let n1 = fake_note(ivk, &d, 6 * 100_000_000, rng);
        let cm1 = Node::new(n1.cm(&JUBJUB).into_repr());
        tree.append(cm1).unwrap();
        let mut w1 = IncrementalWitness::from_tree(&tree);

        let n2 = fake_note(ivk, &d, 4 * 100_000_000, rng);
        let cm2 = Node::new(n2.cm(&JUBJUB).into_repr());
        tree.append(cm2).unwrap();
        w1.append(cm2).unwrap();
        let mut w2 = IncrementalWitness::from_tree(&tree);

        let n3 = fake_note(ivk, &d, 5 * 100_000_000, rng);
        let cm3 = Node::new(n3.cm(&JUBJUB).into_repr());
        tree.append(cm3).unwrap();
        w1.append(cm3).unwrap();
        w2.append(cm3).unwrap();
        let w3 = IncrementalWitness::from_tree(&tree);

        // Receive a transaction at height 100
        wallet.received_tx(
            100,
            TxId([0u8; 32]),
            12345,
            0,
            vec![
                (account, 0, d.clone(), n1, w1),
                (account, 1, d.clone(), n2, w2),
                (account, 2, d.clone(), n3, w3),
            ],
        );

        wallet.chain_tip(110);
        assert_eq!(
            wallet.accounts()[0].balances(),
            (Amount(15 * 100_000_000), Amount(0))
        );

        let addr = PaymentAddress {
            diversifier: Diversifier([0u8; 11]),
            pk_d: edwards::Point::<Bls12, _>::rand(rng, &JUBJUB).mul_by_cofactor(&JUBJUB),
        };

        match wallet.make_payment(account, addr, Amount(8 * 100_000_000), None) {
            Ok(result) => assert_eq!(result, SendResult::BestEffort),
            Err(e) => panic!("Failed to make payment: {}", e),
        }
    }
}
