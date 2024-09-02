use std::str::FromStr;

use near_sdk::{
    json_types::U128, serde_json::json, test_utils::accounts, AccountId, Gas, NearToken,
};

pub mod constants;
use constants::SHARE_PRICE_SCALING_FACTOR;

pub mod helpers;
use helpers::*;

pub mod event;
use event::*;
use serde_json::Value;
use tokio::try_join;

#[tokio::test]
async fn test_distribute_rewards_in_trunear_when_no_rewards_accrued(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;

    let bob_balance = get_trunear_balance(&contract, &bob).await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let bob_post_balance = get_trunear_balance(&contract, &bob).await?;
    assert_eq!(bob_balance, bob_post_balance);
    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_near_when_no_rewards_accrued(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance = bob.view_account().await?.balance;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    assert_eq!(pre_balance, bob.view_account().await?.balance);

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_trunear() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;

    let bob_balance = get_trunear_balance(&contract, &bob).await?;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), &bob).await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;

    let lhs = (U256::from(4 * ONE_NEAR)) * pre_share_price_denom / (pre_share_price_num / ONE_NEAR);
    let rhs = (U256::from(4 * ONE_NEAR)) * share_price_denom / (share_price_num / ONE_NEAR);
    let trunear_amount = lhs - rhs;

    let bob_post_balance = get_trunear_balance(&contract, &bob).await?;
    let alice_balance = get_trunear_balance(&contract, alice.id()).await?;

    assert!(bob_balance < bob_post_balance);
    assert_eq!(bob_post_balance - bob_balance, trunear_amount.as_u128());

    let event_json = get_event(distribution.logs());

    assert_eq!(event_json["event"], "distributed_rewards_event");
    assert_eq!(event_json["data"][0]["user"], alice.id().to_string());
    assert_eq!(event_json["data"][0]["recipient"], bob.to_string());
    assert_eq!(event_json["data"][0]["shares"], trunear_amount.to_string());
    assert_eq!(
        event_json["data"][0]["near_amount"],
        near_amount.to_string()
    );
    assert_eq!(
        event_json["data"][0]["user_balance"],
        alice_balance.to_string()
    );
    assert_eq!(
        event_json["data"][0]["recipient_balance"],
        bob_post_balance.to_string()
    );
    assert_eq!(event_json["data"][0]["fees"], 0.to_string());
    assert_eq!(event_json["data"][0]["treasury_balance"], 0.to_string());
    assert_eq!(
        event_json["data"][0]["share_price_num"],
        share_price_num.to_string()
    );
    assert_eq!(
        event_json["data"][0]["share_price_denom"],
        share_price_denom.to_string()
    );
    assert_eq!(event_json["data"][0]["in_near"], false);

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_trunear_with_no_trunear_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let alice_balance = get_trunear_balance(&contract, alice.id()).await?;

    let register = alice
        .call(contract.id(), "storage_deposit")
        .args_json(json!(
            {
                "account_id": accounts(1),
                "registration_only": true
            }
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(register.is_success());

    // transfer away all of Alice's balance
    let transfer = alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": accounts(1),
            "amount": U128::from(alice_balance),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    assert!(transfer.is_success());

    let post_alice_balance = get_trunear_balance(&contract, alice.id()).await?;
    assert_eq!(post_alice_balance, 0);

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());

    check_error_msg(distribution, "Insufficient TruNEAR balance");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_near() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance_alice = alice.view_account().await?.balance;
    let pre_balance_bob = bob.view_account().await?.balance;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), bob.id()).await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let post_balance_alice = alice.view_account().await?.balance;
    let post_balance_bob = bob.view_account().await?.balance;

    assert!(post_balance_bob > pre_balance_bob);
    assert_eq!(
        post_balance_bob,
        pre_balance_bob
            .checked_add(NearToken::from_yoctonear(near_amount))
            .unwrap()
    );

    // check that excess amount was refunded
    assert!(pre_balance_alice.checked_sub(post_balance_alice) < Some(NearToken::from_near(1)));

    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;

    let lhs = (U256::from(4 * ONE_NEAR)) * pre_share_price_denom / (pre_share_price_num / ONE_NEAR);
    let rhs = (U256::from(4 * ONE_NEAR)) * share_price_denom / (share_price_num / ONE_NEAR);
    let trunear_amount = lhs - rhs;

    let bob_post_balance = get_trunear_balance(&contract, bob.id()).await?;
    let alice_balance = get_trunear_balance(&contract, alice.id()).await?;

    let event_json = get_event(distribution.logs());

    assert_eq!(event_json["event"], "distributed_rewards_event");
    assert_eq!(event_json["data"][0]["user"], alice.id().to_string());
    assert_eq!(event_json["data"][0]["recipient"], bob.id().to_string());
    assert_eq!(event_json["data"][0]["shares"], trunear_amount.to_string());
    assert_eq!(
        event_json["data"][0]["near_amount"],
        near_amount.to_string()
    );
    assert_eq!(
        event_json["data"][0]["user_balance"],
        alice_balance.to_string()
    );
    assert_eq!(
        event_json["data"][0]["recipient_balance"],
        bob_post_balance.to_string()
    );
    assert_eq!(event_json["data"][0]["fees"], 0.to_string());
    assert_eq!(event_json["data"][0]["treasury_balance"], 0.to_string());
    assert_eq!(
        event_json["data"][0]["share_price_num"],
        share_price_num.to_string()
    );
    assert_eq!(
        event_json["data"][0]["share_price_denom"],
        share_price_denom.to_string()
    );
    assert_eq!(event_json["data"][0]["in_near"], true);

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_near_does_not_alter_tax_exempt_stake(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let set_dist_fee = owner
        .call(contract.id(), "set_distribution_fee")
        .args_json(json!({
            "new_distribution_fee": 1000 //10%
        }))
        .transact()
        .await?;
    assert!(set_dist_fee.is_success());

    let pre_balance_alice = alice.view_account().await?.balance;
    let pre_balance_bob = bob.view_account().await?.balance;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), bob.id()).await?;

    let tax_exempt_stake_pre_distribution = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let post_balance_alice = alice.view_account().await?.balance;
    let post_balance_bob = bob.view_account().await?.balance;

    assert!(post_balance_bob > pre_balance_bob);
    assert_eq!(
        post_balance_bob,
        pre_balance_bob
            .checked_add(NearToken::from_yoctonear(near_amount))
            .unwrap()
    );

    // check that excess amount was refunded
    assert!(pre_balance_alice.checked_sub(post_balance_alice) < Some(NearToken::from_near(1)));

    let tax_exempt_stake = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    assert!(tax_exempt_stake.0 == tax_exempt_stake_pre_distribution.0);

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribution_amount_in_near_gives_correct_amount(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance_bob = bob.view_account().await?.balance;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), bob.id()).await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(near_amount))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let post_balance_bob = bob.view_account().await?.balance;

    assert!(post_balance_bob > pre_balance_bob);
    assert_eq!(
        post_balance_bob,
        pre_balance_bob
            .checked_add(NearToken::from_yoctonear(near_amount))
            .unwrap()
    );

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribution_amount_allocating_less_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), bob.id()).await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(near_amount - 1))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "Attached deposit too small");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_near_with_no_attached_deposit_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance_alice = alice.view_account().await?.balance;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "Attached deposit too small");

    let fees = NearToken::from_millinear(5);
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance_alice.as_yoctonear() - fees.as_yoctonear()
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_refunds_unused_attached_deposit(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;

    let pre_balance_alice = alice.view_account().await?.balance;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": bob.id(),
            "in_near": true,
        }))
        .deposit(NearToken::from_near(5))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let fees = NearToken::from_millinear(5);
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance_alice.as_yoctonear() - fees.as_yoctonear()
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_with_no_allocations_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "User has no allocations");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_not_whitelisted_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "User not whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_contract_paused_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let pausing_contract = owner
        .call(contract.id(), "pause")
        .gas(Gas::from_tgas(5))
        .transact()
        .await?;
    assert!(pausing_contract.is_success());

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "Contract is paused");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_with_no_allocation_to_recipient_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    setup_allocation(&alice, &accounts(1), 4 * ONE_NEAR, contract.id()).await?;
    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "User has no allocations to this recipient");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_gives_fees_to_treasury() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    setup_allocation(&alice, &accounts(4), 4 * ONE_NEAR, contract.id()).await?;

    let set_dist_fee = owner
        .call(contract.id(), "set_distribution_fee")
        .args_json(json!({
            "new_distribution_fee": 1000 //10%
        }))
        .transact()
        .await?;
    assert!(set_dist_fee.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let treasury_balance = get_trunear_balance(&contract, &accounts(1)).await?;
    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    assert!(get_trunear_balance(&contract, &accounts(1)).await? > treasury_balance);

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_in_near_with_no_trunear_if_dist_fee_is_set_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    setup_allocation(&alice, &accounts(4), 4 * ONE_NEAR, contract.id()).await?;

    let set_dist_fee = owner
        .call(contract.id(), "set_distribution_fee")
        .args_json(json!({
            "new_distribution_fee": 1000 //10%
        }))
        .transact()
        .await?;
    assert!(set_dist_fee.is_success());

    let alice_balance = get_trunear_balance(&contract, alice.id()).await?;

    let register = alice
        .call(contract.id(), "storage_deposit")
        .args_json(json!(
            {
                "account_id": accounts(2),
                "registration_only": true
            }
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(register.is_success());

    // transfer away all of Alice's balance
    let transfer = alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": accounts(2),
            "amount": U128::from(alice_balance),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    assert!(transfer.is_success());

    let post_alice_balance = get_trunear_balance(&contract, alice.id()).await?;
    assert_eq!(post_alice_balance, 0);

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let distribution = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "The account doesn't have enough balance");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_trunear_when_no_rewards_accrued(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 2 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 4 * ONE_NEAR, contract.id()).await?;

    let bob_balance = get_trunear_balance(&contract, &bob).await?;
    let charlie_balance = get_trunear_balance(&contract, &charlie).await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let bob_post_balance = get_trunear_balance(&contract, &bob).await?;
    let charlie_post_balance = get_trunear_balance(&contract, &charlie).await?;
    assert_eq!(bob_balance, bob_post_balance);
    assert_eq!(charlie_balance, charlie_post_balance);

    // verify distributed_all_event was emitted
    let events_json = get_events(distribution.logs());
    assert_eq!(events_json.len(), 1);

    let event = find_event(&events_json, |event: &Value| {
        event["event"] == "distributed_all_event"
    })
    .unwrap();

    verify_staker_event(
        event,
        "distributed_all_event",
        vec![DistributedAllEvent {
            user: alice.id().to_string(),
        }],
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_near_when_no_rewards_accrued(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = setup_whitelisted_user(&owner, &contract, "charlie").await?;
    setup_allocation(&alice, bob.id(), 2 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 4 * ONE_NEAR, contract.id()).await?;

    let bob_pre_balance: NearToken = bob.view_account().await?.balance;
    let charlie_pre_balance: NearToken = charlie.view_account().await?.balance;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    assert_eq!(bob.view_account().await?.balance, bob_pre_balance);
    assert_eq!(charlie.view_account().await?.balance, charlie_pre_balance);

    // verify distributed_all_event was emitted
    let events_json = get_events(distribution.logs());
    assert_eq!(events_json.len(), 1);

    let event = find_event(&events_json, |event: &Value| {
        event["event"] == "distributed_all_event"
    })
    .unwrap();

    verify_staker_event(
        event,
        "distributed_all_event",
        vec![DistributedAllEvent {
            user: alice.id().to_string(),
        }],
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_trunear() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 2 * ONE_NEAR, contract.id()).await?;

    let bob_balance = get_trunear_balance(&contract, &bob).await?;
    let charlie_balance = get_trunear_balance(&contract, &charlie).await?;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let bob_near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), &bob).await?;
    let charlie_near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), &charlie).await?;

    let (total_allocated_amount, _, _, _) = get_total_allocated(&contract, alice.id()).await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_success());

    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;

    let bob_trunear_amount = calculate_trunear_distribution_amount(
        4 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    let charlie_trunear_amount = calculate_trunear_distribution_amount(
        2 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    let alice_balance = get_trunear_balance(&contract, alice.id()).await?;
    let bob_post_balance = get_trunear_balance(&contract, &bob).await?;
    let charlie_post_balance = get_trunear_balance(&contract, &charlie).await?;

    assert!(bob_balance < bob_post_balance);
    assert!(charlie_balance < charlie_post_balance);
    assert_eq!(bob_post_balance - bob_balance, bob_trunear_amount);
    assert_eq!(
        charlie_post_balance - charlie_balance,
        charlie_trunear_amount
    );

    let events_json = get_events(distribution.logs());
    assert!(events_json.len() == 5);

    // verify bob distributed_rewards_event
    let bob_distribution_event: Event<DistributedRewardsEvent> =
        find_event(&events_json, |event: &Value| {
            event["event"] == "distributed_rewards_event"
                && event["data"][0]["recipient"] == bob.to_string()
        })
        .unwrap();

    verify_staker_event(
        bob_distribution_event,
        "distributed_rewards_event",
        vec![DistributedRewardsEvent {
            user: alice.id().to_string(),
            recipient: bob.to_string(),
            shares: bob_trunear_amount.to_string(),
            near_amount: bob_near_amount.to_string(),
            user_balance: alice_balance.to_string(),
            recipient_balance: bob_post_balance.to_string(),
            fees: 0.to_string(),
            treasury_balance: 0.to_string(),
            share_price_num: share_price_num.to_string(),
            share_price_denom: share_price_denom.to_string(),
            in_near: false,
            total_allocated_amount: total_allocated_amount.to_string(),
            total_allocated_share_price_num: share_price_num.to_string(),
            total_allocated_share_price_denom: share_price_denom.to_string(),
        }],
    );

    // verify charlie distributed_rewards_event
    let charlie_distribution_event: Event<DistributedRewardsEvent> =
        find_event(&events_json, |event: &Value| {
            event["event"] == "distributed_rewards_event"
                && event["data"][0]["recipient"] == charlie.to_string()
        })
        .unwrap();

    verify_staker_event(
        charlie_distribution_event,
        "distributed_rewards_event",
        vec![DistributedRewardsEvent {
            user: alice.id().to_string(),
            recipient: charlie.to_string(),
            shares: charlie_trunear_amount.to_string(),
            near_amount: charlie_near_amount.to_string(),
            user_balance: (alice_balance + bob_trunear_amount).to_string(),
            recipient_balance: charlie_post_balance.to_string(),
            fees: 0.to_string(),
            treasury_balance: 0.to_string(),
            share_price_num: share_price_num.to_string(),
            share_price_denom: share_price_denom.to_string(),
            in_near: false,
            total_allocated_amount: total_allocated_amount.to_string(),
            total_allocated_share_price_num: share_price_num.to_string(),
            total_allocated_share_price_denom: share_price_denom.to_string(),
        }],
    );

    // verify distributed_all_event was emitted
    let event = find_event(&events_json, |event: &Value| {
        event["event"] == "distributed_all_event"
    })
    .unwrap();

    verify_staker_event(
        event,
        "distributed_all_event",
        vec![DistributedAllEvent {
            user: alice.id().to_string(),
        }],
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_near() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = setup_whitelisted_user(&owner, &contract, "charlie").await?;

    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 2 * ONE_NEAR, contract.id()).await?;

    let alice_pre_near_balance = alice.view_account().await?.balance;
    let bob_pre_near_balance = bob.view_account().await?.balance;
    let charlie_pre_near_balance = charlie.view_account().await?.balance;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let bob_dist_near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), bob.id()).await?;
    let charlie_dist_near_amount =
        calculate_distribute_to_recipient_in_near(&contract, alice.id(), charlie.id()).await?;

    let (total_allocated_amount, _, _, _) = get_total_allocated(&contract, alice.id()).await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .deposit(NearToken::from_near(1))
        .transact()
        .await?;

    assert!(distribution.is_success());

    let alice_post_near_balance = alice.view_account().await?.balance;
    let bob_post_near_balance = bob.view_account().await?.balance;
    let charlie_post_near_balance = charlie.view_account().await?.balance;

    let alice_post_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let bob_post_trunear_balance = get_trunear_balance(&contract, bob.id()).await?;
    let charlie_post_trunear_balance = get_trunear_balance(&contract, charlie.id()).await?;

    // verify that bob received the distribution amount in near
    assert!(bob_post_near_balance > bob_pre_near_balance);
    assert_eq!(
        bob_post_near_balance,
        bob_pre_near_balance
            .checked_add(NearToken::from_yoctonear(bob_dist_near_amount))
            .unwrap()
    );

    // verify that charlie received the distribution amount in near
    assert!(charlie_post_near_balance > charlie_pre_near_balance);
    assert_eq!(
        charlie_post_near_balance,
        charlie_pre_near_balance
            .checked_add(NearToken::from_yoctonear(charlie_dist_near_amount))
            .unwrap()
    );

    // verify that excess amount was refunded
    assert!(
        alice_pre_near_balance
            .checked_sub(alice_post_near_balance)
            .unwrap()
            < NearToken::from_near(1)
    );

    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;

    let bob_dist_trunear_amount = calculate_trunear_distribution_amount(
        4 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );
    let charlie_dist_trunear_amount = calculate_trunear_distribution_amount(
        2 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    let events_json = get_events(distribution.logs());
    assert!(events_json.len() == 3);

    // verify bob distributed_rewards_event was emitted
    let bob_distribution_event: Event<DistributedRewardsEvent> =
        find_event(&events_json, |event: &Value| {
            event["event"] == "distributed_rewards_event"
                && event["data"][0]["recipient"] == bob.id().to_string()
        })
        .unwrap();

    verify_staker_event(
        bob_distribution_event,
        "distributed_rewards_event",
        vec![DistributedRewardsEvent {
            user: alice.id().to_string(),
            recipient: bob.id().to_string(),
            shares: bob_dist_trunear_amount.to_string(),
            near_amount: bob_dist_near_amount.to_string(),
            user_balance: alice_post_trunear_balance.to_string(),
            recipient_balance: bob_post_trunear_balance.to_string(),
            fees: 0.to_string(),
            treasury_balance: 0.to_string(),
            share_price_num: share_price_num.to_string(),
            share_price_denom: share_price_denom.to_string(),
            in_near: true,
            total_allocated_amount: total_allocated_amount.to_string(),
            total_allocated_share_price_num: share_price_num.to_string(),
            total_allocated_share_price_denom: share_price_denom.to_string(),
        }],
    );

    // verify charlie distributed_rewards_event was emitted
    let charlie_distribution_event: Event<DistributedRewardsEvent> =
        find_event(&events_json, |event: &Value| {
            event["event"] == "distributed_rewards_event"
                && event["data"][0]["recipient"] == charlie.id().to_string()
        })
        .unwrap();

    verify_staker_event(
        charlie_distribution_event,
        "distributed_rewards_event",
        vec![DistributedRewardsEvent {
            user: alice.id().to_string(),
            recipient: charlie.id().to_string(),
            shares: charlie_dist_trunear_amount.to_string(),
            near_amount: charlie_dist_near_amount.to_string(),
            user_balance: alice_post_trunear_balance.to_string(),
            recipient_balance: charlie_post_trunear_balance.to_string(),
            fees: 0.to_string(),
            treasury_balance: 0.to_string(),
            share_price_num: share_price_num.to_string(),
            share_price_denom: share_price_denom.to_string(),
            in_near: true,
            total_allocated_amount: total_allocated_amount.to_string(),
            total_allocated_share_price_num: share_price_num.to_string(),
            total_allocated_share_price_denom: share_price_denom.to_string(),
        }],
    );

    // verify distributed_all_event was emitted
    let event = find_event(&events_json, |event: &Value| {
        event["event"] == "distributed_all_event"
    })
    .unwrap();

    verify_staker_event(
        event,
        "distributed_all_event",
        vec![DistributedAllEvent {
            user: alice.id().to_string(),
        }],
    );

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_near_with_exact_distribution_amounts(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    set_distribution_fee(&contract, &owner, 500).await?;
    let treasury = get_treasury_id(&contract).await?;

    // alice allocates to many recipients at different share prices
    let recipients = ["aa", "bb", "cc", "dd", "ee", "ff"];
    for recipient in recipients {
        let account_id = AccountId::from_str(recipient).unwrap();
        setup_allocation(&alice, &account_id, ONE_NEAR, contract.id()).await?;
        let _ =
            move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    }

    // get the required amounts for the distribute_all call
    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, alice.id(), true).await?;

    // transfer excess trunear to bob so alice is left with the exact amount required by the distribution
    let initial_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let excess_trunear = initial_trunear_balance - required_trunear;
    register_account(&contract, &bob, &bob.id()).await?;
    transfer_trunear(&contract, &alice, &bob.id(), excess_trunear).await?;

    // get alice and treasury trunear balannces before the distribute_all call
    let pre_alice_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let pre_treasury_trunear_balance = get_trunear_balance(&contract, &treasury).await?;

    // alice distributes to all recipients in near
    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(required_near))
        .gas(Gas::from_gas(300 * 1_000_000_000_00))
        .transact()
        .await?;

    // verify the distribution was successful
    assert!(distribution.is_success());

    // calculate the total amount of near spent in the distribution
    let mut total_near_spent: u128 = 0;
    let events_json = get_events(distribution.logs());
    for recipient in recipients {
        let distribution_event: Event<DistributedRewardsEvent> =
            find_event(&events_json, |event: &Value| {
                event["event"] == "distributed_rewards_event"
                    && event["data"][0]["recipient"] == recipient.to_string()
            })
            .unwrap();
        let data = distribution_event.data.first().unwrap();
        total_near_spent += data.near_amount.parse::<u128>().unwrap();
    }

    // verify that the total near spent is not greater than the required near amount
    assert!(total_near_spent <= required_near);

    // verify that the total trunear spent is not greater than the required trunear amount
    let alice_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let total_trunear_spent = pre_alice_trunear_balance - alice_trunear_balance;
    assert!(total_trunear_spent <= required_trunear);

    // verify that the treasury received the exact amount of trunear spent
    let treasury_trunear_balance = get_trunear_balance(&contract, &treasury).await?;
    let treasury_received_trunear = treasury_trunear_balance - pre_treasury_trunear_balance;
    assert_eq!(treasury_received_trunear, total_trunear_spent);

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_not_whitelisted_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "User not whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_contract_paused_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let pausing_contract = owner
        .call(contract.id(), "pause")
        .gas(Gas::from_tgas(5))
        .transact()
        .await?;
    assert!(pausing_contract.is_success());

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;
    assert!(distribution.is_failure());
    check_error_msg(distribution, "Contract is paused");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_trunear_with_insufficient_trunear_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = setup_whitelisted_user(&owner, &contract, "charlie").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 2 * ONE_NEAR, contract.id()).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let (required_trunear, _) = calculate_distribute_amounts(&contract, alice.id(), false).await?;

    // transfer away some of alice trunear so it's insufficient for the distribution
    register_account(&contract, &alice, &accounts(1)).await?;
    let alice_truenear_balance: u128 = get_trunear_balance(&contract, alice.id()).await?;
    transfer_trunear(
        &contract,
        &alice,
        &accounts(1),
        alice_truenear_balance - required_trunear / 2,
    )
    .await?;

    let alice_pre_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let bob_pre_trunear_balance = get_trunear_balance(&contract, bob.id()).await?;
    let charlie_pre_trunear_balance = get_trunear_balance(&contract, charlie.id()).await?;

    // distribute all rewards
    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": false,
        }))
        .transact()
        .await?;

    // verify the distribution failed
    assert!(distribution.is_failure());

    // verify users' balances remain unchanged
    let alice_post_trunear_balance = get_trunear_balance(&contract, alice.id()).await?;
    let bob_post_trunear_balance = get_trunear_balance(&contract, bob.id()).await?;
    let charlie_post_trunear_balance = get_trunear_balance(&contract, charlie.id()).await?;

    assert_eq!(alice_post_trunear_balance, alice_pre_trunear_balance);
    assert_eq!(bob_post_trunear_balance, bob_pre_trunear_balance);
    assert_eq!(charlie_post_trunear_balance, charlie_pre_trunear_balance);

    // verify no events were emitted
    let events_json = get_events(distribution.logs());
    assert_eq!(events_json.len(), 0);

    // verify the error message
    check_error_msg(distribution, "Insufficient TruNEAR balance");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_in_near_with_insufficient_attached_near_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = setup_whitelisted_user(&owner, &contract, "charlie").await?;
    setup_allocation(&alice, bob.id(), 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 2 * ONE_NEAR, contract.id()).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let (_, required_near) = calculate_distribute_amounts(&contract, alice.id(), true).await?;

    let alice_pre_trunear_balance = alice.view_account().await?.balance;
    let bob_pre_trunear_balance = bob.view_account().await?.balance;
    let charlie_pre_trunear_balance = charlie.view_account().await?.balance;

    // distribute all rewards attaching less then the required near amount
    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(required_near / 2))
        .transact()
        .await?;

    // verify the distribution failed
    assert!(distribution.is_failure());

    // verify users' balances remain unchanged
    let alice_post_trunear_balance = alice.view_account().await?.balance;
    let bob_post_trunear_balance = bob.view_account().await?.balance;
    let charlie_post_trunear_balance = charlie.view_account().await?.balance;

    let fees = NearToken::from_millinear(5);
    assert_approx_eq!(
        alice_post_trunear_balance.as_yoctonear(),
        alice_pre_trunear_balance.as_yoctonear(),
        fees.as_yoctonear()
    );
    assert_eq!(bob_post_trunear_balance, bob_pre_trunear_balance);
    assert_eq!(charlie_post_trunear_balance, charlie_pre_trunear_balance);

    // verify no events were emitted
    let events_json = get_events(distribution.logs());
    assert_eq!(events_json.len(), 0);

    // verify the error message
    check_error_msg(distribution, "Attached deposit too small");

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribute_amounts_in_trunear() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    // alice allocates to bob and charlie
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 2 * ONE_NEAR, contract.id()).await?;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // get the required trunear amounts for the distribute_all call
    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, alice.id(), false).await?;

    // calculate the expected distribution amounts
    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;
    let bob_trunear_amount = calculate_trunear_distribution_amount(
        4 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );
    let charlie_trunear_amount = calculate_trunear_distribution_amount(
        2 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    // verify the calculated distribution amounts
    assert_eq!(required_near, 0);
    assert_eq!(
        required_trunear,
        bob_trunear_amount + charlie_trunear_amount
    );

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribute_amounts_in_near() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    // alice allocates to bob and charlie
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 2 * ONE_NEAR, contract.id()).await?;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // get the required near amounts for the distribute_all call
    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, alice.id(), true).await?;

    // calculate the expected distribution amounts
    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;
    let bob_trunear_amount = calculate_trunear_distribution_amount(
        4 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );
    let charlie_trunear_amount = calculate_trunear_distribution_amount(
        2 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    let expected_trunear_amount = bob_trunear_amount + charlie_trunear_amount;
    let expected_near_amount = (U256::from(expected_trunear_amount)
        * (share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR))
        / share_price_denom)
        .as_u128();

    // verify the calculated distribution amounts
    assert_eq!(required_trunear, 0);
    assert_eq!(required_near, expected_near_amount);

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribute_amounts_with_no_allocations(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    // alice allocates to bob and charlie
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 2 * ONE_NEAR, contract.id()).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // get the required distribute_all amounts for an account with no allocations
    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, &charlie, true).await?;

    assert_eq!(required_trunear, 0);
    assert_eq!(required_near, 0);

    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, &charlie, true).await?;
    assert_eq!(required_trunear, 0);
    assert_eq!(required_near, 0);

    Ok(())
}

