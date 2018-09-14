//! Implementation of a Zcash light wallet.

use account::{Account, AccountId};
use types::{ChainState, KeyStore, TxProver, TxSender};

pub struct Wallet {
    coin_type: u32,
    ks: Box<KeyStore>,
    cs: Box<ChainState>,
    prover: Box<TxProver>,
    sender: Box<TxSender>,
    accounts: Vec<Account>,
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
}

#[cfg(test)]
mod tests {
    use super::{AccountId, Wallet};
    use address::encode_payment_address;
    use chain::{sync::MockChainSync, ChainManager};
    use constants;
    use keystore::LocalKeyStore;
    use prover::MockTxProver;
    use sender::MockTxSender;

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
}
