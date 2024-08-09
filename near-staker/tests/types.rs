use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    json_types::{U128, U64},
    AccountId, NearToken,
};

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum ValidatorState {
    NONE,
    ENABLED,
    DISABLED,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct PoolInfo {
    pub pool_id: AccountId,
    pub state: ValidatorState,
    pub total_staked: U128,
    pub unstake_available: bool,
    pub next_unstake_epoch: U64,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct StorageBalance {
    pub total: NearToken,
    pub available: NearToken,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct StorageBalanceBounds {
    pub min: NearToken,
    pub max: Option<NearToken>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct AllocationInfo {
    pub recipient: AccountId,
    pub near_amount: U128,
    pub share_price_num: String,
    pub share_price_denom: String,
}

#[derive(Deserialize, Serialize, PartialEq, Debug)]
pub struct StakerInfo {
    pub owner_id: AccountId,
    pub treasury_id: AccountId,
    pub default_delegation_pool: AccountId,
    pub fee: u16,
    pub dist_fee: u16,
    pub min_deposit: U128,
    pub is_paused: bool,
    pub current_epoch: U64,
}
