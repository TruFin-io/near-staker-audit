use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BurnEvent {
    pub owner_id: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct MintEvent {
    pub owner_id: String,
    pub amount: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TransferEvent {
    pub old_owner_id: String,
    pub new_owner_id: String,
    pub amount: String,
    pub memo: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedRewardsEvent {
    pub user: String,
    pub recipient: String,
    pub shares: String,
    pub near_amount: String,
    pub user_balance: String,
    pub recipient_balance: String,
    pub fees: String,
    pub treasury_balance: String,
    pub share_price_num: String,
    pub share_price_denom: String,
    pub in_near: bool,
    pub total_allocated_amount: String,
    pub total_allocated_share_price_num: String,
    pub total_allocated_share_price_denom: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DistributedAllEvent {
    pub user: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event<T> {
    pub standard: String,
    pub version: String,
    pub event: String,
    pub data: Vec<T>,
}

pub fn find_event<T, F>(events_json: &[Value], filter_fn: F) -> Option<T>
where
    T: DeserializeOwned,
    F: Fn(&Value) -> bool,
{
    // Find the event using the provided filter function
    let event_json = events_json.iter().find(|event| filter_fn(event));
    Option::map_or_else(
        event_json,
        || None,
        |event| {
            // Deserialize the event to the specified type
            match serde_json::from_value::<T>(event.clone()) {
                Ok(parsed_event) => Some(parsed_event),
                Err(e) => {
                    eprintln!("Error deserializing event: {:?}", e);
                    None
                }
            }
        },
    )
}

pub fn verify_nep141_event<T>(event: Event<T>, expected_event_type: &str, expected_data: Vec<T>)
where
    T: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    assert_eq!(event.standard, "nep141");
    assert_eq!(event.version, "1.0.0");
    assert_eq!(event.event, expected_event_type);
    assert_eq!(event.data, expected_data);
}

pub fn verify_staker_event<T>(event: Event<T>, expected_event_type: &str, expected_data: Vec<T>)
where
    T: serde::de::DeserializeOwned + std::fmt::Debug + PartialEq,
{
    assert_eq!(event.standard, "staker");
    assert_eq!(event.version, "1.0.0");
    assert_eq!(event.event, expected_event_type);
    assert_eq!(event.data, expected_data);
}
