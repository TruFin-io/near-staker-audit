use near_contract_standards::fungible_token::{FungibleToken, FungibleTokenCore};
use near_sdk::store::{LookupMap, LookupSet};
use near_sdk::{
    env,
    json_types::Base64VecU8,
    json_types::{U128, U64},
    log, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise, PromiseError,
    PromiseResult,
};

use std::collections::HashMap;
mod constants;
pub mod errors;
mod events;
mod external;
mod internal;
mod math;
mod trunear;
mod types;
mod upgrade;
pub mod whitelist;

use crate::constants::*;
use crate::errors::*;
use crate::events::Event;
use crate::types::*;
use crate::upgrade::VersionedNearStaker;

// Define the contract structure
#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct NearStaker {
    /// The whitelist.
    pub whitelist: Whitelist,
    /// The account ID of the owner.
    pub owner_id: AccountId,
    /// The account ID of the pending owner.
    pub pending_owner: Option<AccountId>,
    /// The account ID of the treasury.
    pub treasury: AccountId,
    /// The default delegation pool contract ID.
    pub default_delegation_pool: AccountId,
    /// Boolean flag indicating whether the contract is currently paused for user operations.
    pub is_paused: bool,
    /// Both the fee and distribution fee are percentages with FEE_PRECISION digits of precision i.e. 1000 = 10%.
    /// The treasury fee charged on staking rewards.
    pub fee: u16,
    /// The treasury distribution fee charged upon distributing allocation rewards.
    pub distribution_fee: u16,
    /// The minimum NEAR amount a user can deposit.
    pub min_deposit: u128,
    /// The delegation pools.
    delegation_pools: HashMap<AccountId, Pool>,
    /// List of the delegation pools.
    pub delegation_pools_list: Vec<AccountId>,
    /// The total staked across all delegation pools.
    pub total_staked: u128,
    /// Epoch when total_staked was last updated.
    pub total_staked_last_updated_at: u64,
    /// Allocations.
    allocations: LookupMap<AccountId, HashMap<AccountId, Allocation>>,
    /// Unstake requests.
    unstake_requests: LookupMap<u128, UnstakeRequest>,
    /// The most recent unstake nonce.
    pub unstake_nonce: u128,
    /// Total amount of NEAR staked in the staker for which no fees are charged/have already been charged.
    tax_exempt_stake: u128,
    /// Total amount of NEAR withdrawn into the staker.
    withdrawn_amount: u128,
    /// TruNEAR token.
    token: FungibleToken,
    /// Reentrancy flag when contract is in the middle of a cross-contract call.
    is_locked: bool,
}

#[near(serializers = [borsh])]
pub struct Whitelist {
    agents: LookupSet<AccountId>,
    users: LookupMap<AccountId, UserStatus>,
}

// Implement the contract structure.
#[near]
impl NearStaker {
    /// Initialises the contract with the given owner ID, treasury ID, and default delegation pool ID.
    #[init]
    pub fn new(
        owner_id: AccountId,
        treasury: AccountId,
        default_delegation_pool: AccountId,
    ) -> Self {
        let mut delegation_pools = HashMap::new();
        let default_pool = Pool {
            state: ValidatorState::ENABLED,
            total_staked: U128(0),
            total_unstaked: U128(0),
            last_unstake: None,
        };
        delegation_pools.insert(default_delegation_pool.clone(), default_pool);

        let mut token = FungibleToken::new(b"t".to_vec());
        token.accounts.insert(&treasury, &0);

        Event::StakerInitialisedEvent {
            owner: &owner_id,
            treasury: &treasury,
            default_delegation_pool: &default_delegation_pool,
            fee: &0,
            distribution_fee: &0,
            min_deposit: &U128::from(ONE_NEAR),
        }
        .emit();

        Self {
            whitelist: Whitelist {
                agents: LookupSet::new(b"o".to_vec()),
                users: LookupMap::new(b"w".to_vec()),
            },
            owner_id,
            pending_owner: None,
            treasury,
            default_delegation_pool: default_delegation_pool.clone(),
            is_paused: false,
            fee: 0,
            distribution_fee: 0,
            min_deposit: ONE_NEAR,
            delegation_pools,
            delegation_pools_list: vec![default_delegation_pool],
            allocations: LookupMap::new(b"a".to_vec()),
            unstake_requests: LookupMap::new(b"u".to_vec()),
            unstake_nonce: 0,
            total_staked: 0,
            total_staked_last_updated_at: env::epoch_height(),
            token,
            tax_exempt_stake: 0,
            withdrawn_amount: 0,
            is_locked: false,
        }
    }

    /// View Methods

    /// Checks if the provided address is the contract owner.
    pub fn is_owner(&self, account_id: AccountId) -> bool {
        self.owner_id == account_id
    }

    /// Checks whether the unstake request is ready for withdrawal.
    pub fn is_claimable(&self, unstake_nonce: U128) -> bool {
        let request = self
            .unstake_requests
            .get(&unstake_nonce.0)
            .expect(ERR_INVALID_NONCE);
        request.epoch + NUM_EPOCHS_TO_UNLOCK <= env::epoch_height()
    }

