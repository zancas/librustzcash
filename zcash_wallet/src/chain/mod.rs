use constants::UPGRADES_TEST;
use failure::Error;
use hex;
use pairing::bls12_381::Bls12;
use sapling_crypto::jubjub::{edwards, Unknown};
use std::collections::HashMap;
use std::fmt;
use zcash_primitives::{
    block::{BlockHash, BlockHeader},
    transaction::{Transaction, TxId},
};
use zip32::ExtendedFullViewingKey;

use types::ChainState;

pub mod sync;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Nullifier([u8; 32]);

impl fmt::Display for Nullifier {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut data = self.0.to_vec();
        data.reverse();
        formatter.write_str(&hex::encode(data))
    }
}

pub struct NoteCommitment([u8; 32]);

impl fmt::Display for NoteCommitment {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        let mut data = self.0.to_vec();
        data.reverse();
        formatter.write_str(&hex::encode(data))
    }
}

pub struct EncCiphertextFrag([u8; 52]);

impl fmt::Display for EncCiphertextFrag {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str(&hex::encode(&self.0[..]))
    }
}

pub struct CompactBlock {
    pub hash: BlockHash,
    pub prev: BlockHash,
    pub height: u32,
    pub header: Option<BlockHeader>,
    pub txs: Vec<CompactTx>,
}

pub struct CompactTx {
    pub txid: TxId,
    pub shielded_spends: Vec<Nullifier>,
    pub shielded_outputs: Vec<(
        NoteCommitment,
        edwards::Point<Bls12, Unknown>,
        EncCiphertextFrag,
    )>,
}

pub trait ChainSync {
    /// Start a synchronisation session. Returns a stream of compact blocks, along with
    /// the height the stream starts from if a rollback is necessary.
    fn start_session(
        &self,
        start_height: u32,
    ) -> Result<
        (
            Box<Iterator<Item = Result<CompactBlock, Error>>>,
            Option<u32>,
        ),
        Error,
    >;
}

struct BlockIndex {
    hash: BlockHash,
    prev: BlockHash,
    height: u32,
    txs: Vec<BlockTx>,
}

struct BlockTx {
    txid: TxId,
    tx: Option<Transaction>,
}

pub(crate) struct ChainManager {
    upgrades: [(u32, u32); 3],
    sync: Box<ChainSync>,
    blocks: Vec<BlockIndex>,
    xfvks: Vec<ExtendedFullViewingKey>,
    nullifiers: HashMap<Nullifier, Option<u32>>,
}

impl ChainManager {
    pub(crate) fn new(sync: Box<ChainSync>) -> Self {
        ChainManager {
            upgrades: UPGRADES_TEST,
            sync,
            blocks: vec![],
            xfvks: vec![],
            nullifiers: HashMap::new(),
        }
    }

    pub fn set_viewing_keys(&mut self, xfvks: Vec<ExtendedFullViewingKey>) {
        self.xfvks = xfvks;
    }

    pub fn sync(&mut self) -> Result<(), Error> {
        // TODO Assumes we never get sent the genesis block
        let cur_height = self.blocks.last().map(|b| b.height).unwrap_or(0);
        let (stream, rollback) = self.sync.start_session(cur_height + 1)?;

        // Handle rollbacks
        if let Some(height) = rollback {
            if let Some(split) = self
                .blocks
                .iter()
                .enumerate()
                .skip_while(|(_, b)| b.height <= height)
                .map(|(i, _)| i)
                .next()
            {
                self.blocks.split_off(split);
            }
        }

        // Skip over blocks we already have, or until we hit an error
        let mut stream = stream.skip_while(|res| match res {
            Ok(b) => b.height <= cur_height,
            Err(_) => false,
        });

        while let Some(block) = stream.next() {
            // Raise any errors
            let block = block?;

            let mut txs = vec![];
            for tx in block.txs {
                // Check for spent notes
                for nf in tx.shielded_spends {
                    if let Some(spent) = self.nullifiers.get_mut(&nf) {
                        if let Some(height) = spent {
                            return Err(format_err!(
                                "Double-spend received for nullifier {:?} at heights {} and {}",
                                nf,
                                height,
                                block.height
                            ));
                        } else {
                            *spent = Some(block.height);
                        }
                    }
                }

                // Check for incoming notes
                for (cmu, epk, enc_ct) in tx.shielded_outputs {
                    for xfvk in &self.xfvks {
                        // Trial-decrypt
                    }
                }

                txs.push(BlockTx {
                    txid: tx.txid,
                    tx: None,
                })
            }

            self.blocks.push(BlockIndex {
                hash: block.hash,
                prev: block.prev,
                height: block.height,
                txs,
            })
        }

        Ok(())
    }
}

impl ChainState for ChainManager {
    fn consensus_branch_id(&self) -> u32 {
        let cur_height = self.blocks.last().map(|b| b.height).unwrap_or(0);
        self.upgrades
            .iter()
            .skip_while(|(id, height)| *height > cur_height)
            .next()
            .expect("")
            .0
    }
}
