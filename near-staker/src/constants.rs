use near_sdk::{Gas, NearToken};

pub const FEE_PRECISION: u16 = 10000;
pub const ONE_NEAR: u128 = 1_000_000_000_000_000_000_000_000; // 1 $NEAR as yoctoNEAR
pub const SHARE_PRICE_SCALING_FACTOR: u128 = 1_000_000_000_000_000_000_000_000;
pub const NO_DEPOSIT: NearToken = NearToken::from_near(0);
pub const NO_ARGS: Vec<u8> = vec![];
pub const XCC_GAS: Gas = Gas::from_tgas(30); // approx gas needed for cross-contract calls
pub const VIEW_GAS: Gas = Gas::from_tgas(5); // approx gas needed for view calls
pub const NUM_EPOCHS_TO_UNLOCK: u64 = 4; // number of epochs until unstaked amount can be withdrawn
pub const STORAGE_BYTES: u128 = 200; // approx bytes used to add unstake requests and allocations
