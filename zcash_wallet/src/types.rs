//! The traits for the various wallet components.

use failure::Error;
use pairing::bls12_381::{Bls12, Fr};
use sapling_crypto::{
    jubjub::{edwards, fs::Fs, Unknown},
    primitives::{Diversifier, PaymentAddress, ProofGenerationKey},
    redjubjub::PublicKey,
};
use zcash_primitives::transaction::{components::GROTH_PROOF_SIZE, Transaction, TransactionData};
use zcash_proofs::sapling::{CommitmentTreeWitness, SaplingProvingContext};
use zip32::ExtendedFullViewingKey;

use account::AccountId;

/// Holds the wallet's keys, and authorises the spending of wallet funds.
pub trait KeyStore {
    /// Returns the ProofGenerationKey for the given coin_type and account
    /// index, using the ZIP 32 path `m/32'/coin_type'/account'`.
    fn proving_key(&self, coin_type: u32, account: u32) -> ProofGenerationKey<Bls12>;

    /// Returns the ExtendedFullViewingKey for the given coin_type and account
    /// index, using the ZIP 32 path `m/32'/coin_type'/account'`.
    fn xfvk(&self, coin_type: u32, account: u32) -> ExtendedFullViewingKey;

    /// Sign the given transaction.
    /// TODO: Errors
    fn sign(
        &self,
        mtx: &mut TransactionData,
        inputs_to_sign: &[Option<(AccountId, Fs)>],
        consensus_branch_id: u32,
        coin_type: u32,
        ctx: SaplingProvingContext,
    ) -> Result<(), Error>;
}

/// Handles block chain state and synchronisation.
pub trait ChainState {
    /// Returns the consensus branch ID for the current network epoch.
    fn consensus_branch_id(&self) -> u32;
}

/// Interface for creating zero-knowledge proofs for shielded transactions.
pub trait TxProver {
    /// Create the value commitment, re-randomized key, and proof for a Sapling
    /// SpendDescription, while accumulating its value commitment randomness
    /// inside the context for later use.
    fn spend_proof(
        &self,
        ctx: &mut SaplingProvingContext,
        proof_generation_key: ProofGenerationKey<Bls12>,
        diversifier: Diversifier,
        rcm: Fs,
        ar: Fs,
        value: u64,
        anchor: Fr,
        witness: CommitmentTreeWitness,
    ) -> Result<
        (
            [u8; GROTH_PROOF_SIZE],
            edwards::Point<Bls12, Unknown>,
            PublicKey<Bls12>,
        ),
        Error,
    >;

    /// Create the value commitment and proof for a Sapling OutputDescription,
    /// while accumulating its value commitment randomness inside the context
    /// for later use.
    fn output_proof(
        &self,
        ctx: &mut SaplingProvingContext,
        esk: Fs,
        payment_address: PaymentAddress<Bls12>,
        rcm: Fs,
        value: u64,
    ) -> ([u8; GROTH_PROOF_SIZE], edwards::Point<Bls12, Unknown>);
}

/// The result of trying to send a transaction.
#[derive(Debug, PartialEq)]
pub enum SendResult {
    /// The transaction has definitely reached the network, and is currently in
    /// the mempool.
    InMemPool,

    /// The transaction has been accepted by the sending backend, but is not
    /// guaranteed to reach the mempool. Retransmissions are the responsibility
    /// of the sender.
    BestEffort,
}

/// Sends transactions to the Zcash network for mining.
pub trait TxSender {
    /// Send the given transaction.
    fn send(&self, tx: &Transaction) -> Result<SendResult, Error>;
}
