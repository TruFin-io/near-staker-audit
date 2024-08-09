use near_sdk::{
    json_types::{U128, U64},
    serde_json::json,
    Gas, NearToken,
};
use tokio::try_join;

pub mod helpers;
use helpers::*;

#[tokio::test]
async fn test_unstake_partial_amount() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let _ = stake(&contract, alice.clone(), 10).await?;
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    // as rewards have accrued, max_withdraw is more than 8 NEAR
    assert!(max_withdraw > 8 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_unstake_partial_amount_with_total_staked_already_updated(
) -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 8 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_unstake_max_withdraw_amount() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let _ = stake(&contract, alice.clone(), 10).await?;

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;

    // unstake max withdraw amount
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(max_withdraw),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 0);

    Ok(())
}

#[tokio::test]
async fn test_unstake_close_to_max_withdraw_amount_unstakes_entire_amount(
) -> Result<(), Box<dyn std::error::Error>> {
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

    // rewards accrue
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // max withdraw is now 10.000044061348224227030004 NEAR
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert!(max_withdraw > 10 * ONE_NEAR);

    // unstake 10 NEAR
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(10 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    // burnt all tokens
    let alice_post_balance = get_trunear_balance(&contract, alice.clone().id()).await?;
    assert_eq!(alice_post_balance, 0);

    // max withdraw is now 0
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 0);

    Ok(())
}

#[tokio::test]
async fn test_unstake_from_specific_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let swanky_new_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 8 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_unstake_more_than_staked_from_specific_pool() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, sandbox, contract, default_pool) = setup_contract_with_pool().await?;

    let alice_balance = NearToken::from_near(10_000);
    let alice =
        setup_whitelisted_user_with_custom_balance(&owner, &contract, "alice", alice_balance)
            .await?;

    let swanky_new_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_address": default_pool.id(),
        }))
        .deposit(NearToken::from_near(3000))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 3000 * ONE_NEAR);

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
            "amount": U128::from(3000 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_failure());

    check_error_msg(unstake, "Insufficient funds on delegation pool");

    Ok(())
}

#[tokio::test]
async fn test_unstake_twice() -> Result<(), Box<dyn std::error::Error>> {
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

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 6 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_unstake_by_non_whitelisted_user_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // remvoe alice from whitelist
    let _ = owner
        .call(contract.id(), "clear_user_status")
        .args_json(json!({
            "user_id": alice.id(),
        }))
        .transact()
        .await?;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_failure());

    check_error_msg(unstake, "User not whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_unstake_from_disabled_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let swanky_new_pool = setup_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": swanky_new_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 8 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_unstake_more_than_max_withdraw_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(max_withdraw+1),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_failure());

    check_error_msg(unstake, "Invalid unstake amount");

    Ok(())
}

#[tokio::test]
async fn test_unstake_from_paused_staker_fails() -> Result<(), Box<dyn std::error::Error>> {
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

    let pausing_contract = owner
        .call(contract.id(), "pause")
        .gas(Gas::from_tgas(5))
        .transact()
        .await?;
    assert!(pausing_contract.is_success());

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_failure());

    check_error_msg(unstake, "Contract is paused");

    Ok(())
}

#[tokio::test]
async fn test_unstake_with_no_attached_deposit_fails() -> Result<(), Box<dyn std::error::Error>> {
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

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_failure());
    check_error_msg(
        unstake,
        "The attached deposit is less than the storage cost",
    );

    Ok(())
}

#[tokio::test]
async fn test_unstake_refunds_excess_attached_deposit() -> Result<(), Box<dyn std::error::Error>> {
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

    let pre_balance = alice.view_account().await?.balance;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    println!("unstake {:?}", pre_balance.as_yoctonear());
    println!(
        "unstake {:?}",
        alice.view_account().await?.balance.as_yoctonear()
    );
    let fees = NearToken::from_millinear(5);
    let storage_cost: NearToken = contract.view("get_storage_cost").await?.json().unwrap();
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance.as_yoctonear() - fees.as_yoctonear() - storage_cost.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_unstake_from_nonexistent_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
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

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_address": "nonexistent.pool",
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_failure());

    check_error_msg(unstake, "Delegation pool does not exist");

    Ok(())
}

#[tokio::test]
async fn test_unstake_decreases_total_staked_by_unstake_amount(
) -> Result<(), Box<dyn std::error::Error>> {
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

    // total staked is now 10 NEAR
    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked = total_staked_result.json::<(U128, U64)>()?;
    let total_staked_before_unstake = total_staked.0 .0;

    let unstake_amount = 2 * ONE_NEAR;
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(unstake_amount),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    // total staked is now 8 NEAR
    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked = total_staked_result.json::<(U128, U64)>()?;
    let total_staked_after_unstake = total_staked.0 .0;

    assert!((total_staked_before_unstake - unstake_amount) == total_staked_after_unstake);

    Ok(())
}

