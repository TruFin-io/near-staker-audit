use near_sdk::{ext_contract, json_types::U128, AccountId};

#[ext_contract(staking_pool)]
trait _StakingPool {
    fn get_account_staked_balance(&self, account_id: AccountId) -> U128;
    fn get_account_unstaked_balance(&self, account_id: AccountId) -> U128;
    fn get_account_total_balance(&self, account_id: AccountId) -> U128;
    fn is_account_unstaked_balance_available(&self, account_id: AccountId) -> bool;
    fn deposit_and_stake(&mut self);
    fn ping(&mut self);
    fn unstake(&mut self, amount: U128);
    fn withdraw(&mut self, amount: U128);
}
