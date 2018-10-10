use failure::Error;
use pairing::bls12_381::{Bls12, Fr};
use rand::{OsRng, Rand};
use sapling_crypto::{
    jubjub::{fs::Fs, FixedGenerators, JubjubParams},
    primitives::{Diversifier, Note, PaymentAddress},
    redjubjub::Signature,
};
use zcash_primitives::{
    transaction::{
        components::{Amount, OutputDescription, SpendDescription},
        Transaction, TransactionData,
    },
    JUBJUB,
};
use zcash_proofs::sapling::{CommitmentTreeWitness, SaplingProvingContext};
use zip32::OutgoingViewingKey;

use super::Memo;
use account::AccountId;
use types::{KeyStore, TxProver};

const DEFAULT_FEE: Amount = Amount(10000);

struct SpendDescriptionInfo {
    account_id: AccountId,
    diversifier: Diversifier,
    note: Note<Bls12>,
    ar: Fs,
    witness: CommitmentTreeWitness,
}

struct OutputDescriptionInfo {
    ovk: OutgoingViewingKey,
    to: PaymentAddress<Bls12>,
    note: Note<Bls12>,
    memo: Memo,
}

/// Generates a Transaction from its inputs and outputs.
pub struct Builder {
    mtx: TransactionData,
    coin_type: u32,
    fee: Amount,
    anchor: Option<Fr>,
    spends: Vec<SpendDescriptionInfo>,
    outputs: Vec<OutputDescriptionInfo>,
    change_address: Option<(OutgoingViewingKey, PaymentAddress<Bls12>)>,
}

impl Builder {
    pub fn new(coin_type: u32) -> Builder {
        Builder {
            mtx: TransactionData::new(),
            coin_type,
            fee: DEFAULT_FEE,
            anchor: None,
            spends: vec![],
            outputs: vec![],
            change_address: None,
        }
    }

    pub fn set_fee(&mut self, fee: Amount) {
        self.fee = fee;
    }

    pub fn add_sapling_spend(
        &mut self,
        account_id: AccountId,
        diversifier: Diversifier,
        note: Note<Bls12>,
        ar: Fs,
        witness: CommitmentTreeWitness,
    ) -> Result<(), Error> {
        // Consistency check: all anchors must equal the first one
        if let Some(anchor) = self.anchor {
            if witness.root() != anchor {
                return Err(format_err!(
                    "Anchor mismatch (expected {}, got {})",
                    anchor,
                    witness.root()
                ));
            }
        } else {
            self.anchor = Some(witness.root())
        }

        self.mtx.value_balance.0 += note.value as i64;

        self.spends.push(SpendDescriptionInfo {
            account_id,
            diversifier,
            note,
            ar,
            witness,
        });

        Ok(())
    }

    pub fn add_sapling_output(
        &mut self,
        ovk: OutgoingViewingKey,
        to: PaymentAddress<Bls12>,
        value: Amount,
        rcm: Fs,
        memo: Option<Memo>,
    ) -> Result<(), Error> {
        let g_d = match to.g_d(&JUBJUB) {
            Some(g_d) => g_d,
            None => return Err(format_err!("Invalid target address")),
        };

        self.mtx.value_balance.0 -= value.0;

        let note = Note {
            g_d,
            pk_d: to.pk_d.clone(),
            value: value.0 as u64,
            r: rcm,
        };
        self.outputs.push(OutputDescriptionInfo {
            ovk,
            to,
            note,
            memo: memo.unwrap_or_default(),
        });

        Ok(())
    }

    pub fn build(
        mut self,
        consensus_branch_id: u32,
        ks: &Box<KeyStore>,
        prover: &Box<TxProver>,
    ) -> Result<Transaction, Error> {
        //
        // Consistency checks
        //

        // TODO: Remove placeholder usages
        let mut rng = OsRng::new().expect("should be able to construct RNG");

        // Valid change
        let change = self.mtx.value_balance.0 - self.fee.0;
        if change.is_negative() {
            return Err(format_err!("Change is negative: {}", change));
        }

        //
        // Change output
        //

        if change.is_positive() {
            // Send change to the specified change address. If no change address
            // was set, send change to the first Sapling address given as input.
            let change_address = if let Some(change_address) = self.change_address.take() {
                change_address
            } else if !self.spends.is_empty() {
                let ovk = ks.xfvk(self.coin_type, self.spends[0].account_id.0).fvk.ovk;
                (
                    ovk,
                    PaymentAddress {
                        diversifier: self.spends[0].diversifier,
                        pk_d: self.spends[0].note.pk_d.clone(),
                    },
                )
            } else {
                return Err(format_err!("No change address"));
            };

            self.add_sapling_output(
                change_address.0,
                change_address.1,
                Amount(change),
                Fs::rand(&mut rng), // TODO
                None,
            )?;
        }

        //
        // Sapling spends and outputs
        //

        let mut ctx = SaplingProvingContext::new();
        let anchor = self.anchor.expect("anchor was set if spends were added");

        // Create Sapling SpendDescriptions
        for spend in &self.spends {
            let proof_generation_key = ks.proving_key(self.coin_type, spend.account_id.0);

            let mut nullifier = [0u8; 32];
            nullifier.copy_from_slice(&spend.note.nf(
                &proof_generation_key.into_viewing_key(&JUBJUB),
                spend.witness.position,
                &JUBJUB,
            ));

            let (zkproof, cv, rk) = prover.spend_proof(
                &mut ctx,
                proof_generation_key,
                spend.diversifier,
                spend.note.r,
                spend.ar,
                spend.note.value,
                anchor,
                spend.witness.clone(),
            )?;

            self.mtx.shielded_spends.push(SpendDescription {
                cv,
                anchor: anchor,
                nullifier,
                rk,
                zkproof,
                spend_auth_sig: Signature::blank(),
            });
        }

        // Create Sapling OutputDescriptions
        for output in self.outputs {
            // let note_plaintext = (output.note, output.memo);

            // let enc = note_plaintext.encrypt(output.note.pk_d)?;
            // let encryptor = enc.second;
            let esk = Fs::rand(&mut rng); //encryptor.get_esk();

            let (zkproof, cv) =
                prover.output_proof(&mut ctx, esk, output.to, output.note.r, output.note.value);

            let cmu = output.note.cm(&JUBJUB);
            let ephemeral_key = JUBJUB
                .generator(FixedGenerators::SpendingKeyGenerator)
                .mul(esk, &JUBJUB)
                .into(); //encryptor.get_epk();
            let enc_ciphertext = [0u8; 580]; //enc.first;

            // let out_plaintext = (output.note.pk_d, esk);
            let out_ciphertext = [0u8; 80]; //out_plaintext.encrypt(output.ovk, cv, cmu, encryptor);

            self.mtx.shielded_outputs.push(OutputDescription {
                cv,
                cmu,
                ephemeral_key,
                enc_ciphertext,
                out_ciphertext,
                zkproof,
            });
        }

        //
        // Signatures
        //

        // Sign every input using the provided KeyStore
        let inputs_to_sign: Vec<Option<(AccountId, Fs)>> = self
            .spends
            .into_iter()
            .map(|s| Some((s.account_id, s.ar)))
            .collect();

        ks.sign(
            &mut self.mtx,
            &inputs_to_sign,
            consensus_branch_id,
            self.coin_type,
            ctx,
        )?;

        Ok(self.mtx.freeze())
    }
}

#[cfg(test)]
mod tests {
    use super::Builder;

    #[test]
    fn fails_on_negative_change() {
    }
}
