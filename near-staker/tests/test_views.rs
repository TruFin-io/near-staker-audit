pub mod helpers;
use helpers::*;
use near_sdk::{
    json_types::{U128, U64},
    test_utils::accounts,
    Gas, NearToken,
};
use serde_json::json;

mod types;
use types::*;

pub mod constants;
use constants::SHARE_PRICE_SCALING_FACTOR;

#[tokio::test]
async fn test_share_price_initial_value() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;

    let share_price = contract
        .view("share_price")
        .args_json(json!({}))
        .await?
        .json::<(String, String)>()
        .unwrap();

    assert_eq!(share_price.0, SHARE_PRICE_SCALING_FACTOR.to_string());
    assert_eq!(share_price.1, "1");

    Ok(())
}

#[tokio::test]
async fn test_share_price_increases_with_rewards() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = stake(&contract, alice.clone(), 10).await?;
    let share_price_first_epoch = get_share_price(contract.clone()).await?;

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let share_price_second_epoch = get_share_price(contract.clone()).await?;
    assert!(share_price_second_epoch > share_price_first_epoch);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let share_price_third_epoch = get_share_price(contract.clone()).await?;
    assert!(share_price_third_epoch > share_price_second_epoch);

    Ok(())
}

#[tokio::test]
async fn test_max_withdraw_initial_value() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;
    let alice = accounts(0);

    let max_withdraw = contract
        .view("max_withdraw")
        .args_json(json!({
            "account_id": alice,
        }))
        .await?
        .json::<U128>()
        .unwrap();

    assert_eq!(max_withdraw.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_max_withdraw_increases_with_rewards() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = stake(&contract, alice.clone(), 10).await?;

    let max_withdraw_first_epoch = get_max_withdraw(contract.clone(), alice.clone()).await?;

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let max_withdraw_second_epoch = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert!(max_withdraw_second_epoch > max_withdraw_first_epoch);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let max_withdraw_third_epoch = get_max_withdraw(contract.clone(), alice.clone()).await?;
    assert!(max_withdraw_third_epoch > max_withdraw_second_epoch);

    Ok(())
}

#[tokio::test]
async fn test_total_staked_initial_value() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;

    let (total_staked, last_updated_at) = contract
        .view("get_total_staked")
        .args_json(json!({}))
        .await?
        .json::<(U128, U64)>()
        .unwrap();

    let current_epoch = get_current_epoch(&contract).await?;
    assert_eq!(total_staked.0, 0);
    assert_eq!(last_updated_at.0, current_epoch);

    Ok(())
}

#[tokio::test]
async fn test_total_staked_increases_with_rewards() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = stake(&contract, alice.clone(), 10).await?;
    let (total_staked_first_epoch, first_epoch) = get_total_staked(contract.clone()).await?;
    assert_eq!(first_epoch, 1);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let (total_staked_second_epoch, second_epoch) = get_total_staked(contract.clone()).await?;
    assert_eq!(second_epoch, 2);
    assert!(total_staked_second_epoch > total_staked_first_epoch);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let (total_staked_third_epoch, third_epoch) = get_total_staked(contract.clone()).await?;
    assert_eq!(third_epoch, 3);
    assert!(total_staked_third_epoch > total_staked_second_epoch);

    Ok(())
}

#[tokio::test]
async fn test_total_staked_and_share_price_increase_with_paused_pool(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, default_pool) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    // pause staking on the default pool
    let pause_pool = owner
        .call(default_pool.clone().id(), "pause_staking")
        .gas(Gas::from_tgas(15))
        .transact()
        .await?;
    assert!(pause_pool.is_success());

    // verify staking is paused
    let is_paused = default_pool
        .view("is_staking_paused")
        .args_json(json!({}))
        .await?
        .json::<bool>()
        .unwrap();
    assert!(is_paused);

    // verify that initial total staked is 0
    let _ = update_total_staked(contract.clone(), owner.clone()).await?;
    let (total_staked, _) = get_total_staked(contract.clone()).await?;
    assert_eq!(total_staked, 0);

    // verify that the initial share price is 1
    let share_price = get_share_price(contract.clone()).await?;
    assert_eq!(share_price, SHARE_PRICE_SCALING_FACTOR);

    // alice stakes 12 NEAR to the paused pool
    let _ = stake(&contract, alice.clone(), 12).await?;
    let (total_staked_epoch_1, _) = get_total_staked(contract.clone()).await?;
    let share_price_epoch_1 = get_share_price(contract.clone()).await?;

    // move epoch forward a few times and verify that total staked and share prices increase
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let (total_staked_epoch_2, _) = get_total_staked(contract.clone()).await?;
    assert!(total_staked_epoch_2 > total_staked_epoch_1);

    let share_price_epoch_2 = get_share_price(contract.clone()).await?;
    assert!(share_price_epoch_2 > share_price_epoch_1);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let (total_staked_epoch_3, _) = get_total_staked(contract.clone()).await?;
    assert!(total_staked_epoch_3 > total_staked_epoch_2);

    let share_price_epoch_3 = get_share_price(contract.clone()).await?;
    assert!(share_price_epoch_3 > share_price_epoch_2);

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let (total_staked_epoch_4, _) = get_total_staked(contract.clone()).await?;
    assert!(total_staked_epoch_4 > total_staked_epoch_3);

    let share_price_epoch_4 = get_share_price(contract.clone()).await?;
    assert!(share_price_epoch_4 > share_price_epoch_3);

    Ok(())
}

