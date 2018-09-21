use exonum_jsonrpc::client::Client;
use failure::Error;
use hex;
use hex_serde;
use pairing::bls12_381::Bls12;
use sapling_crypto::jubjub::edwards;
use serde::{
    de::{self, Deserialize, Deserializer, Unexpected, Visitor},
    ser::{Serialize, Serializer},
};
use std::fmt;
use zcash_primitives::JUBJUB;
use zcash_primitives::{
    block::{BlockHash, BlockHeaderData},
    transaction::TxId,
};

use chain::{ChainSync, CompactBlock, CompactTx, EncCiphertextFrag, NoteCommitment, Nullifier};

#[derive(Debug)]
pub struct BitcoinUint256([u8; 32]);

impl Serialize for BitcoinUint256 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut data = self.0.to_vec();
        data.reverse();
        serializer.serialize_str(&hex::encode(data))
    }
}

impl<'de> Deserialize<'de> for BitcoinUint256 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(BitcoinUint256Visitor)
    }
}

struct BitcoinUint256Visitor;

impl<'de> Visitor<'de> for BitcoinUint256Visitor {
    type Value = BitcoinUint256;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a 32-byte array")
    }

    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if s.len() == 64 {
            match hex::decode(s) {
                Ok(mut data) => {
                    data.reverse();
                    let mut hash = [0u8; 32];
                    hash.copy_from_slice(&data);
                    Ok(BitcoinUint256(hash))
                }
                Err(e) => Err(de::Error::invalid_value(Unexpected::Str(s), &self)),
            }
        } else {
            Err(de::Error::invalid_value(Unexpected::Str(s), &self))
        }
    }
}

#[derive(Deserialize)]
struct GetBlockResult {
    hash: BitcoinUint256,
    height: u32,
    version: i32,
    merkleroot: BitcoinUint256,
    finalsaplingroot: BitcoinUint256,
    tx: Vec<BitcoinUint256>,
    time: u32,
    nonce: BitcoinUint256,
    #[serde(with = "hex_serde")]
    solution: Vec<u8>,
    #[serde(with = "hex_serde")]
    bits: [u8; 4],
    previousblockhash: BitcoinUint256,
    #[serde(default)]
    nextblockhash: Option<BitcoinUint256>,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct ShieldedSpendResult {
    nullifier: BitcoinUint256,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct ShieldedOutputResult {
    cmu: BitcoinUint256,
    ephemeralKey: BitcoinUint256,
    #[serde(with = "hex_serde")]
    encCiphertext: Vec<u8>,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct GetRawTransactionResult {
    txid: BitcoinUint256,
    vShieldedSpend: Vec<ShieldedSpendResult>,
    vShieldedOutput: Vec<ShieldedOutputResult>,
}

/// A light client chain synchronisation backend using the `zcashd` JSON-RPC interface.
pub struct RpcChainSync {
    server: String,
    user: Option<String>,
    password: Option<String>,
}

impl RpcChainSync {
    pub fn new<S: Into<String>>(server: S, user: Option<S>, password: Option<S>) -> Self {
        RpcChainSync {
            server: server.into(),
            user: user.map(|s| s.into()),
            password: password.map(|s| s.into()),
        }
    }
}

impl ChainSync for RpcChainSync {
    fn start_session(
        &self,
        start_height: u32,
    ) -> Result<
        (
            Box<Iterator<Item = Result<CompactBlock, Error>>>,
            Option<u32>,
        ),
        Error,
    > {
        let client = Client::new(
            self.server.clone(),
            self.user.clone(),
            self.password.clone(),
        );

        // Find the first block hash we need to request
        let req = client.build_request("getblockhash".to_owned(), vec![json!(start_height)]);
        match client.send_request(&req) {
            Ok(ret) => match ret.into_result() {
                Ok(next_block_hash) => Ok((
                    Box::new(RpcChainSession {
                        client,
                        next_block_hash,
                    }),
                    None,
                )),
                Err(e) => Err(format_err!("zcashd returned error: {}", e)),
            },
            Err(e) => Err(format_err!("Error while fetching next block: {}", e)),
        }
    }
}

struct RpcChainSession {
    client: Client,
    next_block_hash: Option<BitcoinUint256>,
}

impl Iterator for RpcChainSession {
    type Item = Result<CompactBlock, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let next_block_hash = self.next_block_hash.take();

        // Fetch the next block
        let block = match next_block_hash {
            Some(hash) => {
                let req = self
                    .client
                    .build_request("getblock".to_owned(), vec![json!(hash)]);
                match self.client.send_request(&req) {
                    Ok(ret) => match ret.into_result::<GetBlockResult>() {
                        Ok(block) => block,
                        Err(e) => return Some(Err(format_err!("zcashd returned error: {}", e))),
                    },
                    Err(e) => {
                        return Some(Err(format_err!("Error while fetching next block: {}", e)))
                    }
                }
            }
            None => return None,
        };

        let mut txs = vec![];
        for txid in block.tx {
            // Fetch the transaction components
            let req = self
                .client
                .build_request("getrawtransaction".to_owned(), vec![json!(txid), json!(1)]);
            let tx = match self.client.send_request(&req) {
                Ok(ret) => match ret.into_result::<GetRawTransactionResult>() {
                    Ok(tx) => tx,
                    Err(e) => return Some(Err(format_err!("zcashd returned error: {}", e))),
                },
                Err(e) => return Some(Err(format_err!("Error while fetching transaction: {}", e))),
            };

            // Assemble the compact transaction
            txs.push(CompactTx {
                txid: TxId(tx.txid.0),
                shielded_spends: tx
                    .vShieldedSpend
                    .into_iter()
                    .map(|spend| Nullifier(spend.nullifier.0))
                    .collect(),
                shielded_outputs: tx
                    .vShieldedOutput
                    .into_iter()
                    .map(|output| {
                        let epk =
                            edwards::Point::<Bls12, _>::read(&output.ephemeralKey.0[..], &JUBJUB)
                                .map_err(|e| format_err!("Failed to decode epk: {}", e))
                                .unwrap();
                        let mut frag = [0u8; 52];
                        frag.copy_from_slice(&output.encCiphertext[..52]);
                        Ok((NoteCommitment(output.cmu.0), epk, EncCiphertextFrag(frag)))
                    }).collect::<Result<Vec<_>, _>>()?,
            })
        }

        // Store the next block hash we need to request
        self.next_block_hash = block.nextblockhash;

        // Assemble and return the compact block
        Some(Ok(CompactBlock {
            hash: BlockHash(block.hash.0),
            prev: BlockHash(block.previousblockhash.0.clone()),
            height: block.height,
            header: Some(
                BlockHeaderData {
                    version: block.version,
                    prev_block: block.previousblockhash.0,
                    merkle_root: block.merkleroot.0,
                    final_sapling_root: block.finalsaplingroot.0,
                    time: block.time,
                    bits: block.bits,
                    nonce: block.nonce.0,
                    solution: block.solution,
                }.freeze(),
            ),
            txs,
        }))
    }
}
