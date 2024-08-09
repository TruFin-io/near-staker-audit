use near_sdk::{json_types::U128, test_utils::accounts};
use serde_json::json;

pub mod constants;
pub mod helpers;
mod types;

use constants::*;
use helpers::*;
use types::*;

#[tokio::test]
async fn test_deallocation_reduces_allocation() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    setup_allocation(&alice, &accounts(4), 4 * ONE_NEAR, contract.id()).await?;

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_success());

    let allocation: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": alice.id(),
        }))
        .await?
        .json()
        .unwrap();

    assert_eq!(allocation.len(), 1);
    assert_eq!(allocation[0].near_amount, (3 * ONE_NEAR).into());
    assert_eq!(
        allocation[0].share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocation[0].share_price_denom, "1");

    // assert event was emitted
    let event_json = get_event(deallocation.logs());

    assert_eq!(event_json["event"], "deallocated_event");
    assert_eq!(event_json["data"][0]["user"], alice.id().to_string());
    assert_eq!(event_json["data"][0]["recipient"], accounts(4).to_string());
    assert_eq!(event_json["data"][0]["amount"], (ONE_NEAR).to_string());
    assert_eq!(
        event_json["data"][0]["total_amount"],
        (3 * ONE_NEAR).to_string()
    );
    assert_eq!(
        event_json["data"][0]["share_price_num"],
        U256::from(SHARE_PRICE_SCALING_FACTOR).to_string()
    );
    assert_eq!(event_json["data"][0]["share_price_denom"], "1");

    Ok(())
}

#[tokio::test]
async fn test_deallocating_full_amount_removes_allocation() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    setup_allocation(&alice, &accounts(4), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance = alice.view_account().await?.balance.as_yoctonear();

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(4 * ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_success());

    // assert storage fee was refunded
    assert!(alice.view_account().await?.balance.as_yoctonear() > pre_balance);

    let allocation: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": alice.id(),
        }))
        .await?
        .json()
        .unwrap();

    assert_eq!(allocation.len(), 0);

    Ok(())
}

#[tokio::test]
async fn test_deallocating_with_no_allocations_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(4 * ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_failure());
    check_error_msg(deallocation, "User has no allocations");

    Ok(())
}

#[tokio::test]
async fn test_deallocating_non_existent_allocation_fails() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    setup_allocation(&alice, &accounts(2), ONE_NEAR, &contract.id()).await?;

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_failure());
    check_error_msg(deallocation, "User has no allocations to this recipient");

    Ok(())
}

#[tokio::test]
async fn test_deallocating_excessive_amount_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    setup_allocation(&alice, &accounts(4), ONE_NEAR, contract.id()).await?;

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(4 * ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_failure());
    check_error_msg(deallocation, "Cannot deallocate more than is allocated");

    Ok(())
}

#[tokio::test]
async fn test_deallocating_to_less_than_one_near_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    setup_allocation(&alice, &accounts(4), ONE_NEAR, contract.id()).await?;

    let deallocation = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR/2),
        }))
        .transact()
        .await?;
    assert!(deallocation.is_failure());
    check_error_msg(deallocation, "Allocated amount must be at least 1 NEAR");

    Ok(())
}

#[tokio::test]
async fn test_deallocate_not_whitelisted_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let result = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "User not whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_deallocate_when_contract_paused_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let pause = owner.call(contract.id(), "pause").transact().await?;
    assert!(pause.is_success());

    let result = alice
        .call(contract.id(), "deallocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Contract is paused");

    Ok(())
}
