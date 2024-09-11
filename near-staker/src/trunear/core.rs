use near_contract_standards::fungible_token::{FungibleTokenCore, FungibleTokenResolver};
use near_sdk::json_types::U128;
use near_sdk::{near, AccountId, PromiseOrValue};

use crate::*;

#[near]
impl FungibleTokenCore for NearStaker {
    /// Sends TruNEAR to another registered account.
    #[payable]
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        self.token.ft_transfer(receiver_id, amount, memo)
    }

    /// Transfers with a callback to the receiver contract.
    #[payable]
    fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<U128> {
        self.token.ft_transfer_call(receiver_id, amount, memo, msg)
    }

    /// Returns the total supply of the token.
    fn ft_total_supply(&self) -> U128 {
        self.token.ft_total_supply()
    }

    /// Returns the balance of the account. If the account doesn't exist it returns `"0"`.
    fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        self.token.ft_balance_of(account_id)
    }
}

#[near]
impl FungibleTokenResolver for NearStaker {
    #[private]
    /// Callback used inside ft_transfer_call to handle the result of ft_on_transfer.
    fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
    ) -> U128 {
        self.token
            .ft_resolve_transfer(sender_id, receiver_id, amount)
            .0
            .into()
    }
}
