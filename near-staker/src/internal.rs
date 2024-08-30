// Private Methods
use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::FungibleTokenCore;
use near_sdk::{
    env, json_types::U128, log, require, serde_json::json, AccountId, NearToken, Promise,
};

use crate::constants::*;
use crate::errors::*;
use crate::events::*;
use crate::math::*;
use crate::types::*;
use crate::whitelist::WhitelistTrait;
use crate::NearStaker;

// Internal Methods

impl NearStaker {
    /// Internal Checks ///

    /// Checks that the contract is not currently paused.
    pub(crate) fn check_not_paused(&self) {
        require!(!self.is_paused, ERR_PAUSED);
    }

    /// Checks that the contract is not currently executing a cross contract call.
    pub(crate) fn check_not_locked(&self) {
        require!(!self.is_locked, ERR_LOCKED);
    }

    /// Checks that the caller is the owner of the contract.
    pub(crate) fn check_owner(&self) {
        require!(
            self.owner_id == env::predecessor_account_id(),
            ERR_ONLY_OWNER
        );
    }

    /// Checks that the caller is a whitelisted contract user.
    pub(crate) fn check_whitelisted(&self) {
        require!(
            self.is_whitelisted(env::predecessor_account_id()),
            ERR_USER_NOT_WHITELISTED
        );
    }

    /// Checks that the deposit amount is greater than the staker's minimum deposit amount.
    pub(crate) fn check_min_deposit_amount(&self, amount: u128) {
        require!(amount >= self.min_deposit, ERR_STAKE_BELOW_MIN_DEPOSIT);
    }

    /// Checks that the chosen delegation pool exists and is enabled.
    pub(crate) fn check_pool(&self, pool_id: AccountId) {
        let pool = self
            .delegation_pools
            .get(&pool_id)
            .expect(ERR_POOL_DOES_NOT_EXIST);
        require!(pool.state == ValidatorState::ENABLED, ERR_POOL_NOT_ENABLED);
    }

    /// Checks that the contract total staked and share price are up to date.
    pub(crate) fn check_contract_in_sync(&self) {
        require!(
            self.total_staked_last_updated_at == env::epoch_height(),
            ERR_NOT_IN_SYNC
        );
    }

    /// Internal Methods ///

    /// Stakes the specified amount of NEAR tokens into the specified delegation pool.
    pub(crate) fn internal_deposit_and_stake(
        &mut self,
        pool_id: AccountId,
        amount: u128,
        caller: AccountId,
    ) -> Promise {
        self.check_pool(pool_id.clone());

        self.check_min_deposit_amount(amount);

        self.check_contract_in_sync();

        Self::send_stake_promises(pool_id, amount, caller)
    }

    /// Sends the stake promises to the staking pool upon user deposit.
    pub(crate) fn send_stake_promises(
        pool_id: AccountId,
        amount: u128,
        caller: AccountId,
    ) -> Promise {
        let staker_id: AccountId = env::current_account_id();

        let staker_arg = json!({ "account_id": staker_id }).to_string().into_bytes();

        // we first call deposit_and_stake followed by get_account_total_balance to ensure the stake has been added
        Promise::new(pool_id.clone())
            .function_call(
                "deposit_and_stake".to_owned(),
                NO_ARGS,
                NearToken::from_yoctonear(amount),
                XCC_GAS,
            )
            .function_call(
                "get_account_total_balance".to_owned(),
                staker_arg,
                NO_DEPOSIT,
                VIEW_GAS,
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(XCC_GAS)
                    .finalize_deposit_and_stake(pool_id, U128(amount), caller),
            )
    }