#[tokio::test]
async fn test_unstake_increases_total_unstaked_by_unstake_amount(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // unstaked balance of pool before unstake
    let unstaked_balance = get_account_unstaked_balance(&pool, contract.id().clone()).await?;
    assert!(unstaked_balance == 0);

    let unstake_amount = 2 * ONE_NEAR;
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(unstake_amount),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    // unstaked balance of pool after unstake
    let unstaked_balance = get_account_unstaked_balance(&pool, contract.id().clone()).await?;
    assert!(unstaked_balance == unstake_amount);

    Ok(())
}

#[tokio::test]
async fn test_unstake_decreases_staked_amount_by_unstake_amount(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // staked balance of pool before unstake
    let staked_balance_before_unstake =
        get_account_staked_balance(&pool, contract.id().clone()).await?;
    assert!(staked_balance_before_unstake == 10 * ONE_NEAR);

    let unstake_amount = 2 * ONE_NEAR;
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(unstake_amount),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    // unstaked balance of pool after unstake
    let staked_balance = get_account_staked_balance(&pool, contract.id().clone()).await?;
    assert!(staked_balance == staked_balance_before_unstake - unstake_amount);

    Ok(())
}

#[tokio::test]
async fn test_unstake_fails_when_not_enough_funds_on_pool() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // stake 5 NEAR with the default pool
    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // update total staked to 5 NEAR
    let _ = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked = total_staked_result.json::<(U128, U64)>()?;

    assert!(total_staked.0 == U128(5 * ONE_NEAR));

    let default_pool_balance = get_account_staked_balance(&pool, contract.id().clone()).await?;
    assert!(default_pool_balance == (5 * ONE_NEAR));

    // add a second broken pool
    let pool_2 = setup_pool(&sandbox, &owner, "test-pool").await?;
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    // epoch passed
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // try to unstake but the update_total_staked will fail
    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "amount": U128::from(5 * ONE_NEAR),
            "pool_address": pool_2.id()
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    // the unstake will succeed, but no money was unstaked
    assert!(unstake.is_failure());
    check_error_msg(unstake, "Insufficient funds on delegation pool");

    Ok(())
}

#[tokio::test]
async fn test_unstake_when_withdraw_ready_withdraws_all() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let _ = stake(&contract, alice.clone(), 10).await?;
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert_eq!(max_withdraw, 10 * ONE_NEAR);

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    for _ in 0..4 {
        let _ =
            move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    }

    let pre_balance = contract.view_account().await?.balance.as_yoctonear();

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    let fees = NearToken::from_millinear(5);
    assert!(
        contract.view_account().await?.balance.as_yoctonear()
            >= pre_balance + 2 * ONE_NEAR - fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_unstake_in_epoch_after_different_unstake_fails(
) -> Result<(), Box<dyn std::error::Error>> {
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

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_failure());
    check_error_msg(unstake, "Unstake is currently locked for this pool");

    Ok(())
}

#[tokio::test]
async fn test_unstake_does_not_unstake_if_get_account_unstaked_balance_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // add a pool that breaks when get_account_unstaked_balance is called
    let pool_2 = setup_breakable_pool(&sandbox, &owner, "test_pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .deposit(NearToken::from_near(10))
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // set flag that will cause get_account_unstaked_balance to fail
    let break_pool = owner
        .call(pool_2.id(), "set_get_unstake_fail")
        .transact()
        .await?;
    assert!(break_pool.is_success());

    // unstake directly from pool to show unstake still works normally
    let unstake_from_pool = contract
        .as_account()
        .call(pool_2.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake_from_pool.is_success());

    // unbreak it to get the pre_unstaked_amount
    let break_pool = owner
        .call(pool_2.id(), "set_get_unstake_fail")
        .transact()
        .await?;
    assert!(break_pool.is_success());

    let pre_unstaked_amount = get_account_unstaked_balance(&pool_2, contract.id().clone()).await?;
    let pre_balance_alice = get_trunear_balance(&contract, &alice.clone().id()).await?;

    // break it again
    let break_pool = owner
        .call(pool_2.id(), "set_get_unstake_fail")
        .transact()
        .await?;
    assert!(break_pool.is_success());

    // call unstake from contract
    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
            "pool_address": pool_2.id()
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    // the unstake will succeed, but no money was unstaked
    assert!(unstake.is_success());
    assert!(unstake
        .logs()
        .contains(&"Failed to unstake: Callback failed"));

    // unbreak
    let break_pool = owner
        .call(pool_2.id(), "set_get_unstake_fail")
        .transact()
        .await?;
    assert!(break_pool.is_success());

    let post_unstaked_amount = get_account_unstaked_balance(&pool_2, contract.id().clone()).await?;
    assert_eq!(pre_unstaked_amount, post_unstaked_amount);

    let post_balance_alice = get_trunear_balance(&contract, alice.clone().id()).await?;
    assert_eq!(pre_balance_alice, post_balance_alice);

    Ok(())
}

