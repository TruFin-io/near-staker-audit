use crate::{Allocation, NearStaker, Pool, UnstakeRequest, Whitelist};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::store::LookupMap;
use near_sdk::{near, AccountId};
use std::collections::HashMap;

#[near(serializers=[borsh])]
pub enum VersionedNearStaker {
    V1(NearStakerV1),
    V2(NearStaker),
}

/// An old version of the contract state
#[near(serializers = [borsh])]
pub struct NearStakerV1 {
    pub whitelist: Whitelist,
    pub owner_id: AccountId,
    pub pending_owner: Option<AccountId>,
    pub treasury: AccountId,
    pub default_delegation_pool: AccountId,
    pub is_paused: bool,
    pub fee: u16,
    pub distribution_fee: u16,
    pub min_deposit: u128,
    delegation_pools: HashMap<AccountId, Pool>,
    pub delegation_pools_list: Vec<AccountId>,
    pub total_staked: u128,
    pub total_staked_last_updated_at: u64,
    allocations: LookupMap<AccountId, HashMap<AccountId, Allocation>>,
    unstake_requests: LookupMap<u128, UnstakeRequest>,
    pub unstake_nonce: u128,
    tax_exempt_stake: u128,
    withdrawn_amount: u128,
    token: FungibleToken,
    is_locked: bool,
}

/// Converts from an old version of the contract to the new one.
impl From<VersionedNearStaker> for NearStaker {
    fn from(contract: VersionedNearStaker) -> Self {
        match contract {
            VersionedNearStaker::V2(state) => state,
            VersionedNearStaker::V1(state) => NearStaker {
                whitelist: state.whitelist,
                owner_id: state.owner_id,
                pending_owner: state.pending_owner,
                treasury: state.treasury,
                default_delegation_pool: state.default_delegation_pool,
                is_paused: state.is_paused,
                fee: state.fee,
                min_deposit: state.min_deposit,
                delegation_pools: state.delegation_pools,
                delegation_pools_list: state.delegation_pools_list,
                total_staked: state.total_staked,
                total_staked_last_updated_at: state.total_staked_last_updated_at,
                unstake_requests: state.unstake_requests,
                unstake_nonce: state.unstake_nonce,
                tax_exempt_stake: state.tax_exempt_stake,
                withdrawn_amount: state.withdrawn_amount,
                token: state.token,
                is_locked: state.is_locked,
            },
        }
    }
}