    /// Unstakes NEAR from the specified pool, withdrawing first if necessary.
    pub(crate) fn send_unstake_promises(
        &mut self,
        pool_id: AccountId,
        amount: u128,
        caller: AccountId,
        attached_near: NearToken,
    ) -> Promise {
        // ensure amount of shares burned is greater than 0
        let (share_price_num, share_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.token.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );

        let shares_amount =
            Self::convert_to_shares(amount, share_price_num, share_price_denom, false);
        if shares_amount == 0 {
            log!("Failed to unstake: {}", ERR_UNSTAKE_AMOUNT_TOO_LOW);
            self.is_locked = false;
            return Promise::new(caller).transfer(attached_near);
        }

        // burn user shares and update total staked to keep share price the same
        self.internal_burn(shares_amount, caller.clone());
        self.total_staked -= amount;
        self.tax_exempt_stake = self.tax_exempt_stake.saturating_sub(amount);

        // prepare unstake arguments
        let unstake_amount = json!({ "amount": NearToken::from_yoctonear(amount) })
            .to_string()
            .into_bytes();

        let staker_id_arg = json!({ "account_id": env::current_account_id()})
            .to_string()
            .into_bytes();

        let pre_unstake_staker_balance = env::account_balance();
        let mut promise = Promise::new(pool_id.clone());

        // we fetch the total amount requested for unstake on the given pool and last unstake epoch as we should withdraw
        // any unlocked stake into the staker before unlocking more due to the 4 epoch wait period
        let pool_info = self.delegation_pools.get(&pool_id).unwrap();
        let mut withdraw_occurred: bool = false;

        if let Some(last_unstake) = pool_info.last_unstake {
            if last_unstake + NUM_EPOCHS_TO_UNLOCK <= env::epoch_height()
                && pool_info.total_unstaked.0 > 0
            {
                // if there is stake to withdraw, we withdraw it before calling unstake
                let withdraw_args = json!({ "amount": pool_info.total_unstaked })
                    .to_string()
                    .into_bytes();
                promise = promise.function_call(
                    "withdraw".to_owned(),
                    withdraw_args,
                    NO_DEPOSIT,
                    XCC_GAS,
                );
                withdraw_occurred = true;
            }
        }
        // call unstake on the pool and fetch the new account unstaked balance
        promise = promise
            .function_call("unstake".to_owned(), unstake_amount, NO_DEPOSIT, XCC_GAS)
            .function_call(
                "get_account_unstaked_balance".to_owned(),
                staker_id_arg,
                NO_DEPOSIT,
                VIEW_GAS,
            );
        promise.then(
            Self::ext(env::current_account_id())
                .with_static_gas(XCC_GAS)
                .finalize_unstake(
                    pool_id,
                    U128(amount),
                    caller,
                    pre_unstake_staker_balance,
                    share_price_num.to_string(),
                    share_price_denom.to_string(),
                    U128(shares_amount),
                    withdraw_occurred,
                    attached_near,
                    env::epoch_height(),
                ),
        )
    }

    /// Unstakes the specified amount of NEAR tokens from the specified delegation pool.
    pub(crate) fn internal_unstake(
        &mut self,
        pool_id: AccountId,
        amount: u128,
        caller: AccountId,
    ) -> Promise {
        self.check_contract_in_sync();

        let attached_near = env::attached_deposit();
        require!(
            attached_near.as_yoctonear() >= Self::get_storage_cost().0,
            ERR_STORAGE_DEPOSIT_TOO_SMALL
        );

        // We must check that there is no pending unstake from previous epochs on the pool. If there is, we cannot unlock as
        // it would push back the pending unstake by a further four epochs.
        let pool_last_unstake = self.delegation_pools.get(&pool_id).unwrap().last_unstake;
        let current_epoch = env::epoch_height();

        // we can unlock if the last unstake happened in the same epoch or more than 4 epochs ago (there is withdrawable stake)
        if let Some(last_unstake) = pool_last_unstake {
            require!(
                last_unstake == current_epoch
                    || last_unstake + NUM_EPOCHS_TO_UNLOCK <= current_epoch,
                ERR_UNSTAKE_LOCKED
            );
        }

        // if the total staked is up to date, check the requested unstake amount
        let amount = self.internal_check_unstake_amount(&pool_id, amount, &caller);

        self.send_unstake_promises(pool_id, amount, caller, attached_near)
    }

    /// Updates the total staked amount.   
    pub(crate) fn internal_update_stake(&self) -> Promise {
        let staker_id = env::current_account_id();
        let staker_arg = json!({ "account_id": staker_id }).to_string().into_bytes();

        // For each pool, we first call ping on each pool to ensure the pool is synced and up to date.
        // We then fetch the staked + unstaked (total) balance of our staker on the pool.
        let combined_promises = self.delegation_pools_list.iter().flat_map(|pool_id| {
            vec![Promise::new(pool_id.clone())
                .function_call("ping".to_owned(), NO_ARGS, NO_DEPOSIT, XCC_GAS)
                .function_call(
                    "get_account_total_balance".to_owned(),
                    staker_arg.to_owned(),
                    NO_DEPOSIT,
                    VIEW_GAS,
                )]
        });

        combined_promises.reduce(|acc, p| acc.and(p)).unwrap()
    }

