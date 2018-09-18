use pairing::bls12_381::Bls12;
use rand::{Rand, Rng};
use sapling_crypto::{
    jubjub::fs::Fs,
    primitives::{Diversifier, Note},
};
use zcash_primitives::{
    merkle_tree::{CommitmentTree, IncrementalWitness},
    JUBJUB,
};

pub(crate) fn fake_note(ivk: Fs, d: &Diversifier, value: u64, mut rng: &mut Rng) -> Note<Bls12> {
    let g_d = d.g_d::<Bls12>(&JUBJUB).unwrap();
    let pk_d = g_d.mul(ivk, &JUBJUB);

    Note {
        value,
        g_d,
        pk_d,
        r: Fs::rand(&mut rng),
    }
}

pub(crate) fn fake_witness() -> IncrementalWitness {
    IncrementalWitness::from_tree(&CommitmentTree::new())
}