    /// Returns the total staked across all pools.
    pub fn get_total_staked(&self) -> (U128, U64) {
        (
            self.total_staked.into(),
            self.total_staked_last_updated_at.into(),
        )
    }

    /// Returns the tax exempt stake.
    pub fn get_tax_exempt_stake(&self) -> U128 {
        self.tax_exempt_stake.into()
    }

    /// Returns all available pools and their info.
    pub fn get_pools(&self) -> Vec<PoolInfo> {
        self.delegation_pools
            .iter()
            .map(|(pool_id, pool)| {
                let last_unstake_in_same_epoch = pool.last_unstake.is_none()
                    || pool.last_unstake.unwrap() == env::epoch_height();
                let no_pending_unstakes = pool.last_unstake.is_none()
                    || pool.last_unstake.unwrap() + NUM_EPOCHS_TO_UNLOCK <= env::epoch_height();

                let next_unstake_epoch = if pool.last_unstake.is_none() {
                    env::epoch_height()
                } else {
                    pool.last_unstake.unwrap() + NUM_EPOCHS_TO_UNLOCK
                };

                PoolInfo {
                    pool_id: pool_id.clone(),
                    state: pool.state,
                    total_staked: pool.total_staked,
                    unstake_available: last_unstake_in_same_epoch || no_pending_unstakes,
                    next_unstake_epoch: next_unstake_epoch.into(),
                }
            })
            .collect()
    }

    /// Returns the latest unstake nonce.
    pub fn get_latest_unstake_nonce(&self) -> U128 {
        self.unstake_nonce.into()
    }

    /// Returns the allocation and unstake storage cost
    pub fn get_storage_cost() -> U128 {
        env::storage_byte_cost()
            .saturating_mul(STORAGE_BYTES)
            .as_yoctonear()
            .into()
    }

    /// Returns all allocations for a given user.
    pub fn get_allocations(&self, allocator: AccountId) -> Vec<AllocationInfo> {
        self.allocations
            .get(&allocator)
            .expect(ERR_NO_ALLOCATIONS)
            .iter()
            .map(|(recipient, allocation)| AllocationInfo {
                recipient: recipient.clone(),
                near_amount: allocation.near_amount.into(),
                share_price_num: allocation.share_price_num.to_string(),
                share_price_denom: allocation.share_price_denom.to_string(),
            })
            .collect()
    }

    /// Returns the total amount of NEAR allocated by a user and their average allocation share price.
    pub fn get_total_allocated(&self, allocator: AccountId) -> (U128, String, String) {
        let total_allocation = match self.allocations.get(&allocator) {
            Some(user_allocations) => {
                user_allocations
                    .iter()
                    .map(|(_, allocation)| allocation)
                    .fold(Allocation::default(), |acc, allocation| {
                        // this can only be true for the first iteration
                        if acc.near_amount == 0 {
                            *allocation
                        } else {
                            Self::calculate_updated_allocation(
                                &acc,
                                allocation.near_amount,
                                allocation.share_price_num,
                                allocation.share_price_denom,
                            )
                        }
                    })
            }

            None => Allocation::default(),
        };

        (
            total_allocation.near_amount.into(),
            total_allocation.share_price_num.to_string(),
            total_allocation.share_price_denom.to_string(),
        )
    }

    /// Returns the amounts of TruNEAR and NEAR required to distribute to a single recipient
    /// or to all recipients when no recipient account is provided.
    pub fn get_rewards_distribution_amounts(
        &self,
        distributor: &AccountId,
        recipient: Option<AccountId>,
        in_near: bool,
    ) -> (U128, U128) {
        let user_allocations = self.allocations.get(distributor);

        // if the distributor has no allocations no TruNEAR or NEAR is needed
        if user_allocations.is_none() {
            return (U128(0), U128(0));
        };

        let (global_price_num, global_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );

        let required_shares;
        if let Some(r) = recipient {
            // calculate the amount of shares required to distribute to a single recipient
            let allocation = user_allocations
                .unwrap()
                .get(&r)
                .expect(ERR_NO_ALLOCATIONS_TO_RECIPIENT);

            required_shares = Self::internal_calculate_distribution_amount(
                allocation,
                global_price_num,
                global_price_denom,
            );
        } else {
            // calculate the amount of shares required to distribute to all recipients
            required_shares = user_allocations
                .unwrap()
                .iter()
                .map(|(_, allocation)| allocation)
                .fold(0, |acc, a| {
                    acc + Self::internal_calculate_distribution_amount(
                        a,
                        global_price_num,
                        global_price_denom,
                    )
                });
        }

        if in_near {
            // for NEAR distributions fees are deducted from the required NEAR amount and accounted as required TruNEAR
            let fees = required_shares * (self.distribution_fee as u128) / (FEE_PRECISION as u128);
            let required_near = Self::convert_to_assets(
                required_shares - fees,
                global_price_num,
                global_price_denom,
                false,
            );
            (U128::from(fees), U128::from(required_near))
        } else {
            // for TruNEAR distributions the required TruNEAR amount includes the distribution fees
            (U128::from(required_shares), U128(0))
        }
    }