    /// Executes the unstake requested associated with the given nonce.
    pub(crate) fn internal_withdraw(&mut self, unstake_nonce: U128) -> Option<Promise> {
        let sender = env::predecessor_account_id();
        // we first perform checks on the unlock request before withdrawing anything
        let UnstakeRequest {
            pool_id,
            user,
            near_amount,
            epoch,
        } = self
            .unstake_requests
            .get(&unstake_nonce.0)
            .expect(ERR_INVALID_NONCE);

        require!(*user == sender, ERR_SENDER_MUST_BE_RECEIVER);
        require!(
            epoch + NUM_EPOCHS_TO_UNLOCK <= env::epoch_height(),
            ERR_WITHDRAW_NOT_READY
        );

        let pool_info = self.delegation_pools.get(pool_id).unwrap();

        // we first check if there is stake to be withdrawn from the pool
        // and if there is, if the last unstake happened four or more epochs ago, as otherwise it is
        // a recently unstaked amount that cannot be withdrawn yet.
        if pool_info.last_unstake.unwrap() + NUM_EPOCHS_TO_UNLOCK <= env::epoch_height()
            && pool_info.total_unstaked.0 > 0
        {
            // if there is withdrawable stake, we withdraw it and then fetch the new unstaked balance
            let staker_id = env::current_account_id();
            let amount_args = json!({ "amount": pool_info.total_unstaked})
                .to_string()
                .into_bytes();
            let staker_arg = json!({ "account_id": staker_id }).to_string().into_bytes();

            return Some(
                Promise::new(pool_id.clone())
                    .function_call("withdraw".to_owned(), amount_args, NO_DEPOSIT, XCC_GAS)
                    .function_call(
                        "get_account_unstaked_balance".to_owned(),
                        staker_arg,
                        NO_DEPOSIT,
                        VIEW_GAS,
                    )
                    .then(
                        Self::ext(staker_id)
                            .with_static_gas(XCC_GAS)
                            .withdraw_callback(
                                unstake_nonce,
                                pool_info.total_unstaked,
                                pool_id.clone(),
                                env::account_balance(),
                                U128::from(*near_amount),
                            ),
                    ),
            );
        }
        // if there is nothing to withdraw (because it has already been withdrawn by previous withdrawals or unstakes)
        // we can finalize the withdraw
        self.finalize_withdraw(unstake_nonce, U128::from(*near_amount));
        // set locked flag to false as no cross-contract call was made
        self.is_locked = false;
        None
    }

    /// Calculates fees of the taxable amount and mints shares to the treasury.
    pub(crate) fn internal_collect_fees(&mut self) {
        let (share_price_num, share_price_denom) = Self::internal_share_price(
            self.total_staked,
            self.ft_total_supply().0,
            self.tax_exempt_stake,
            self.fee,
        );

        let taxable_amount = self.total_staked.saturating_sub(self.tax_exempt_stake);

        let near_amount_increase_treasury = mul_div_with_rounding(
            U256::from(taxable_amount),
            U256::from(self.fee),
            U256::from(FEE_PRECISION),
            false,
        );

        log!(
            "NEAR collected as fees: {}",
            near_amount_increase_treasury.to_string()
        );

        let share_increase_treasury = Self::convert_to_shares(
            near_amount_increase_treasury.as_u128(),
            share_price_num,
            share_price_denom,
            false,
        );

        if share_increase_treasury > 0 {
            // mint the shares to the treasury
            self.internal_mint(share_increase_treasury, self.treasury.clone());

            // update tax exempt stake
            self.tax_exempt_stake = self.total_staked;

            // emit FeesCollected event
            Event::FeesCollectedEvent {
                shares_minted: &U128(share_increase_treasury),
                treasury_balance: &self.ft_balance_of(self.treasury.clone()),
                share_price_num: &share_price_num.to_string(),
                share_price_denom: &share_price_denom.to_string(),
                epoch: &env::epoch_height().into(),
            }
            .emit();
        };
    }

    /// Mints an amount of TruNEAR tokens to a specified account.
    pub(crate) fn internal_mint(&mut self, shares_amount: u128, recipient: AccountId) {
        let account_balance = self.token.accounts.get(&recipient).unwrap_or(0);

        self.token
            .accounts
            .insert(&recipient, &(account_balance + shares_amount));

        self.token.total_supply += shares_amount;

        // emit a mint event
        FtMint {
            owner_id: &recipient,
            amount: near_sdk::json_types::U128(shares_amount),
            memo: None,
        }
        .emit();
    }