#[tokio::test]
async fn test_calculate_distribute_amounts_with_fees() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    // set the distribution fee to 10%
    set_distribution_fee(&contract, &owner, 1000).await?;

    // alice allocates to bob and charlie
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 2 * ONE_NEAR, contract.id()).await?;

    let (pre_share_price_num, pre_share_price_denom) = share_price_fraction(&contract).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // get the required near amounts for the distribute_all call
    let (required_trunear, required_near) =
        calculate_distribute_amounts(&contract, alice.id(), true).await?;

    // calculate the expected distribution amounts
    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;
    let bob_trunear_amount = calculate_trunear_distribution_amount(
        4 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );
    let charlie_trunear_amount = calculate_trunear_distribution_amount(
        2 * ONE_NEAR,
        pre_share_price_num,
        pre_share_price_denom,
        share_price_num,
        share_price_denom,
    );

    let expected_trunear_amount = bob_trunear_amount + charlie_trunear_amount;
    let expected_near_amount_before_fees = (U256::from(expected_trunear_amount)
        * (share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR))
        / share_price_denom)
        .as_u128();

    // calculate the expected trunear as the distribution fees
    let expected_trunear = expected_trunear_amount / 10;

    // calculate the expected near net of fees
    let expected_near_amount = expected_near_amount_before_fees * 9 / 10;

    // verify the calculated distribution amounts
    assert_eq!(required_trunear, expected_trunear);
    assert_approx_eq!(required_near, expected_near_amount, 1);

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_with_insufficient_gas_does_not_emit_events(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_user(&sandbox, "bob").await?;
    let charlie = setup_user(&sandbox, "charlie").await?;
    let eve = setup_user(&sandbox, "eve").await?;

    setup_allocation(&alice, bob.id(), 10 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 10 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, eve.id(), 10 * ONE_NEAR, contract.id()).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let bob_pre_near_balance = bob.view_account().await?.balance;
    let charlie_pre_near_balance = charlie.view_account().await?.balance;
    let eve_pre_near_balance = eve.view_account().await?.balance;

    let (_, near_amount_to_distribute) =
        calculate_distribute_amounts(&contract, alice.id(), true).await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(near_amount_to_distribute))
        .gas(Gas::from_gas(4671800000000)) // insufficient gas
        .transact()
        .await?;

    // verify the distribution failed because of insufficient gas
    assert!(distribution.is_failure());
    check_error_msg(distribution.clone(), "Exceeded the prepaid gas.");

    let bob_near_balance = bob.view_account().await?.balance;
    let charlie_near_balance = charlie.view_account().await?.balance;
    let eve_near_balance = eve.view_account().await?.balance;

    // verify that no NEAR tokens were distributed
    assert_eq!(bob_near_balance, bob_pre_near_balance);
    assert_eq!(charlie_near_balance, charlie_pre_near_balance);
    assert_eq!(eve_near_balance, eve_pre_near_balance);

    // verify that no event is emitted
    let events_json = get_events(distribution.logs());
    assert!(events_json.is_empty());

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_with_insufficient_gas_does_not_update_the_allocations_price(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_user(&sandbox, "bob").await?;
    let charlie = setup_user(&sandbox, "charlie").await?;
    let eve = setup_user(&sandbox, "eve").await?;

    // set up some allocations at the current share price
    let pre_share_price = get_share_price(contract.clone()).await?;

    setup_allocation(&alice, bob.id(), 10 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, charlie.id(), 10 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, eve.id(), 10 * ONE_NEAR, contract.id()).await?;

    // get the average share price for the allocations before the distribution
    let (_, pre_total_alloc_share_price, _, _) = get_total_allocated(&contract, alice.id()).await?;

    // verify the average allocations share price matches the current share price
    assert_eq!(pre_share_price, pre_total_alloc_share_price);

    // increase the share price
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // verify the share price is now greater than the average allocation share price
    let share_price = get_share_price(contract.clone()).await?;
    assert!(share_price > pre_total_alloc_share_price);

    // call distribute all with insufficient gas to complete the distribution
    let (_, near_amount_to_distribute) =
        calculate_distribute_amounts(&contract, alice.id(), true).await?;

    let distribution = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": true,
        }))
        .deposit(NearToken::from_yoctonear(near_amount_to_distribute))
        .gas(Gas::from_gas(4671800000000))
        .transact()
        .await?;

    // verify the distribution failed because of insufficient gas
    assert!(distribution.is_failure());
    check_error_msg(distribution.clone(), "Exceeded the prepaid gas.");

    // get the current average allocation share price
    let (_, total_alloc_share_price, _, _) = get_total_allocated(&contract, alice.id()).await?;

    // verify that the average share price didn't change
    assert_eq!(total_alloc_share_price, pre_total_alloc_share_price);

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_with_contract_not_in_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    setup_allocation(&alice, &bob, 4 * ONE_NEAR, contract.id()).await?;

    move_epoch_forward(&sandbox, &contract).await?;

    let result = alice
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": accounts(4),
            "in_near": false,
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_distribute_rewards_with_locked_contract_should_fail(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = accounts(4);

    setup_allocation(&bob, &charlie, ONE_NEAR, contract.id()).await?;

    let stake_tx = alice
        .call(contract.id(), "stake")
        .args_json(json!({
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let distribute_tx = bob
        .call(contract.id(), "distribute_rewards")
        .args_json(json!({
            "recipient": charlie,
            "in_near": false,
        }))
        .transact();

    let (stake_tx_result, distribute_tx_result) = try_join!(stake_tx, distribute_tx)?;

    // verify that the distribute_rewards tx failed because the stake tx locked the contract
    assert!(stake_tx_result.is_success());
    assert!(distribute_tx_result.is_failure());
    check_error_msg(distribute_tx_result, "Contract is currently executing");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_with_contract_not_in_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = accounts(4);
    let charlie = accounts(5);
    setup_allocation(&alice, &bob, 2 * ONE_NEAR, contract.id()).await?;
    setup_allocation(&alice, &charlie, 4 * ONE_NEAR, contract.id()).await?;

    move_epoch_forward(&sandbox, &contract).await?;

    let result = alice
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": false,
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_distribute_all_with_locked_contract_should_fail(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;
    let charlie = accounts(4);

    setup_allocation(&bob, &charlie, ONE_NEAR, contract.id()).await?;

    let stake_tx = alice
        .call(contract.id(), "stake")
        .args_json(json!({
            "amount": U128::from(ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let distribute_all_tx = bob
        .call(contract.id(), "distribute_all")
        .args_json(json!({
            "in_near": false,
        }))
        .transact();

    let (stake_tx_result, distribute_all_tx_result) = try_join!(stake_tx, distribute_all_tx)?;

    // verify that the distribute_all tx failed because the stake tx locked the contract
    assert!(stake_tx_result.is_success());
    assert!(distribute_all_tx_result.is_failure());
    check_error_msg(distribute_all_tx_result, "Contract is currently executing");

    Ok(())
}