    /// Returns some of the Staker internal state
    pub fn get_staker_info(&self) -> StakerInfo {
        StakerInfo {
            owner_id: self.owner_id.clone(),
            treasury_id: self.treasury.clone(),
            default_delegation_pool: self.default_delegation_pool.clone(),
            fee: self.fee,
            dist_fee: self.distribution_fee,
            min_deposit: U128::from(self.min_deposit),
            is_paused: self.is_paused,
            current_epoch: env::epoch_height().into(),
        }
    }

    /// Returns the current TruNEAR share price in NEAR.
    pub fn share_price(&self) -> (String, String) {
        let (num, denom) = Self::internal_share_price(
            self.total_staked,
            self.token.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );

        (num.to_string(), denom.to_string())
    }

    /// Returns the maximum amount of NEAR a user can withdraw from the vault, rounding the result up.
    pub fn max_withdraw(&self, account_id: AccountId) -> U128 {
        let (share_price_num, share_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );
        let shares_balance = self.ft_balance_of(account_id).0;
        let assets =
            Self::convert_to_assets(shares_balance, share_price_num, share_price_denom, true);

        U128(assets)
    }

    /// Returns whether the contract is locked.
    pub fn get_is_locked(&self) -> bool {
        self.is_locked
    }

    /// Owner Functionality

