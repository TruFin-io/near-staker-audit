use near_sdk::{json_types::U128, serde_json::json, test_utils::accounts, NearToken};
pub mod constants;
pub mod helpers;
mod types;

use constants::*;
use helpers::*;
use types::*;

#[tokio::test]
async fn test_first_allocation() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_success());

    let allocation: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": alice.id(),
        }))
        .await?
        .json()
        .unwrap();

    assert_eq!(allocation.len(), 1);
    assert_eq!(allocation[0].near_amount, ONE_NEAR.into());
    assert_eq!(
        allocation[0].share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocation[0].share_price_denom, "1");

    // assert event was emitted
    let event_json = get_event(result.logs());

    assert_eq!(event_json["event"], "allocated_event");
    assert_eq!(event_json["data"][0]["user"], alice.id().to_string());
    assert_eq!(event_json["data"][0]["recipient"], accounts(4).to_string());
    assert_eq!(event_json["data"][0]["amount"], (ONE_NEAR).to_string());
    assert_eq!(
        event_json["data"][0]["total_amount"],
        (ONE_NEAR).to_string()
    );
    assert_eq!(
        event_json["data"][0]["share_price_num"],
        U256::from(SHARE_PRICE_SCALING_FACTOR).to_string()
    );
    assert_eq!(event_json["data"][0]["share_price_denom"], "1");

    Ok(())
}

#[tokio::test]
async fn test_multiple_recipients() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(5),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_success());

    let allocations: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": alice.id(),
        }))
        .await?
        .json()
        .unwrap();

    let allocation_1 = allocations
        .iter()
        .find(|a| a.recipient == accounts(4))
        .unwrap();
    assert_eq!(allocation_1.near_amount, (2 * ONE_NEAR).into());
    assert_eq!(
        allocation_1.share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocation_1.share_price_denom, "1");

    let allocation_2 = allocations
        .iter()
        .find(|a| a.recipient == accounts(5))
        .unwrap();
    assert_eq!(allocation_2.near_amount, ONE_NEAR.into());
    assert_eq!(
        allocation_2.share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocation_2.share_price_denom, "1");

    Ok(())
}

#[tokio::test]
async fn test_multiple_allocations() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = bob
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(5),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_success());

    let allocations_alice: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": alice.id(),
        }))
        .await?
        .json()
        .unwrap();

    assert_eq!(allocations_alice.len(), 1);
    assert_eq!(allocations_alice[0].near_amount, (ONE_NEAR).into());
    assert_eq!(
        allocations_alice[0].share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocations_alice[0].share_price_denom, "1");

    let allocations_bob: Vec<AllocationInfo> = contract
        .view("get_allocations")
        .args_json(json!({
            "allocator": bob.id(),
        }))
        .await?
        .json()
        .unwrap();

    assert_eq!(allocations_bob.len(), 1);
    assert_eq!(allocations_bob[0].near_amount, (ONE_NEAR).into());
    assert_eq!(
        allocations_bob[0].share_price_num,
        SHARE_PRICE_SCALING_FACTOR.to_string()
    );
    assert_eq!(allocations_bob[0].share_price_denom, "1");

    Ok(())
}

#[tokio::test]
async fn test_allocate_to_same_person_twice_same_share_price(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let allocate_1 = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(allocate_1.is_success());

    // no deposit is necessary since we are just updating the allocation
    let allocate_2 = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(allocate_2.is_success());

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
        U256::from_dec_str(&allocation[0].share_price_num).unwrap()
            / U256::from_dec_str(&allocation[0].share_price_denom).unwrap(),
        U256::from(SHARE_PRICE_SCALING_FACTOR / 1)
    );

    Ok(())
}

#[tokio::test]
async fn test_allocate_to_self_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": alice.id(),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Cannot allocate to this recipient");

    Ok(())
}

#[tokio::test]
async fn test_allocate_below_one_near_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR/2),
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Allocated amount must be at least 1 NEAR");

    Ok(())
}

#[tokio::test]
async fn test_allocate_not_whitelisted_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
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
async fn test_allocate_with_paused_contract_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let result = owner
        .call(contract.id(), "pause")
        .args_json(json!({}))
        .transact()
        .await?;
    assert!(result.is_success());

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
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

