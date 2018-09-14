use pairing::bls12_381::Bls12;
use sapling_crypto::primitives::PaymentAddress;
use zip32::ExtendedFullViewingKey;

#[derive(Debug, PartialEq)]
pub struct AccountId(pub u32);

/// A pool of ZEC controlled by a single spending key.
pub struct Account {
    pub label: String,
    pub(crate) xfvk: ExtendedFullViewingKey,
}

impl Account {
    pub(crate) fn new(label: String, xfvk: ExtendedFullViewingKey) -> Self {
        Account { label, xfvk }
    }

    pub fn default_address(&self) -> PaymentAddress<Bls12> {
        self.xfvk.default_address().unwrap().1
    }
}
