use crate::*;
use near_contract_standards::storage_management::{
    StorageBalance, StorageBalanceBounds, StorageManagement,
};
use near_sdk::{env, AccountId, NearToken};

#[allow(unused_variables)]
#[near]
impl StorageManagement for NearStaker {
    /// Registers an account to be able to store token data.
    #[allow(unused_variables)]
    #[payable]
    fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        self.token.storage_deposit(account_id, registration_only)
    }

    /// storage_withdraw is not supported. storage_balance_of should be used instead.
    #[payable]
    fn storage_withdraw(&mut self, amount: Option<NearToken>) -> StorageBalance {
        env::panic_str("Storage withdraw is not supported.");
    }

    /// storage_unregister is not supported. We do not allow users to unregister their accounts.
    #[payable]
    fn storage_unregister(&mut self, _force: Option<bool>) -> bool {
        env::panic_str("Storage unregister is not supported.");
    }

    /// Returns the storage balance bounds.
    fn storage_balance_bounds(&self) -> StorageBalanceBounds {
        self.token.storage_balance_bounds()
    }

    /// Returns the storage balance struct of the account.
    fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.token.storage_balance_of(account_id)
    }
}
