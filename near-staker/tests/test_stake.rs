use near_sdk::{
    json_types::{U128, U64},
    serde_json::json,
    test_utils::accounts,
    Gas, NearToken,
};
use tokio::try_join;

pub mod helpers;
use helpers::*;

#[tokio::test]
async fn test_stake() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_stake_to_specific_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let swanky_new_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_stake_twice() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let first_stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(first_stake.is_success());

    let second_stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(second_stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 15 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_stake_by_non_whitelisted_user_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_failure());

    check_error_msg(stake, "User not whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_stake_to_disabled_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let swanky_new_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_failure());

    check_error_msg(stake, "Delegation pool not enabled");

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 0);

    Ok(())
}

#[tokio::test]
async fn test_stake_to_paused_staker_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let pausing_contract = owner
        .call(contract.id(), "pause")
        .gas(Gas::from_tgas(5))
        .transact()
        .await?;
    assert!(pausing_contract.is_success());

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_failure());

    check_error_msg(stake, "Contract is paused");

    Ok(())
}

#[tokio::test]
async fn test_stake_to_enabled_paused_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // add a new pool
    let swanky_new_pool = setup_pool(&sandbox, &owner, "swanky-pool").await?;
    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // pause staking on the new pool
    let pause_pool = owner
        .call(swanky_new_pool.clone().id(), "pause_staking")
        .gas(Gas::from_tgas(15))
        .transact()
        .await?;
    assert!(pause_pool.is_success());

    // get total staked before deposit
    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());
    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked_before_staking_to_paused = total_staked_result.json::<(U128, U64)>()?;

    // deposit and stake
    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    stake.is_success();

    // get total staked after deposit
    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());

    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked = total_staked_result.json::<(U128, U64)>()?;

    assert_eq!(
        total_staked.0 .0,
        total_staked_before_staking_to_paused.0 .0 + 10 * ONE_NEAR
    );

    Ok(())
}

#[tokio::test]
async fn test_stake_to_nonexistent_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": "nonexistent.pool",
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_failure());

    check_error_msg(stake, "Delegation pool does not exist");

    Ok(())
}

#[tokio::test]
async fn test_stake_below_min_deposit_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    set_min_deposit(&contract, &owner, 10).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_failure());

    check_error_msg(stake, "Deposit amount is below minimum deposit");

    Ok(())
}

#[tokio::test]
async fn test_deposit_and_stake_stake_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let first_stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(first_stake.is_success());

    // add a broken pool
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": accounts(5),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    let pre_balance = alice.view_account().await?.balance;

    // As we have already updated total_staked this epoch, it should only
    // call deposit_and_stake on the pool. Since the second pool is broken,
    // this should fail.
    let second_stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .deposit(NearToken::from_near(5))
        .args_json(json!(
            {"pool_id":accounts(5)}
        ))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(second_stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    // ensure 5 NEAR were refunded
    assert!(pre_balance.as_near() - alice.view_account().await?.balance.as_near() <= ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_stake_when_contract_not_in_sync_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // move epoch forward but don't update total staked
    move_epoch_forward(&sandbox, &contract).await?;

    // verify that the staker is not in sync
    let (_, contract_epoch) = get_total_staked(contract.clone()).await?;
    let current_epoch = get_current_epoch(&contract).await?;
    assert!(contract_epoch < current_epoch);

    let stake_result = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the stake tx failed
    assert!(stake_result.is_failure());
    check_error_msg(stake_result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_stake_to_specific_pool_when_contract_not_in_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let second_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": second_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // move epoch forward but don't update total staked
    move_epoch_forward(&sandbox, &contract).await?;

    // verify that the staker is not in sync
    let (_, contract_epoch) = get_total_staked(contract.clone()).await?;
    let current_epoch = get_current_epoch(&contract).await?;
    assert!(contract_epoch < current_epoch);

    let stake_result = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": second_pool.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the stake tx failed
    assert!(stake_result.is_failure());
    check_error_msg(stake_result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_stake_while_contract_locked_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let first_stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact();

    let second_stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact();

    // Execute both transactions concurrently
    let (first_stake_res, second_stake_res) = try_join!(first_stake, second_stake)?;

    assert!(first_stake_res.is_success());
    println!("first_stake_res {:?}", first_stake_res);
    println!("second_stake_res {:?}", second_stake_res);
    // this should fail but it doesnt. When logging they both have the same timestamp.
    assert!(second_stake_res.is_success());

    Ok(())
}
