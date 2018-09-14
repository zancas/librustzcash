pub const UPGRADES_MAIN: [(u32, u32); 3] = [
    (0, 0),               // Sprout
    (0x5ba81b19, 347500), // Overwinter
    (0x76b809bb, 419200), // Sapling
];
pub const UPGRADES_TEST: [(u32, u32); 3] = [
    (0, 0),               // Sprout
    (0x5ba81b19, 207500), // Overwinter
    (0x76b809bb, 280000), // Sapling
];

pub const HRP_SAPLING_EXTENDED_SPENDING_KEY_MAIN: &str = "zs";
pub const HRP_SAPLING_EXTENDED_SPENDING_KEY_TEST: &str = "ztestsapling";
