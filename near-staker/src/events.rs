use crate::UserStatus;
use crate::ValidatorState;
use near_sdk::{
    json_types::{U128, U64},
    log,
    serde::Serialize,
    serde_json::json,
    AccountId,
};

const EVENT_STANDARD: &str = "staker";
const EVENT_STANDARD_VERSION: &str = "1.0.0";

#[derive(Serialize, Debug, Clone)]
#[serde(crate = "near_sdk::serde")]
#[serde(tag = "event", content = "data")]
#[serde(rename_all = "snake_case")]
pub enum Event<'a> {
    // Staker events
    StakerInitialisedEvent {
        owner: &'a AccountId,
        treasury: &'a AccountId,
        default_delegation_pool: &'a AccountId,
        fee: &'a u16,
        distribution_fee: &'a u16,
        min_deposit: &'a U128,
    },
    SetTreasuryEvent {
        old_treasury: &'a AccountId,
        new_treasury: &'a AccountId,
    },
    SetDefaultDelegationPoolEvent {
        old_default_delegation_pool: &'a AccountId,
        new_default_delegation_pool: &'a AccountId,
    },
    SetFeeEvent {
        old_fee: &'a u16,
        new_fee: &'a u16,
    },
    SetDistributionFeeEvent {
        old_distribution_fee: &'a u16,
        new_distribution_fee: &'a u16,
    },
    SetMinDepositEvent {
        old_min_deposit: &'a U128,
        new_min_deposit: &'a U128,
    },
    SetPendingOwnerEvent {
        current_owner: &'a AccountId,
        pending_owner: &'a AccountId,
    },
    OwnershipClaimedEvent {
        old_owner: &'a AccountId,
        new_owner: &'a AccountId,
    },
    PausedEvent {},
    UnpausedEvent {},
    DelegationPoolAddedEvent {
        pool_address: &'a AccountId,
    },
    DelegationPoolStateChangedEvent {
        pool_address: &'a AccountId,
        old_state: ValidatorState,
        new_state: ValidatorState,
    },
    DepositedEvent {
        user_id: &'a AccountId,
        amount: &'a U128,
        amount_staked: &'a U128,
        user_balance: &'a U128,
        shares_amount: &'a U128,
        total_staked: &'a U128,
        total_supply: &'a U128,
        share_price_num: &'a String,
        share_price_denom: &'a String,
        epoch: &'a U64,
        pool_id: &'a AccountId,
    },
    UnstakedEvent {
        user_id: &'a AccountId,
        amount: &'a U128,
        user_balance: &'a U128,
        shares_amount: &'a U128,
        total_staked: &'a U128,
        total_supply: &'a U128,
        share_price_num: &'a String,
        share_price_denom: &'a String,
        unstake_nonce: &'a U128,
        epoch: &'a U64,
        pool_id: &'a AccountId,
    },
    AllocatedEvent {
        user: &'a AccountId,
        recipient: &'a AccountId,
        amount: &'a U128,
        total_amount: &'a U128,
        share_price_num: &'a String,
        share_price_denom: &'a String,
        total_allocated_amount: &'a U128,
        total_allocated_share_price_num: &'a String,
        total_allocated_share_price_denom: &'a String,
    },
    DeallocatedEvent {
        user: &'a AccountId,
        recipient: &'a AccountId,
        amount: &'a U128,
        total_amount: &'a U128,
        share_price_num: &'a String,
        share_price_denom: &'a String,
        total_allocated_amount: &'a U128,
        total_allocated_share_price_num: &'a String,
        total_allocated_share_price_denom: &'a String,
    },
    DistributedRewardsEvent {
        user: AccountId,
        recipient: AccountId,
        shares: U128,
        near_amount: U128,
        user_balance: U128,
        recipient_balance: U128,
        fees: U128,
        treasury_balance: U128,
        share_price_num: String,
        share_price_denom: String,
        in_near: bool,
        total_allocated_amount: U128,
        total_allocated_share_price_num: String,
        total_allocated_share_price_denom: String,
    },
    WithdrawalEvent {
        user: &'a AccountId,
        amount: &'a U128,
        unstake_nonce: &'a U128,
        epoch: &'a U64,
        delegation_pool: &'a AccountId,
    },
    FeesCollectedEvent {
        shares_minted: &'a U128,
        treasury_balance: &'a U128,
        share_price_num: &'a String,
        share_price_denom: &'a String,
        epoch: &'a U64,
    },
    DistributedAllEvent {
        user: &'a AccountId,
    },
    // Whitelist events
    AgentAddedEvent {
        account_id: &'a AccountId,
    },
    AgentRemovedEvent {
        account_id: &'a AccountId,
    },
    WhitelistStateChangedEvent {
        account_id: &'a AccountId,
        old_status: UserStatus,
        new_status: UserStatus,
    },
}

impl Event<'_> {
    pub fn emit(&self) {
        emit_event(&self);
    }
}

// Emit event that follows NEP-297 standard: https://nomicon.io/Standards/EventsFormat
// Arguments
// * `standard`: name of standard, e.g. nep171
// * `version`: e.g. 1.0.0
// * `event`: type of the event, e.g. nft_mint
// * `data`: associate event data. Strictly typed for each set {standard, version, event} inside corresponding NEP
pub(crate) fn emit_event<T: ?Sized + Serialize>(data: &T) {
    let result = json!(data);
    let event_json = json!({
        "standard": EVENT_STANDARD,
        "version": EVENT_STANDARD_VERSION,
        "event": result["event"],
        "data": [result["data"]]
    })
    .to_string();
    log!("EVENT_JSON:{}", event_json);
}