    /// Burns an amount of TruNEAR tokens from a specified account.
    pub(crate) fn internal_burn(&mut self, shares_burned: u128, user: AccountId) {
        let user_balance = self.token.accounts.get(&user).unwrap_or(0);
        require!(
            shares_burned <= user_balance,
            ERR_INSUFFICIENT_TRUNEAR_BALANCE
        );
        let new_user_balance = user_balance - shares_burned;

        self.token.accounts.insert(&user, &new_user_balance);

        self.token.total_supply -= shares_burned;

        // emit a burn event
        FtBurn {
            owner_id: &user,
            amount: U128(shares_burned),
            memo: Some("unstake"),
        }
        .emit();
    }

    /// Calculates rewards for a specified account and distributes them if there are any.
    pub(crate) fn internal_distribute(
        &mut self,
        distributor: AccountId,
        recipient: AccountId,
        global_price_num: U256,
        global_price_denom: U256,
        in_near: bool,
        attached_near: NearToken,
    ) -> Result<Option<DistributionInfo>, Box<dyn std::error::Error>> {
        let allocation = self
            .allocations
            .get_mut(&distributor)
            .unwrap()
            .get_mut(&recipient)
            .unwrap();

        // check if there are any rewards to distribute. If not, return no distribution
        if allocation.share_price_num / allocation.share_price_denom
            == global_price_num / global_price_denom
        {
            return Ok(None);
        }

        // calculate the amount of rewards that have accumulated
        let mut shares_to_move = Self::internal_calculate_distribution_amount(
            allocation,
            global_price_num,
            global_price_denom,
        );

        // calculate the distribution fee if applicable
        let fees = shares_to_move * (self.distribution_fee as u128) / (FEE_PRECISION as u128);

        if fees > 0 {
            shares_to_move -= fees;
            self.token
                .internal_transfer(&distributor, &self.treasury, fees, None);
        }

        // calculate the amount of rewards in NEAR
        let near_amount = NearToken::from_yoctonear(Self::convert_to_assets(
            shares_to_move,
            global_price_num,
            global_price_denom,
            false,
        ));

        let refund_amount;
        if in_near {
            if near_amount > attached_near {
                // if don't have enough NEAR return an error
                return Err(ERR_INSUFFICIENT_NEAR_BALANCE.into());
            };

            // calculate the refund amount and transfer NEAR to the recipient
            refund_amount = attached_near.checked_sub(near_amount).unwrap();
            Promise::new(recipient.clone()).transfer(near_amount);
        } else {
            let distributor_trunear_balance = self.token.ft_balance_of(distributor.clone()).0;
            if shares_to_move > distributor_trunear_balance {
                // if don't have enough TruNEAR return an error
                return Err(ERR_INSUFFICIENT_TRUNEAR_BALANCE.into());
            };

            // register the recipient if they don't have a TruNEAR account
            if !self.token.accounts.contains_key(&recipient) {
                self.token.accounts.insert(&recipient, &0);
            }

            // transfer TruNEAR to the recipient and refund any attached NEAR
            refund_amount = attached_near;
            self.token
                .internal_transfer(&distributor, &recipient, shares_to_move, None);
        }

        // update the allocation and return the distribution info
        allocation.share_price_num = global_price_num;
        allocation.share_price_denom = global_price_denom;

        Ok(Some(DistributionInfo {
            shares_amount: shares_to_move,
            near_amount: near_amount.as_yoctonear(),
            refund_amount: refund_amount.as_yoctonear(),
            fees,
            share_price_num: allocation.share_price_num,
            share_price_denom: allocation.share_price_denom,
        }))
    }

    /// Performs checks on the amount the user requested to unstake
    /// and returns the amount that will be unstaked.
    pub(crate) fn internal_check_unstake_amount(
        &self,
        pool_id: &AccountId,
        amount: u128,
        caller: &AccountId,
    ) -> u128 {
        // check if user has enough TruNEAR to unstake
        let max_withdraw = self.max_withdraw(caller.clone()).0;
        require!(max_withdraw >= amount, ERR_INVALID_UNSTAKE_AMOUNT);

        // if the user's remaining balance falls below one NEAR, unstake the entire user stake
        let unstake_amount = if max_withdraw - amount < ONE_NEAR {
            max_withdraw
        } else {
            amount
        };

        // check if there's enough staked balance to unstake on the pool
        require!(
            self.delegation_pools.get(pool_id).unwrap().total_staked >= U128(unstake_amount),
            ERR_INSUFFICIENT_FUNDS_ON_POOL
        );

        unstake_amount
    }

