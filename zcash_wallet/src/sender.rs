use failure::Error;
use zcash_primitives::transaction::Transaction;

use types::{SendResult, TxSender};

pub(crate) struct MockTxSender;

impl TxSender for MockTxSender {
    fn send(&self, _tx: &Transaction) -> Result<SendResult, Error> {
        Ok(SendResult::BestEffort)
    }
}
