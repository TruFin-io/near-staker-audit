use near_sdk::{env, near, require, AccountId};

use crate::errors::*;
use crate::events::Event;
use crate::*;

/// Whitelist trait for whitelisting and blacklisting users.
pub trait WhitelistTrait {
    fn add_agent(&mut self, agent_id: AccountId);
    fn remove_agent(&mut self, agent_id: AccountId);
    fn add_user_to_whitelist(&mut self, user_id: AccountId);
    fn add_user_to_blacklist(&mut self, user_id: AccountId);
    fn clear_user_status(&mut self, user_id: AccountId);
    fn is_whitelisted(&self, user_id: AccountId) -> bool;
    fn is_blacklisted(&self, user_id: AccountId) -> bool;
    fn is_agent(&self, agent_id: AccountId) -> bool;
    fn get_current_user_status(&self, user_id: AccountId) -> UserStatus;
    fn check_agent(&self, agent_id: AccountId);
}

#[near]
impl WhitelistTrait for NearStaker {
    /// Adds a new agent.
    fn add_agent(&mut self, agent_id: AccountId) {
        self.check_agent(env::predecessor_account_id());

        // check that the new agent is not the owner
        require!(agent_id != self.owner_id, ERR_OWNER_CANNOT_BE_ADDED);

        // add the agent and fail if the user was already an agent
        require!(
            self.whitelist.agents.insert(agent_id.clone()),
            ERR_AGENT_ALREADY_EXISTS
        );

        Event::AgentAddedEvent {
            account_id: &agent_id,
        }
        .emit();
    }

    /// Removes an existing agent.
    fn remove_agent(&mut self, agent_id: AccountId) {
        self.check_agent(env::predecessor_account_id());

        // check if the account is not the owner
        require!(agent_id != self.owner_id, ERR_OWNER_CANNOT_BE_REMOVED);

        // remove the agent and fail if the user was not an agent
        require!(
            self.whitelist.agents.remove(&agent_id),
            ERR_AGENT_DOES_NOT_EXIST
        );

        Event::AgentRemovedEvent {
            account_id: &agent_id,
        }
        .emit();
    }

    /// Adds a user to the whitelist.
    fn add_user_to_whitelist(&mut self, user_id: AccountId) {
        self.check_agent(env::predecessor_account_id());

        // get the current status of the user
        let current_user_status = self.get_current_user_status(user_id.clone());

        // check if the user is already whitelisted
        require!(
            current_user_status != UserStatus::WHITELISTED,
            ERR_USER_ALREADY_WHITELISTED
        );

        // add the user to the whitelist
        self.whitelist
            .users
            .set(user_id.clone(), Some(UserStatus::WHITELISTED));

        // emit the event
        Event::WhitelistStateChangedEvent {
            account_id: &user_id,
            old_status: current_user_status,
            new_status: UserStatus::WHITELISTED,
        }
        .emit();
    }

    /// Adds a user to the blacklist.
    fn add_user_to_blacklist(&mut self, user_id: AccountId) {
        self.check_agent(env::predecessor_account_id());

        // get the current status of the user
        let current_user_status = self.get_current_user_status(user_id.clone());

        // check if the user is already blacklisted
        require!(
            current_user_status != UserStatus::BLACKLISTED,
            ERR_USER_ALREADY_BLACKLISTED
        );

        // add the user to the blacklist
        self.whitelist
            .users
            .set(user_id.clone(), Some(UserStatus::BLACKLISTED));

        // emit the event
        Event::WhitelistStateChangedEvent {
            account_id: &user_id,
            old_status: current_user_status,
            new_status: UserStatus::BLACKLISTED,
        }
        .emit();
    }

    /// Removes a user's status.
    fn clear_user_status(&mut self, user_id: AccountId) {
        self.check_agent(env::predecessor_account_id());

        // get the current status of the user
        let current_user_status = self.get_current_user_status(user_id.clone());

        // check if the user is in the list
        require!(
            current_user_status != UserStatus::NO_STATUS,
            ERR_USER_STATUS_ALREADY_CLEARED
        );

        // clear user status
        self.whitelist
            .users
            .set(user_id.clone(), Some(UserStatus::NO_STATUS));

        // emit the event
        Event::WhitelistStateChangedEvent {
            account_id: &user_id,
            old_status: current_user_status,
            new_status: UserStatus::NO_STATUS,
        }
        .emit();
    }

    /// Checks if a user is whitelisted.
    fn is_whitelisted(&self, user_id: AccountId) -> bool {
        self.whitelist.users.get(&user_id) == Some(&UserStatus::WHITELISTED)
    }

    /// Checks if a user is blacklisted.
    fn is_blacklisted(&self, user_id: AccountId) -> bool {
        self.whitelist.users.get(&user_id) == Some(&UserStatus::BLACKLISTED)
    }

    /// Checks whether an account is an agent or the owner.
    fn is_agent(&self, agent_id: AccountId) -> bool {
        self.owner_id == agent_id || self.whitelist.agents.contains(&agent_id)
    }

    /// Checks whether an account is an agent or the owner. Fails if its neither.
    fn check_agent(&self, agent_id: AccountId) {
        require!(self.is_agent(agent_id), ERR_CALLER_NOT_AGENT);
    }

    /// Gets the current status of a user.
    fn get_current_user_status(&self, user_id: AccountId) -> UserStatus {
        match self.whitelist.users.get(&user_id) {
            Some(v) => v,
            None => &UserStatus::NO_STATUS,
        }
        .to_owned()
    }
}
