use near_sdk::{
    json_types::{U128, U64},
    near, AccountId,
};
use uint::construct_uint;

construct_uint! {
    #[near(serializers = [json, borsh])]
    pub struct U256(4);
}

/// Enums

#[near(serializers = [json, borsh])]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ValidatorState {
    NONE,
    ENABLED,
    DISABLED,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, PartialEq, Clone)]
pub enum UserStatus {
    #[allow(non_camel_case_types)]
    NO_STATUS,
    WHITELISTED,
    BLACKLISTED,
}

/// Structs

#[near(serializers = [json, borsh])]
#[derive(Clone)]
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

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct Pool {
    pub state: ValidatorState,
    pub total_staked: U128,
    // we keep track of the total amounts requested for unstake on each pool ourselves
    pub total_unstaked: U128,
    pub last_unstake: Option<u64>,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct PoolInfo {
    pub pool_id: AccountId,
    pub state: ValidatorState,
    pub total_staked: U128,
    pub unstake_available: bool,
    pub next_unstake_epoch: U64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone, Copy, Default)]
pub struct Allocation {
    pub near_amount: u128,
    pub share_price_num: U256,
    pub share_price_denom: U256,
}

#[near(serializers = [json, borsh])]
pub struct AllocationInfo {
    pub recipient: AccountId,
    pub near_amount: U128,
    pub share_price_num: String,
    pub share_price_denom: String,
}

pub(crate) struct DistributionInfo {
    pub near_amount: u128,
    pub shares_amount: u128,
    pub refund_amount: u128,
    pub fees: u128,
    pub share_price_num: U256,
    pub share_price_denom: U256,
}

#[near(serializers = [json, borsh])]
pub struct UnstakeRequest {
    pub user: AccountId,
    pub near_amount: u128,
    pub pool_address: AccountId,
    pub epoch: u64,
}