    /// Transfers the withdrawn NEAR to the user and emits the withdrawal event.
    pub(crate) fn finalize_withdraw(&mut self, unstake_nonce: U128, request_amount: U128) {
        // checks that the contract has enough NEAR to withdraw. This should always be the case unless something very unexpected happened.
        if self.withdrawn_amount < request_amount.0 {
            log!("Failed to withdraw: {}", ERR_INSUFFICIENT_STAKER_BALANCE);
            return;
        }

        self.withdrawn_amount -= request_amount.0;

        let UnstakeRequest {
            pool_id,
            user,
            near_amount,
            epoch: _,
        } = self.unstake_requests.remove(&unstake_nonce.0).unwrap();

        // transfer the withdrawn NEAR plus storage costs to the user and update the contract balance
        let total_transfer_amount = near_amount + Self::get_storage_cost().0;
        Promise::new(user.clone()).transfer(NearToken::from_yoctonear(total_transfer_amount));

        Event::WithdrawalEvent {
            user: &user,
            amount: &near_amount.into(),
            unstake_nonce: &unstake_nonce,
            epoch: &env::epoch_height().into(),
            delegation_pool: &pool_id,
        }
        .emit();
    }

    /// Pure functions ///

    /// Calculates the share price using the provided parameters.
    pub(crate) fn internal_share_price(
        total_staked: u128,
        shares_supply: u128,
        tax_exempt_stake: u128,
        fee: u16,
    ) -> (U256, U256) {
        if shares_supply == 0 {
            return (U256::from(SHARE_PRICE_SCALING_FACTOR), U256::from(1));
        };
        // the taxable amount is the accrued rewards that we have not yet accounted for
        let taxable_amount = total_staked.saturating_sub(tax_exempt_stake);
        // we then collect fees on the taxable amount and calculate the new total_staked; this ensures that when we mint shares
        // from fees to the treasury, the share price does not change
        let taxed_total_staked =
            total_staked * (FEE_PRECISION as u128) - taxable_amount * fee as u128;
        let price_num = mul256(taxed_total_staked, SHARE_PRICE_SCALING_FACTOR);
        let price_denom = mul256(shares_supply, FEE_PRECISION as u128);

        (price_num, price_denom)
    }

    /// Converts an amount of NEAR tokens to the equivalent TruNEAR amount using the specified rounding.
    pub(crate) fn convert_to_shares(
        assets: u128,
        share_price_num: U256,
        share_price_denom: U256,
        rounding_up: bool,
    ) -> u128 {
        let shares = mul_div_with_rounding(
            U256::from(assets),
            share_price_denom,
            share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR),
            rounding_up,
        );
        shares.as_u128()
    }

    /// Converts an amount of TruNEAR tokens to the equivalent NEAR amount using the specified rounding.
    pub(crate) fn convert_to_assets(
        shares: u128,
        share_price_num: U256,
        share_price_denom: U256,
        rounding_up: bool,
    ) -> u128 {
        mul_div_with_rounding(
            U256::from(shares),
            share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR),
            share_price_denom,
            rounding_up,
        )
        .as_u128()
    }

    /// Calculates the updated allocation values.
    pub(crate) fn calculate_updated_allocation(
        existing: &Allocation,
        amount: u128,
        global_share_price_num: U256,
        global_share_price_denom: U256,
    ) -> Allocation {
        let share_price_denom = mul_div_with_rounding(
            U256::from(existing.near_amount),
            existing.share_price_denom,
            existing.share_price_num / SHARE_PRICE_SCALING_FACTOR,
            false,
        ) + mul_div_with_rounding(
            U256::from(amount),
            global_share_price_denom,
            global_share_price_num / SHARE_PRICE_SCALING_FACTOR,
            false,
        );

        let share_price_num = mul256(existing.near_amount + amount, SHARE_PRICE_SCALING_FACTOR);
        Allocation {
            near_amount: existing.near_amount + amount,
            share_price_num,
            share_price_denom,
        }
    }

    /// Calculates the distribution amount for the given allocation.
    pub(crate) fn internal_calculate_distribution_amount(
        allocation: &Allocation,
        global_share_price_num: U256,
        global_share_price_denom: U256,
    ) -> u128 {
        let distribution_amount = mul_div_with_rounding(
            U256::from(allocation.near_amount),
            allocation.share_price_denom,
            allocation.share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR),
            false,
        ) - mul_div_with_rounding(
            U256::from(allocation.near_amount),
            global_share_price_denom,
            global_share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR),
            false,
        );

        distribution_amount.as_u128()
    }
}