#[tokio::test]
async fn test_allocate_with_no_deposit_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "The attached deposit is less than the storage cost");
    Ok(())
}

#[tokio::test]
async fn test_allocate_refunds_excess_deposit() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let pre_balance = alice.view_account().await?.balance;

    let result = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(4))
        .transact()
        .await?;
    assert!(result.is_success());

    let fees = NearToken::from_millinear(2);
    let storage_cost: NearToken = contract.view("get_storage_cost").await?.json().unwrap();
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance.as_yoctonear() - fees.as_yoctonear() - storage_cost.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_allocate_refunds_full_deposit() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let first_allocation = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(4))
        .transact()
        .await?;
    assert!(first_allocation.is_success());

    let pre_balance = alice.view_account().await?.balance;

    // allocate again to same recipient to ensure full attached deposit is returned
    let second_allocation = alice
        .call(contract.id(), "allocate")
        .args_json(json!({
            "recipient": accounts(4),
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(4))
        .transact()
        .await?;
    assert!(second_allocation.is_success());

    let fees = NearToken::from_millinear(2);
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance.as_yoctonear() - fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_total_allocated_for_user_with_one_allocation(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    // increaase share price
    let _ = increase_total_staked(&contract, &owner, "user_name", 100).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let share_price = get_share_price(contract.clone()).await?;
    assert!(share_price > 1 * SHARE_PRICE_SCALING_FACTOR);

    // alice allocates at current share price
    let allocation_amount = 123 * ONE_NEAR;
    setup_allocation(&alice, &accounts(4), allocation_amount, contract.id()).await?;

    // get the total allocated amount and share price
    let (total_alloc_amount, total_alloc_share_price, _, _) =
        get_total_allocated(&contract, alice.id()).await?;

    // verify alice's total allocated amount and share price
    assert_eq!(total_alloc_amount, allocation_amount);
    assert_eq!(total_alloc_share_price, share_price);

    Ok(())
}

#[tokio::test]
async fn test_total_allocated_for_user_with_no_allocations(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    // get the total allocated amount and share price
    let (total_alloc_amount, total_alloc_share_price, _, _) =
        get_total_allocated(&contract, alice.id()).await?;

    // verify alice has no allocations
    assert_eq!(total_alloc_amount, 0);
    assert_eq!(total_alloc_share_price, 0);

    Ok(())
}

#[tokio::test]
async fn test_total_allocated_for_user_with_many_allocation_at_same_price(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = increase_total_staked(&contract, &owner, "user_name", 100).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let share_price = get_share_price(contract.clone()).await?;
    assert!(share_price > 1 * SHARE_PRICE_SCALING_FACTOR);

    let first_allocation_amount = 123 * ONE_NEAR;
    setup_allocation(&alice, &accounts(3), first_allocation_amount, contract.id()).await?;

    let second_allocation_amount: u128 = 456 * ONE_NEAR;
    setup_allocation(
        &alice,
        &accounts(4),
        second_allocation_amount,
        contract.id(),
    )
    .await?;

    // get the total allocated amount and share price
    let (total_alloc_amount, total_alloc_share_price, _, _) =
        get_total_allocated(&contract, alice.id()).await?;

    // verify alice's total allocation amount and share price
    assert_eq!(
        total_alloc_amount,
        first_allocation_amount + second_allocation_amount
    );
    assert_eq!(total_alloc_share_price, share_price);

    Ok(())
}

#[tokio::test]
async fn test_total_allocated_for_user_with_many_allocations_at_different_prices(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = increase_total_staked(&contract, &owner, "user_name", 100).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // a first allocation at a certain share price
    let share_price_first_alloc = get_share_price(contract.clone()).await?;
    assert!(share_price_first_alloc > 1);
    let first_allocation_amount = 200 * ONE_NEAR;
    setup_allocation(&alice, &accounts(3), first_allocation_amount, contract.id()).await?;

    // increment epoch to increase the share price
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // a second allocation at the new share price
    let share_price_second_alloc = get_share_price(contract.clone()).await?;
    assert!(share_price_second_alloc > share_price_first_alloc);
    let second_allocation_amount = 100 * ONE_NEAR;
    setup_allocation(
        &alice,
        &accounts(4),
        second_allocation_amount,
        contract.id(),
    )
    .await?;

    // get the total allocated amount and share price
    let (total_alloc_amount, total_alloc_share_price, _, _) =
        get_total_allocated(&contract, alice.id()).await?;

    // calculate the expected total allocated amount and share price
    let expected_new_num = first_allocation_amount + second_allocation_amount;
    let expected_new_denom_summand1 = U256::from(first_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(share_price_first_alloc);
    let expected_new_denom_summand2 = U256::from(second_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(share_price_second_alloc);
    let expected_new_denom = expected_new_denom_summand1 + expected_new_denom_summand2;
    let expected_share_price =
        U256::from(expected_new_num) * U256::from(SHARE_PRICE_SCALING_FACTOR) / expected_new_denom;

    // verify the total allocated amount and share price
    assert_eq!(
        total_alloc_amount,
        first_allocation_amount + second_allocation_amount
    );
    assert_approx_eq!(total_alloc_share_price, expected_share_price.as_u128(), 1);

    Ok(())
}

#[tokio::test]
async fn test_total_allocated_for_many_users_with_many_allocations_at_different_times(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    let _ = increase_total_staked(&contract, &owner, "user_name", 100).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // a first allocation at a certain share price
    let alice_first_share_price = get_share_price(contract.clone()).await?;
    assert!(alice_first_share_price > 1);

    // the amount of the first and second allocation
    let first_allocation_amount = 200 * ONE_NEAR;
    let second_allocation_amount = 100 * ONE_NEAR;

    // alice allocates twice at different prices
    setup_allocation(&alice, &accounts(3), first_allocation_amount, contract.id()).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let alice_second_share_price = get_share_price(contract.clone()).await?;
    setup_allocation(
        &alice,
        &accounts(4),
        second_allocation_amount,
        contract.id(),
    )
    .await?;

    // increment epoch to increase the share price
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // a second allocation at the new share price
    let bob_first_share_price = get_share_price(contract.clone()).await?;
    assert!(bob_first_share_price > alice_second_share_price);

    // bob allocates twice at different prices
    setup_allocation(&bob, &accounts(3), first_allocation_amount, contract.id()).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let bob_second_share_price = get_share_price(contract.clone()).await?;
    setup_allocation(&bob, &accounts(5), second_allocation_amount, contract.id()).await?;

    // get the total allocated amount and share price
    let (alice_total_alloc_amount, alice_total_alloc_share_price, _, _) =
        get_total_allocated(&contract, alice.id()).await?;

    // calculate the expected total allocated for alice
    let expected_new_num = first_allocation_amount + second_allocation_amount;
    let expected_new_denom_summand1 = U256::from(first_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(alice_first_share_price);
    let expected_new_denom_summand2 = U256::from(second_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(alice_second_share_price);
    let expected_new_denom = expected_new_denom_summand1 + expected_new_denom_summand2;
    let expected_share_price =
        U256::from(expected_new_num) * U256::from(SHARE_PRICE_SCALING_FACTOR) / expected_new_denom;

    // verify the total allocated amount and share price for alice
    assert_eq!(
        alice_total_alloc_amount,
        first_allocation_amount + second_allocation_amount
    );
    assert_approx_eq!(
        alice_total_alloc_share_price,
        expected_share_price.as_u128(),
        1
    );

    // calculate the expected total allocated for bob
    let (bob_total_alloc_amount, bob_total_alloc_share_price, _, _) =
        get_total_allocated(&contract, bob.id()).await?;
    let expected_new_denom_summand1 = U256::from(first_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(bob_first_share_price);
    let expected_new_denom_summand2 = U256::from(second_allocation_amount)
        * U256::from(SHARE_PRICE_SCALING_FACTOR)
        / U256::from(bob_second_share_price);
    let expected_new_denom = expected_new_denom_summand1 + expected_new_denom_summand2;
    let expected_share_price =
        U256::from(expected_new_num) * U256::from(SHARE_PRICE_SCALING_FACTOR) / expected_new_denom;

    // verify the total allocated amount and share price for bob
    assert_eq!(
        bob_total_alloc_amount,
        first_allocation_amount + second_allocation_amount
    );
    assert_approx_eq!(
        bob_total_alloc_share_price,
        expected_share_price.as_u128(),
        1
    );

    Ok(())
}