#[tokio::test]
async fn test_unstake_when_contract_not_in_sync_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // alice stakes 10 NEAR
    let _ = stake(&contract, alice.clone(), 10).await?;

    // move epoch forward but don't update total staked
    move_epoch_forward(&sandbox, &contract).await?;

    // verify that the staker is not in sync
    let (_, contract_epoch) = get_total_staked(contract.clone()).await?;
    let current_epoch = get_current_epoch(&contract).await?;
    assert!(contract_epoch < current_epoch);

    // alice tries to unstake 2 NEAR
    let unstake_result = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the unstake tx failed
    assert!(unstake_result.is_failure());
    check_error_msg(unstake_result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_unstake_from_specific_pool_when_contract_not_in_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let second_pool = setup_pool(&sandbox, &owner, "test_pool").await?;
    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": second_pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // alice stakes 10 NEAR to second_pool
    let _ = stake_to_specific_pool(&contract, alice.clone(), second_pool.id().clone(), 10).await?;

    // move epoch forward but don't update total staked
    move_epoch_forward(&sandbox, &contract).await?;

    // verify that the staker is not in sync
    let (_, contract_epoch) = get_total_staked(contract.clone()).await?;
    let current_epoch = get_current_epoch(&contract).await?;
    assert!(contract_epoch < current_epoch);

    // alice tries to unstake 2 NEAR from second_pool
    let unstake_result = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
            "pool_address": second_pool.id()
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the unstake tx failed
    assert!(unstake_result.is_failure());
    check_error_msg(unstake_result, "Contract is not in sync");

    Ok(())
}

#[tokio::test]
async fn test_unstake_and_withdraw_simultaneously() -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    for _ in 0..4 {
        move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    }

    // Prepare unstake and withdraw transactions
    let unstake_tx = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let withdraw_tx = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact();

    // Execute both transactions concurrently
    let (unstake_result, withdraw_result) = try_join!(unstake_tx, withdraw_tx)?;

    // as withdraw executes slightly later, it will attempt to withdraw after we withdraw in the unstake and thus fail
    assert!(unstake_result.is_success());
    assert!(withdraw_result.is_failure());

    Ok(())
}

#[tokio::test]
async fn test_withdraw_and_unstake_simultaneously() -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    for _ in 0..4 {
        move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    }

    // Prepare unstake and withdraw transactions
    let unstake_tx = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let withdraw_tx = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact();

    // Execute both transactions concurrently
    let (withdraw_result, unstake_result) = try_join!(withdraw_tx, unstake_tx)?;

    // as unstake executes slightly later, it will attempt to withdraw after the withdraw and therefore fail
    assert!(withdraw_result.is_success());
    assert!(unstake_result.is_success());
    println!("{:?}", unstake_result.logs());
    assert!(unstake_result
        .logs()
        .contains(&"Failed to unstake: Callback failed"));

    Ok(())
}

#[tokio::test]
async fn test_unstake_above_max_withdraw_refunds_excess_attached_deposit(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    // alice stakes 10 NEAR
    let _ = stake(&contract, alice.clone(), 10).await?;

    let pre_balance = alice.view_account().await?.balance;
    let pre_max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    let pre_unstake_balance = get_account_unstaked_balance(&pool, contract.id().clone()).await?;

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    // alice tries to unstake 20 NEAR which is more than max withdraw
    let unstake = alice
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(20 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the unstake call succeeded
    assert!(unstake.is_failure());

    // verify that nothing was unstaked but max withdraw increased
    let max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
    let unstake_balance = get_account_unstaked_balance(&pool, contract.id().clone()).await?;

    assert!(max_withdraw > pre_max_withdraw);
    assert!(pre_unstake_balance == unstake_balance);

    // verify that alice's NEAR deposit is refunded
    let fees = NearToken::from_millinear(5);
    assert!(
        alice.view_account().await?.balance.as_yoctonear()
            > pre_balance.as_yoctonear() - fees.as_yoctonear()
    );

    Ok(())
}
