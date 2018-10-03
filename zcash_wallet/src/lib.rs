#[macro_use]
extern crate failure;

#[macro_use]
extern crate lazy_static;

extern crate bech32;
extern crate bellman;
extern crate hex;
extern crate pairing;
extern crate rand;
extern crate sapling_crypto;
extern crate zcash_primitives;
extern crate zcash_proofs;
extern crate zip32;

#[cfg(feature = "jsonrpc")]
extern crate exonum_jsonrpc;

#[cfg(feature = "jsonrpc")]
extern crate hex_serde;

#[cfg(feature = "jsonrpc")]
extern crate serde;

#[cfg(feature = "jsonrpc")]
#[macro_use]
extern crate serde_derive;

#[cfg(feature = "jsonrpc")]
#[macro_use]
extern crate serde_json;

pub mod account;
pub mod address;
mod builder;
pub mod chain;
pub mod constants;
mod keystore;
mod prover;
mod sender;
pub mod transaction;
pub mod types;
mod wallet;

#[cfg(test)]
mod tests;

pub use builder::Builder;
pub use wallet::Wallet;