    /// Upgrade the contract and migrate the contract state.
    pub fn upgrade(&self, code: Base64VecU8, migrate: bool) -> Promise {
        self.check_owner();
        if migrate {
            Promise::new(env::current_account_id())
                .deploy_contract(code.0)
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(100))
                        .migrate(),
                )
        } else {
            Promise::new(env::current_account_id()).deploy_contract(code.0)
        }
    }

    /// Pauses the contract to prevent user operations.
    pub fn pause(&mut self) {
        self.check_owner();
        self.check_not_paused();
        self.is_paused = true;
        Event::PausedEvent {}.emit();
    }

    /// Unpauses the contract to allow user operations.
    pub fn unpause(&mut self) {
        self.check_owner();
        require!(self.is_paused, ERR_NOT_PAUSED);
        self.is_paused = false;
        Event::UnpausedEvent {}.emit();
    }

    /// Unlocks the contract if it remains locked due to some unforseen circumstances.
    pub fn manual_unlock(&mut self) {
        self.check_owner();
        self.is_locked = false;
    }

    /// Sets the account ID of the treasury.
    pub fn set_treasury(&mut self, new_treasury: AccountId) {
        self.check_owner();
        Event::SetTreasuryEvent {
            old_treasury: &self.treasury,
            new_treasury: &new_treasury,
        }
        .emit();
        self.treasury = new_treasury;
    }

    /// Sets the treasury fee charged on rewards.
    pub fn set_fee(&mut self, new_fee: u16) {
        self.check_owner();
        require!(new_fee < FEE_PRECISION, ERR_FEE_TOO_LARGE);
        Event::SetFeeEvent {
            old_fee: &self.fee,
            new_fee: &new_fee,
        }
        .emit();
        self.fee = new_fee;
    }

    /// Sets the treasury fee charged on rewards distribution.
    pub fn set_distribution_fee(&mut self, new_distribution_fee: u16) {
        self.check_owner();
        require!(new_distribution_fee < FEE_PRECISION, ERR_FEE_TOO_LARGE);
        Event::SetDistributionFeeEvent {
            old_distribution_fee: &self.distribution_fee,
            new_distribution_fee: &new_distribution_fee,
        }
        .emit();
        self.distribution_fee = new_distribution_fee;
    }

    /// Sets a given pool as the new default delegation pool.
    pub fn set_default_delegation_pool(&mut self, pool_id: AccountId) {
        self.check_owner();

        self.check_pool(pool_id.clone());

        Event::SetDefaultDelegationPoolEvent {
            old_default_delegation_pool: &self.default_delegation_pool,
            new_default_delegation_pool: &pool_id,
        }
        .emit();
        self.default_delegation_pool = pool_id;
    }

    /// Sets the minimum NEAR amount a user can deposit.
    pub fn set_min_deposit(&mut self, min_deposit: U128) {
        require!(min_deposit.0 >= ONE_NEAR, ERR_MIN_DEPOSIT_TOO_SMALL);
        self.check_owner();
        Event::SetMinDepositEvent {
            old_min_deposit: &U128::from(self.min_deposit),
            new_min_deposit: &min_deposit,
        }
        .emit();
        self.min_deposit = min_deposit.0;
    }

    /// Sets a pending owner. The pending owner has no contract privileges.
    pub fn set_pending_owner(&mut self, new_owner_id: AccountId) {
        self.check_owner();
        self.pending_owner = Some(new_owner_id.clone());
        Event::SetPendingOwnerEvent {
            current_owner: &self.owner_id,
            pending_owner: &new_owner_id,
        }
        .emit();
    }

    /// Allows the pending owner to claim ownership of the contract.
    pub fn claim_ownership(&mut self) {
        let new_owner_id = self.pending_owner.take().expect(ERR_NO_PENDING_OWNER);
        require!(
            env::predecessor_account_id() == new_owner_id,
            ERR_NOT_PENDING_OWNER
        );
        Event::OwnershipClaimedEvent {
            old_owner: &self.owner_id,
            new_owner: &new_owner_id,
        }
        .emit();
        self.owner_id = new_owner_id;
    }

    /// Adds a new pool.
    pub fn add_pool(&mut self, pool_id: AccountId) {
        self.check_owner();
        require!(
            !self.delegation_pools.contains_key(&pool_id),
            ERR_POOL_ALREADY_EXISTS
        );

        let pool = Pool {
            state: ValidatorState::ENABLED,
            total_staked: U128(0),
            total_unstaked: U128(0),
            last_unstake: None,
        };

        self.delegation_pools.insert(pool_id.clone(), pool);
        self.delegation_pools_list.push(pool_id.clone());

        // emit event
        Event::DelegationPoolAddedEvent { pool_id: &pool_id }.emit();
    }

    /// Enables a disabled pool.
    pub fn enable_pool(&mut self, pool_id: AccountId) {
        self.check_owner();

        let pool = self
            .delegation_pools
            .get_mut(&pool_id)
            .expect(ERR_POOL_DOES_NOT_EXIST);
        require!(
            pool.state != ValidatorState::ENABLED,
            ERR_POOL_ALREADY_ENABLED
        );

        // enable delegation pool
        pool.state = ValidatorState::ENABLED;

        // emit event
        Event::DelegationPoolStateChangedEvent {
            pool_id: &pool_id,
            old_state: ValidatorState::DISABLED,
            new_state: ValidatorState::ENABLED,
        }
        .emit();
    }

    /// Disables an enabled pool. Disabled pools cannot be staked to, but stake already on the validator can be
    /// unstaked and withdrawn as normal.
    pub fn disable_pool(&mut self, pool_id: AccountId) {
        self.check_owner();

        let pool = self
            .delegation_pools
            .get_mut(&pool_id)
            .expect(ERR_POOL_DOES_NOT_EXIST);
        require!(
            pool.state != ValidatorState::DISABLED,
            ERR_POOL_ALREADY_DISABLED
        );

        // disable delegation pool
        pool.state = ValidatorState::DISABLED;

        // emit event
        Event::DelegationPoolStateChangedEvent {
            pool_id: &pool_id,
            old_state: ValidatorState::ENABLED,
            new_state: ValidatorState::DISABLED,
        }
        .emit();
    }

    /// Updates the total stake to yield the most up-to-date share price.
    pub fn update_total_staked(&mut self) -> Promise {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;
        self.internal_update_stake().then(
            Self::ext(env::current_account_id())
                .with_static_gas(XCC_GAS)
                .total_staked_callback(),
        )
    }

    /// Collects staker fees on behalf of the treasury.
    pub fn collect_fees(&mut self) {
        self.check_not_paused();
        self.check_contract_in_sync();

        self.internal_collect_fees();
    }

    /// User Functionality

    #[payable]
    /// Stakes NEAR to default pool.
    pub fn stake(&mut self) -> Promise {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;

        self.check_whitelisted();

        self.internal_deposit_and_stake(
            self.default_delegation_pool.clone(),
            env::attached_deposit().as_yoctonear(),
            env::predecessor_account_id(),
        )
    }

    #[payable]
    /// Stakes NEAR to a specific pool.
    pub fn stake_to_specific_pool(&mut self, pool_id: AccountId) -> Promise {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;

        self.check_whitelisted();

        self.internal_deposit_and_stake(
            pool_id,
            env::attached_deposit().as_yoctonear(),
            env::predecessor_account_id(),
        )
    }

    /// Unstakes NEAR from default pool.
    #[payable]
    pub fn unstake(&mut self, amount: U128) -> Promise {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;

        self.check_whitelisted();

        self.internal_unstake(
            self.default_delegation_pool.clone(),
            amount.0,
            env::predecessor_account_id(),
        )
    }

    /// Unstakes NEAR from specific pool.
    #[payable]
    pub fn unstake_from_specific_pool(&mut self, pool_id: AccountId, amount: U128) -> Promise {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;

        self.check_whitelisted();

        require!(
            self.delegation_pools.contains_key(&pool_id),
            ERR_POOL_DOES_NOT_EXIST
        );

        self.internal_unstake(pool_id, amount.0, env::predecessor_account_id())
    }

    /// Allocates NEAR staking rewards to a recipient. Requires a storage deposit for new allocations
    /// that is refunded upon deallocation.
    #[payable]
    pub fn allocate(&mut self, recipient: AccountId, amount: U128) {
        self.check_not_paused();
        self.check_whitelisted();
        let allocator = env::predecessor_account_id();
        let amount = amount.0;

        require!(recipient != allocator, ERR_INVALID_RECIPIENT);
        require!(amount >= ONE_NEAR, ERR_ALLOCATION_UNDER_ONE_NEAR);

        let (global_share_price_num, global_share_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );

        let mut storage_cost = NearToken::from_near(0);
        let attached_deposit = env::attached_deposit();

        let allocation = self
            .allocations
            .entry(allocator.clone())
            .or_default() // fetches the users allocations or creates a new hashmap
            .entry(recipient.clone()) //fetches the allocation to the recipient
            .and_modify(|allocation| {
                //updates the recipients allocation if it exists
                *allocation = Self::calculate_updated_allocation(
                    allocation,
                    amount,
                    global_share_price_num,
                    global_share_price_denom,
                )
            })
            .or_insert_with(|| {
                storage_cost = NearToken::from_yoctonear(Self::get_storage_cost().0);
                if attached_deposit < storage_cost {
                    env::panic_str(ERR_STORAGE_DEPOSIT_TOO_SMALL);
                }
                Allocation {
                    // inserts a new allocation if one doesn't exist for this recipient
                    near_amount: amount,
                    share_price_num: global_share_price_num,
                    share_price_denom: global_share_price_denom,
                }
            });

        let updated_allocation = *allocation;
        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = self.get_total_allocated(allocator.clone());

        // refund any excess NEAR to allocator
        if attached_deposit > storage_cost {
            Promise::new(allocator.clone())
                .transfer(attached_deposit.checked_sub(storage_cost).unwrap());
        }

        // emit event
        Event::AllocatedEvent {
            user: &allocator,
            recipient: &recipient,
            amount: &amount.into(),
            total_amount: &updated_allocation.near_amount.into(),
            share_price_num: &updated_allocation.share_price_num.to_string(),
            share_price_denom: &updated_allocation.share_price_denom.to_string(),
            total_allocated_amount: &total_allocated_amount,
            total_allocated_share_price_num: &total_allocated_share_price_num,
            total_allocated_share_price_denom: &total_allocated_share_price_denom,
        }
        .emit();
    }

    /// Deallocates NEAR staking rewards from a recipient.
    pub fn deallocate(&mut self, recipient: AccountId, amount: U128) {
        self.check_not_paused();
        self.check_whitelisted();

        let deallocator = env::predecessor_account_id();
        let user_allocations = self
            .allocations
            .get_mut(&deallocator)
            .expect(ERR_NO_ALLOCATIONS);

        let allocation = user_allocations
            .get_mut(&recipient)
            .expect(ERR_NO_ALLOCATIONS_TO_RECIPIENT);

        require!(
            amount.0 <= allocation.near_amount,
            ERR_EXCESSIVE_DEALLOCATION
        );

        let remaining_amount = allocation.near_amount - amount.0;
        let share_price_num = allocation.share_price_num;
        let share_price_denom = allocation.share_price_denom;

        if remaining_amount == 0 {
            user_allocations.remove(&recipient);
            // refund the storage cost to the deallocator
            Promise::new(deallocator.clone())
                .transfer(NearToken::from_yoctonear(Self::get_storage_cost().0));
        } else {
            require!(remaining_amount >= ONE_NEAR, ERR_ALLOCATION_UNDER_ONE_NEAR);
            allocation.near_amount = remaining_amount;
        }

        let (
            total_allocated_amount,
            total_allocated_share_price_num,
            total_allocated_share_price_denom,
        ) = self.get_total_allocated(deallocator.clone());

        // emit event
        Event::DeallocatedEvent {
            user: &deallocator,
            recipient: &recipient,
            amount: &amount,
            total_amount: &remaining_amount.into(),
            share_price_num: &share_price_num.to_string(),
            share_price_denom: &share_price_denom.to_string(),
            total_allocated_amount: &total_allocated_amount,
            total_allocated_share_price_num: &total_allocated_share_price_num,
            total_allocated_share_price_denom: &total_allocated_share_price_denom,
        }
        .emit();
    }

    #[payable]
    /// Distributes NEAR staking rewards to a recipient. When distributing rewards in NEAR, the distributor must attach the full amount.
    pub fn distribute_rewards(&mut self, recipient: AccountId, in_near: bool) {
        self.check_not_paused();
        self.check_whitelisted();

        let distributor = env::predecessor_account_id();

        let user_allocations = self
            .allocations
            .get(&distributor)
            .expect(ERR_NO_ALLOCATIONS);

        require!(
            user_allocations.contains_key(&recipient),
            ERR_NO_ALLOCATIONS_TO_RECIPIENT
        );

        let (global_price_num, global_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.total_supply,
            self.tax_exempt_stake,
            self.fee,
        );
        let attached_near = env::attached_deposit();

        let distribution_info_result = self.internal_distribute(
            distributor.clone(),
            recipient.clone(),
            global_price_num,
            global_price_denom,
            in_near,
            attached_near,
        );

        match distribution_info_result {
            Err(error) => {
                env::panic_str(error.to_string().as_str());
            }
            Ok(distribution_info_opt) => {
                if distribution_info_opt.is_none() {
                    log!("No rewards to distribute");
                    if attached_near.as_yoctonear() > 0 {
                        Promise::new(distributor.clone()).transfer(attached_near);
                    }
                    return;
                }

                // refund any excess NEAR to distributor
                let distribution_info = distribution_info_opt.unwrap();
                if distribution_info.refund_amount > 0 {
                    Promise::new(distributor.clone())
                        .transfer(NearToken::from_yoctonear(distribution_info.refund_amount));
                }

                let (
                    total_allocated_amount,
                    total_allocated_share_price_num,
                    total_allocated_share_price_denom,
                ) = self.get_total_allocated(distributor.clone());

                // emit Distribute Rewards event
                Event::DistributedRewardsEvent {
                    user: distributor.clone(),
                    recipient: recipient.clone(),
                    shares: U128(distribution_info.shares_amount),
                    near_amount: U128(distribution_info.near_amount),
                    user_balance: self.ft_balance_of(distributor),
                    recipient_balance: self.ft_balance_of(recipient),
                    fees: distribution_info.fees.into(),
                    treasury_balance: self.ft_balance_of(self.treasury.clone()),
                    share_price_num: distribution_info.share_price_num.to_string(),
                    share_price_denom: distribution_info.share_price_denom.to_string(),
                    in_near,
                    total_allocated_amount,
                    total_allocated_share_price_num,
                    total_allocated_share_price_denom,
                }
                .emit();
            }
        }
    }

    #[payable]
    /// Distributes NEAR staking rewards to all recipients.
    pub fn distribute_all(&mut self, in_near: bool) {
        self.check_not_paused();
        self.check_whitelisted();

        // check if distributor has allocations
        let distributor = env::predecessor_account_id();
        require!(
            self.allocations.contains_key(&distributor),
            ERR_NO_ALLOCATIONS
        );

        // ensure distributor has enough NEAR and TruNEAR to complete the distribution
        let (required_shares, required_near) =
            self.get_rewards_distribution_amounts(&distributor, None, in_near);
        if self.ft_balance_of(distributor.clone()).0 < required_shares.0 {
            env::panic_str(ERR_INSUFFICIENT_TRUNEAR_BALANCE);
        }
        if env::attached_deposit().as_yoctonear() < required_near.0 {
            env::panic_str(ERR_INSUFFICIENT_NEAR_BALANCE);
        }

        let (total_allocated_amount, _, _) = self.get_total_allocated(distributor.clone());

        let (global_price_num, global_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.total_supply,
            self.tax_exempt_stake,
            self.fee,
        );

        // this holds the amount of NEAR we will need to refund to the distributor at the end of the distribution
        let mut refund_near_amount = env::attached_deposit();

        let distributor_allocations = self.allocations.get(&distributor).cloned().unwrap();

        let mut distributed_rewards_events: Vec<Event> = vec![];

        distributor_allocations.keys().for_each(|recipient| {
            let distribution_info_result = self.internal_distribute(
                distributor.clone(),
                recipient.clone(),
                global_price_num,
                global_price_denom,
                in_near,
                refund_near_amount,
            );

            match distribution_info_result {
                Err(error) => {
                    log!("Error distributing rewards: {}", error);
                }
                Ok(distribution_info_opt) => {
                    match distribution_info_opt {
                        None => log!("No rewards to distribute to {}", recipient),
                        Some(distribution_info) => {
                            // update the near amount left for the next distribution
                            refund_near_amount =
                                NearToken::from_yoctonear(distribution_info.refund_amount);
                            distributed_rewards_events.push(Event::DistributedRewardsEvent {
                                user: distributor.clone(),
                                recipient: recipient.clone(),
                                shares: U128(distribution_info.shares_amount),
                                near_amount: U128(distribution_info.near_amount),
                                user_balance: self.ft_balance_of(distributor.clone()),
                                recipient_balance: self.ft_balance_of(recipient.clone()),
                                fees: distribution_info.fees.into(),
                                treasury_balance: self.ft_balance_of(self.treasury.clone()),
                                share_price_num: distribution_info.share_price_num.to_string(),
                                share_price_denom: distribution_info.share_price_denom.to_string(),
                                in_near,
                                total_allocated_amount,
                                total_allocated_share_price_num: global_price_num.to_string(),
                                total_allocated_share_price_denom: global_price_denom.to_string(),
                            });
                        }
                    };
                }
            }
        });

        // refund any excess NEAR to distributor
        if refund_near_amount.as_yoctonear() > 0 {
            Promise::new(distributor.clone()).transfer(refund_near_amount);
        }

        // emit DistributedRewardsEvent events
        distributed_rewards_events
            .iter()
            .for_each(|event| event.emit());

        // emit DistributedAllEvent
        Event::DistributedAllEvent { user: &distributor }.emit();
    }

    /// Withdraws the unstaked amount associated with the unstake_nonce.
    pub fn withdraw(&mut self, unstake_nonce: U128) -> Option<Promise> {
        self.check_not_paused();
        self.check_not_locked();
        self.is_locked = true;

        self.check_whitelisted();

        self.internal_withdraw(unstake_nonce)
    }

    #[private]
    #[init(ignore_state)]
    /// Migrates the contract state.
    pub fn migrate() -> Self {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            ERR_INVALID_CALLER
        );

        // read the current contract state
        let state = env::state_read().expect(ERR_NOT_INITIALIZED);

        // perform the migration from the previous version and return the new contract state
        VersionedNearStaker::V1(state).into()
    }

    #[private]
    /// Checks if the withdrawal was successful and performs associated accounting.
    pub fn withdraw_callback(
        &mut self,
        unstake_nonce: U128,
        withdrawn_amount: U128,
        pool_id: AccountId,
        pre_withdraw_staker_balance: NearToken,
        request_amount: U128,
        #[callback_result] staker_unstaked_balance: Result<U128, PromiseError>,
    ) {
        self.is_locked = false;

        // The staker_unstaked_balance will be the amount that is meant to be staked but is part of the
        // unstaked balance due to rounding on the pool. We account for it as staked.
        let staker_unstaked_balance = match staker_unstaked_balance {
            Ok(amount) => amount.0,
            Err(_) => {
                log!("Failed to withdraw: {}", ERR_CALLBACK_FAILED);
                return;
            }
        };

        log!(
            "Unstaked amount {}. Unaccounted unstake amount {}. Pre balance {}. Post balance {}",
            withdrawn_amount.0,
            staker_unstaked_balance,
            pre_withdraw_staker_balance,
            env::account_balance()
        );

        // we add the amount withdrawn to the total amount of not yet claimed withdrawals
        self.withdrawn_amount += withdrawn_amount.0;

        // we reset the pools requested unstake amount to 0
        self.delegation_pools.entry(pool_id).and_modify(|pool| {
            pool.total_unstaked = U128(0);
        });

        self.finalize_withdraw(unstake_nonce, request_amount);
    }

    #[private]
    /// Handles the stake promise, performing associated accounting if successful and error handling if not.
    pub fn finalize_deposit_and_stake(
        &mut self,
        pool_id: AccountId,
        amount: U128,
        caller: AccountId,
        #[callback_result] stake_result: Result<U128, PromiseError>,
    ) {
        self.is_locked = false;

        if stake_result.is_err() {
            log!("Staking failed. Refunding {} to caller", amount.0);
            Promise::new(caller).transfer(NearToken::from_yoctonear(amount.0));
            return;
        }
        let account_total_balance: U128 = stake_result.unwrap();
        let pool = self.delegation_pools.get_mut(&pool_id).unwrap();
        // The new total staked is given by the total pool account balance minus the total requested unstake amount.
        // We require that the new total staked is greater than the previous total staked amount.
        if pool.total_staked >= (account_total_balance.0 - pool.total_unstaked.0).into() {
            log!("Staking failed");
            return;
        };

        let (share_price_num, share_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );
        let shares_amount =
            Self::convert_to_shares(amount.0, share_price_num, share_price_denom, false);

        // The new total staked on the pool is given by the account_total_balance minus the pool's
        // total requested unstake. To get the increased stake we subtract the new total staked amount from
        // the previous total staked amount.
        let increased_stake = account_total_balance.0 - pool.total_unstaked.0 - pool.total_staked.0;

        // We then add the intended amount staked to the pool total_staked and staker total_staked. We add this rather than the increased_stake
        // as due to rounding on the pool it may stake slightly less than the intended amount, which can cause our share price to drop.
        pool.total_staked = (pool.total_staked.0 + amount.0).into();
        self.total_staked += amount.0;
        self.tax_exempt_stake += amount.0;
        log!("Updated total_staked: {}", self.total_staked);

        // finally mint the equivalent TruNEAR to the user
        self.internal_mint(shares_amount, caller.clone());

        // emit Deposited event
        Event::DepositedEvent {
            user_id: &caller,
            amount: &amount,
            amount_staked: &U128(increased_stake),
            user_balance: &U128(self.token.accounts.get(&caller).unwrap_or(0)),
            shares_amount: &U128(shares_amount),
            total_staked: &U128(self.total_staked),
            total_supply: &U128(self.token.total_supply),
            share_price_num: &share_price_num.to_string(),
            share_price_denom: &share_price_denom.to_string(),
            epoch: &env::epoch_height().into(),
            pool_id: &pool_id,
        }
        .emit();
    }

    #[private]
    /// Handles the unstake promise, performing associated accounting if successful.
    pub fn finalize_unstake(
        &mut self,
        pool_id: AccountId,
        amount: U128,
        caller: AccountId,
        pre_unstake_staker_balance: NearToken,
        share_price_num: String,
        share_price_denom: String,
        shares_amount: U128,
        withdraw_occurred: bool,
        attached_near: NearToken,
        unstake_epoch: u64,
        #[callback_result] new_unstaked_amount: Result<U128, PromiseError>,
    ) {
        self.is_locked = false;

        let new_unstaked_amount = match new_unstaked_amount {
            Ok(amount) => amount.0,
            Err(_) => {
                log!("Failed to unstake: {}", ERR_CALLBACK_FAILED);
                self.internal_mint(shares_amount.0, caller.clone());
                Promise::new(caller).transfer(attached_near);
                return;
            }
        };
        let pool = self.delegation_pools.get_mut(&pool_id).unwrap();

        if withdraw_occurred {
            self.withdrawn_amount += pool.total_unstaked.0;
            // if a withdraw occurred, the new total unstake amount on the pool should be the amount
            // requested in this unstake.
            pool.total_unstaked = amount;
        } else {
            // if no withdraw occurred we add the requested unstake amount to the pool total unstaked amount
            pool.total_unstaked = (pool.total_unstaked.0 + amount.0).into();
        }

        // update delegation pool and total_staked
        pool.last_unstake = Some(unstake_epoch);
        pool.total_staked = (pool.total_staked.0 - amount.0).into();
        self.total_staked -= amount.0;
        self.tax_exempt_stake = self.tax_exempt_stake.saturating_sub(amount.0);
        log!("Updated total_staked: {}", self.total_staked);

        log!(
            "New unstaked amount {}. Pool total unstaked {}. Was withdrawn: {}. Pre balance {}. Post balance {}",
            new_unstaked_amount,
            pool.total_unstaked.0,
            withdraw_occurred,
            pre_unstake_staker_balance,
            env::account_balance()
        );

        // create the unstake request
        self.unstake_nonce += 1;

        let unstake_request = UnstakeRequest {
            pool_id: pool_id.clone(),
            near_amount: amount.0,
            user: caller.clone(),
            epoch: unstake_epoch,
        };

        self.unstake_requests
            .insert(self.unstake_nonce, unstake_request);

        // refund any excess NEAR to allocator
        let storage_cost = NearToken::from_yoctonear(Self::get_storage_cost().0);
        if attached_near > storage_cost {
            Promise::new(caller.clone()).transfer(attached_near.checked_sub(storage_cost).unwrap());
        }

        // emit Unstaked event
        Event::UnstakedEvent {
            user_id: &caller,
            amount: &amount,
            user_balance: &U128(self.token.accounts.get(&caller).unwrap_or(0)),
            shares_amount: &shares_amount,
            total_staked: &U128(self.total_staked),
            total_supply: &U128(self.token.total_supply),
            share_price_num: &share_price_num,
            share_price_denom: &share_price_denom,
            unstake_nonce: &U128(self.unstake_nonce),
            epoch: &unstake_epoch.into(),
            pool_id: &pool_id,
        }
        .emit();
    }

    #[private]
    /// Handles the get_account_total_balance promises, updating the total_staked and total_staked_last_updated_at.
    pub fn total_staked_callback(&mut self) {
        self.is_locked = false;
        let mut total_staked_sum = 0;
        let mut account_total_balances: Vec<U128> = vec![];

        // ensure all ping and get_account_total_balance promises succeeded
        for i in 0..self.delegation_pools_list.len() {
            let pool_id: AccountId = self.delegation_pools_list[i].clone();
            match env::promise_result(i as u64) {
                PromiseResult::Successful(result) => {
                    if let Ok(account_total_balance) =
                        near_sdk::serde_json::from_slice::<U128>(&result)
                    {
                        account_total_balances.push(account_total_balance);
                        log!(
                            "Promise success for pool {}, account total balance: {}",
                            pool_id,
                            account_total_balance.0
                        );
                    } else {
                        log!(
                            "Error deserializing the account total balance for pool {}",
                            pool_id
                        );
                        return;
                    }
                }
                PromiseResult::Failed => {
                    log!("Error fetching the staked amount from pool {}", pool_id);
                    return;
                }
            }
        }
        // if all promises succeed, we can now update the pool total_staked amounts and the staker total_staked amount
        for i in 0..account_total_balances.len() {
            let pool_id: AccountId = self.delegation_pools_list[i].clone();
            let account_total_balance = account_total_balances[i].clone();
            // The account_total_balance returns the staked + unstaked balance on the pool.
            // To calculate the actual amount staked, we need to subtract the unstaked balance.
            // Due to rounding errors on the staking pool we need to keep track of the total_unstaked amounts ourselves in pool.total_unstaked.
            let pool_mut = self.delegation_pools.get_mut(&pool_id).unwrap();
            // the new pool total_staked amount is given by the pool total balance minus the total requested unstake amount
            pool_mut.total_staked = U128::from(account_total_balance.0 - pool_mut.total_unstaked.0);
            // we then add the total amount staked on the pool to the total staked by our staker
            total_staked_sum += pool_mut.total_staked.0;
        }

        self.total_staked = total_staked_sum;
        self.total_staked_last_updated_at = env::epoch_height();
        log!("Updated total_staked: {}", self.total_staked);
    }
}

// Unit tests
#[cfg(test)]
mod tests;