#[tokio::test]
async fn test_get_tax_exempt_stake_is_initally_zero() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract, _) = setup_contract_with_pool().await?;

    let tax_exempt_stake = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    assert_eq!(tax_exempt_stake.0, 0);

    Ok(())
}

#[tokio::test]
async fn test_get_tax_exempt_increases_with_stake() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let tax_exempt_stake = contract
        .clone()
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();
    assert_eq!(tax_exempt_stake.0, 0);

    let _: near_workspaces::result::ExecutionFinalResult = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner).await?;

    let tax_exempt_stake = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    assert_eq!(tax_exempt_stake.0, 5 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_get_tax_exempt_does_not_change_as_rewards_accrue(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let _ = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    let tax_exempt_stake = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();
    assert_eq!(tax_exempt_stake.0, 5 * ONE_NEAR);

    // move 4 epochs forward
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner).await?;

    let tax_exempt_stake = contract
        .view("get_tax_exempt_stake")
        .args_json(json!({}))
        .await?
        .json::<U128>()
        .unwrap();

    assert_eq!(tax_exempt_stake.0, 5 * ONE_NEAR);

    Ok(())
}

#[tokio::test]
async fn test_get_staker_info() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, default_pool) = setup_contract_with_pool().await?;

    set_fee(&contract, &owner, 100).await?;
    set_distribution_fee(&contract, &owner, 500).await?;
    set_min_deposit(&contract, &owner, 10).await?;

    let staker_info = contract
        .view("get_staker_info")
        .await?
        .json::<StakerInfo>()
        .unwrap();

    assert_eq!(
        staker_info,
        StakerInfo {
            owner_id: owner.id().clone(),
            treasury_id: accounts(1),
            default_delegation_pool: default_pool.id().clone(),
            fee: 100,
            dist_fee: 500,
            min_deposit: U128(10 * ONE_NEAR),
            is_paused: false,
            current_epoch: U64(1),
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_get_latest_unstake_nonce_is_initially_zero() -> Result<(), Box<dyn std::error::Error>>
{
    let (_, _, contract, _) = setup_contract_with_pool().await?;

    let nonce = get_latest_unstake_nonce(&contract).await?;
    assert_eq!(nonce, 0);

    Ok(())
}

#[tokio::test]
async fn test_get_latest_unstake_nonce_increases_with_unstake(
) -> Result<(), Box<dyn std::error::Error>> {
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

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    let nonce = get_latest_unstake_nonce(&contract).await?;
    assert!(nonce == 1);

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());

    let nonce = get_latest_unstake_nonce(&contract).await?;
    assert!(nonce == 2);

    Ok(())
}

#[tokio::test]
async fn test_is_claimable_returns_false_if_not_enough_time_has_passed(
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

    let _ = unstake(&contract, alice.clone(), 2).await?;

    let nonce = get_latest_unstake_nonce(&contract).await?;

    let claimable = contract
        .view("is_claimable")
        .args_json(json!({
            "unstake_nonce": U128::from(nonce),
        }))
        .await?
        .json::<bool>()
        .unwrap();

    assert!(!claimable);

    Ok(())
}

#[tokio::test]
async fn test_is_claimable() -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = unstake(&contract, alice.clone(), 2).await?;

    let nonce = get_latest_unstake_nonce(&contract).await?;

    // move 4 epochs forward
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;

    // staker says the unstake is claimable
    let claimable = contract
        .view("is_claimable")
        .args_json(json!({
            "unstake_nonce": U128::from(nonce),
        }))
        .await?
        .json::<bool>()
        .unwrap();
    assert!(claimable);

    // assert amount can actually be claimed
    let withdrawal = contract
        .as_account()
        .call(pool.id(), "withdraw")
        .args_json(json!({
            "amount": U128::from(2 * ONE_NEAR),
        }))
        // .deposit(NearToken::from_near(3))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdrawal.is_success());

    Ok(())
}

#[tokio::test]
async fn test_is_claimable_with_invalid_nonce_fails() -> Result<(), Box<dyn std::error::Error>> {
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

    let _ = unstake(&contract, alice.clone(), 2).await?;

    contract
        .view("is_claimable")
        .args_json(json!({
            "unstake_nonce": U128::from(10), // invalid nonce
        }))
        .await
        .expect_err("Invalid nonce");

    Ok(())
}

