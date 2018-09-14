use failure::Error;
use pairing::bls12_381::Bls12;
use sapling_crypto::{jubjub::fs::Fs, primitives::ProofGenerationKey, redjubjub::PrivateKey};
use zcash_primitives::{
    sapling::spend_sig,
    transaction::{signature_hash_data, TransactionData, SIGHASH_ALL},
    JUBJUB,
};
use zcash_proofs::sapling::SaplingProvingContext;
use zip32::{ChildIndex, ExtendedFullViewingKey, ExtendedSpendingKey};

use account::AccountId;
use types::KeyStore;

pub struct LocalKeyStore {
    master: ExtendedSpendingKey,
}

impl LocalKeyStore {
    pub fn from_seed(seed: &[u8]) -> Self {
        LocalKeyStore {
            master: ExtendedSpendingKey::master(&seed),
        }
    }
}

impl KeyStore for LocalKeyStore {
    fn proving_key(&self, coin_type: u32, account: u32) -> ProofGenerationKey<Bls12> {
        let xsk = ExtendedSpendingKey::from_path(
            &self.master,
            &[
                ChildIndex::Hardened(32),
                ChildIndex::Hardened(coin_type),
                ChildIndex::Hardened(account),
            ],
        );
        xsk.expsk.proof_generation_key(&JUBJUB)
    }

    fn xfvk(&self, coin_type: u32, account: u32) -> ExtendedFullViewingKey {
        let xsk = ExtendedSpendingKey::from_path(
            &self.master,
            &[
                ChildIndex::Hardened(32),
                ChildIndex::Hardened(coin_type),
                ChildIndex::Hardened(account),
            ],
        );
        (&xsk).into()
    }

    fn sign(
        &self,
        mtx: &mut TransactionData,
        inputs_to_sign: &[Option<(AccountId, Fs)>],
        consensus_branch_id: u32,
        coin_type: u32,
        ctx: SaplingProvingContext,
    ) -> Result<(), Error> {
        if mtx.shielded_spends.len() < inputs_to_sign.len() {
            return Err(format_err!(
                "Transaction has {} shielded inputs, {} provided to sign",
                mtx.shielded_spends.len(),
                inputs_to_sign.len()
            ));
        }

        let mut sighash = [0u8; 32];
        sighash.copy_from_slice(&signature_hash_data(
            &mtx,
            consensus_branch_id,
            SIGHASH_ALL,
            None,
        ));

        // Create Sapling spendAuth and binding signatures
        for (i, input) in inputs_to_sign.iter().enumerate() {
            if let Some((account_id, ar)) = input {
                let xsk = ExtendedSpendingKey::from_path(
                    &self.master,
                    &[
                        ChildIndex::Hardened(32),
                        ChildIndex::Hardened(coin_type),
                        ChildIndex::Hardened(account_id.0),
                    ],
                );

                mtx.shielded_spends[i].spend_auth_sig =
                    spend_sig(PrivateKey(xsk.expsk.ask), *ar, &sighash, &JUBJUB);
            }
        }
        mtx.binding_sig = Some(
            ctx.binding_sig(mtx.value_balance.0, &sighash, &JUBJUB)
                .map_err(|_| format_err!("Failed to create bindingSig"))?,
        );

        Ok(())
    }
}
