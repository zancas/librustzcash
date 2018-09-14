use failure::Error;
use pairing::bls12_381::{Bls12, Fr};
use rand::{OsRng, Rand};
use sapling_crypto::{
    jubjub::{edwards, fs::Fs, FixedGenerators, Unknown},
    primitives::{Diversifier, PaymentAddress, ProofGenerationKey, ValueCommitment},
    redjubjub::PublicKey,
};
use zcash_primitives::{transaction::components::GROTH_PROOF_SIZE, JUBJUB};
use zcash_proofs::sapling::{CommitmentTreeWitness, SaplingProvingContext};

use types::TxProver;

pub(crate) struct MockTxProver;

impl TxProver for MockTxProver {
    fn spend_proof(
        &self,
        _ctx: &mut SaplingProvingContext,
        proof_generation_key: ProofGenerationKey<Bls12>,
        _diversifier: Diversifier,
        _rcm: Fs,
        ar: Fs,
        value: u64,
        _anchor: Fr,
        _witness: CommitmentTreeWitness,
    ) -> Result<
        (
            [u8; GROTH_PROOF_SIZE],
            edwards::Point<Bls12, Unknown>,
            PublicKey<Bls12>,
        ),
        Error,
    > {
        let mut rng = OsRng::new().expect("should be able to construct RNG");

        let cv = ValueCommitment::<Bls12> {
            value,
            randomness: Fs::rand(&mut rng),
        }.cm(&JUBJUB)
        .into();

        let rk = PublicKey::<Bls12>(proof_generation_key.ak.clone().into()).randomize(
            ar,
            FixedGenerators::SpendingKeyGenerator,
            &JUBJUB,
        );

        Ok(([0u8; GROTH_PROOF_SIZE], cv, rk))
    }

    fn output_proof(
        &self,
        _ctx: &mut SaplingProvingContext,
        _esk: Fs,
        _payment_address: PaymentAddress<Bls12>,
        _rcm: Fs,
        value: u64,
    ) -> ([u8; GROTH_PROOF_SIZE], edwards::Point<Bls12, Unknown>) {
        let mut rng = OsRng::new().expect("should be able to construct RNG");

        let cv = ValueCommitment::<Bls12> {
            value,
            randomness: Fs::rand(&mut rng),
        }.cm(&JUBJUB)
        .into();

        ([0u8; GROTH_PROOF_SIZE], cv)
    }
}