#[tokio::test]
async fn test_is_claimable_from_disabled_validator() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;
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

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_id": swanky_new_pool.id(),
            "amount": U128::from(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(unstake.is_success());
    let nonce = get_latest_unstake_nonce(&contract).await?;

    let disabled_validator = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_id": pool.id(),
        }))
        .transact()
        .await?;
    assert!(disabled_validator.is_success());

    // move 4 epochs forward
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;

    let claimable = contract
        .view("is_claimable")
        .args_json(json!({
            "unstake_nonce": U128::from(nonce),
        }))
        .await?
        .json::<bool>()
        .unwrap();

    assert!(claimable);

    Ok(())
}

#[tokio::test]
async fn test_get_allocations_fails_with_user_has_no_allocations(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    contract
        .view("get_allocations")
        .args_json(json!({
           "allocator": alice.id(),
        }))
        .await
        .expect_err("User has no allocations");

    Ok(())
}

#[tokio::test]
async fn test_get_pools() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let pool_2 = setup_pool(&sandbox, &owner, "test-pool").await?;
    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner.view(contract.id(), "get_pools").await?;
    let pools: Vec<PoolInfo> = result.json()?;

    assert_eq!(pools.len(), 2);

    let epoch_height = get_current_epoch(&contract).await?;

    let pool_1 = pools.iter().find(|p| &p.pool_id == pool.id());
    assert!(pool_1.is_some());
    assert_eq!(pool_1.unwrap().state, ValidatorState::ENABLED);
    assert_eq!(pool_1.unwrap().total_staked, U128(0));
    assert!(pool_1.unwrap().unstake_available);
    assert_eq!(pool_1.unwrap().next_unstake_epoch, epoch_height.into());

    let pool_2 = pools.iter().find(|p| &p.pool_id == pool_2.id());
    assert!(pool_2.is_some());
    assert_eq!(pool_2.unwrap().state, ValidatorState::ENABLED);
    assert_eq!(pool_2.unwrap().total_staked, U128(0));
    assert!(pool_2.unwrap().unstake_available);
    assert_eq!(pool_2.unwrap().next_unstake_epoch, epoch_height.into());

    Ok(())
}

#[tokio::test]
async fn test_get_pools_with_different_unstake_periods() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let pool_2 = setup_pool(&sandbox, &owner, "test-pool").await?;
    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let pool_3 = setup_pool(&sandbox, &owner, "another-pool").await?;
    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": pool_3.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // disable third pool
    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_id": pool_3.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // stake to default pool and pool_2
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    let _ = stake(&contract, alice.clone(), 10).await?;
    let stake_to_specific_pool = alice
        .call(contract.id(), "stake_to_specific_pool")
        .args_json(json!({
            "pool_id": pool_2.id(),
        }))
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake_to_specific_pool.is_success());

    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    let _ = unstake(&contract, alice.clone(), 5).await?;

    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    let unstake_from_specific_pool = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!({
            "pool_id": pool_2.id(),
            "amount": U128::from(5 * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake_from_specific_pool.is_success());

    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;

    let result = owner.view(contract.id(), "get_pools").await?;
    let pools: Vec<PoolInfo> = result.json()?;

    assert_eq!(pools.len(), 3);

    let epoch_height = get_current_epoch(&contract).await?;

    let pool_1 = pools.iter().find(|p| &p.pool_id == pool.id());
    let pool_2 = pools.iter().find(|p| &p.pool_id == pool_2.id());
    let pool_3 = pools.iter().find(|p| &p.pool_id == pool_3.id());

    assert!(pool_1.is_some());
    assert_eq!(pool_1.unwrap().state, ValidatorState::ENABLED);
    assert!(pool_1.unwrap().total_staked >= U128(5 * ONE_NEAR));
    assert!(pool_1.unwrap().unstake_available);
    assert_eq!(pool_1.unwrap().next_unstake_epoch, epoch_height.into());

    assert!(pool_2.is_some());
    assert_eq!(pool_2.unwrap().state, ValidatorState::ENABLED);
    assert!(pool_2.unwrap().total_staked >= U128(5 * ONE_NEAR));
    assert!(!pool_2.unwrap().unstake_available);
    assert_eq!(
        pool_2.unwrap().next_unstake_epoch,
        (epoch_height + 2).into()
    );

    assert!(pool_3.is_some());
    assert_eq!(pool_3.unwrap().state, ValidatorState::DISABLED);
    assert!(pool_3.unwrap().total_staked < U128(ONE_NEAR));
    assert!(pool_3.unwrap().unstake_available);
    assert_eq!(pool_3.unwrap().next_unstake_epoch, epoch_height.into());

    Ok(())
}
