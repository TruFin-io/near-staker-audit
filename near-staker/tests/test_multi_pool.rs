use near_sdk::json_types::{U128, U64};
use near_sdk::test_utils::accounts;
use near_sdk::{Gas, NearToken};
use serde_json::json;

pub mod helpers;
use helpers::*;
mod types;
use types::*;

#[tokio::test]
async fn test_add_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // assert event was emitted
    let logs = result.logs();
    let event_log = logs
        .iter()
        .find(|log| log.starts_with("EVENT_JSON:"))
        .unwrap();
    let event_json: serde_json::Value = serde_json::from_str(&event_log[11..]).unwrap();
    assert_eq!(event_json["event"], "delegation_pool_added_event");
    assert_eq!(event_json["data"][0]["pool_address"], pool.id().to_string());

    Ok(())
}

#[tokio::test]
async fn test_add_pool_with_non_owner_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = pool
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "Only the owner can call this method");

    Ok(())
}

#[tokio::test]
async fn test_add_pool_twice_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Delegation pool already exists");

    Ok(())
}

#[tokio::test]
async fn test_disable_enabled_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // assert event was emitted
    let logs = result.logs();
    let event_log = logs
        .iter()
        .find(|log| log.starts_with("EVENT_JSON:"))
        .unwrap();
    let event_json: serde_json::Value = serde_json::from_str(&event_log[11..]).unwrap();

    assert_eq!(event_json["event"], "delegation_pool_state_changed_event");
    assert_eq!(event_json["data"][0]["pool_address"], pool.id().to_string());
    assert_eq!(event_json["data"][0]["old_state"], "ENABLED");
    assert_eq!(event_json["data"][0]["new_state"], "DISABLED");

    Ok(())
}

#[tokio::test]
async fn test_enabled_enabled_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "enable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Delegation pool already enabled");

    Ok(())
}

#[tokio::test]
async fn test_enable_non_existent_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "enable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Delegation pool does not exist");

    Ok(())
}

#[tokio::test]
async fn test_enable_disabled_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "enable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    // assert event was emitted
    let logs = result.logs();
    let event_log = logs
        .iter()
        .find(|log| log.starts_with("EVENT_JSON:"))
        .unwrap();
    let event_json: serde_json::Value = serde_json::from_str(&event_log[11..]).unwrap();

    assert_eq!(event_json["event"], "delegation_pool_state_changed_event");
    assert_eq!(event_json["data"][0]["pool_address"], pool.id().to_string());
    assert_eq!(event_json["data"][0]["old_state"], "DISABLED");
    assert_eq!(event_json["data"][0]["new_state"], "ENABLED");

    Ok(())
}

#[tokio::test]
async fn test_disable_disabled_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Delegation pool already disabled");

    Ok(())
}

#[tokio::test]
async fn test_disable_non_existent_pool_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Delegation pool does not exist");

    Ok(())
}

#[tokio::test]
async fn test_disable_pool_with_non_owner_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = pool
        .call(contract.id(), "disable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Only the owner can call this method");

    Ok(())
}

#[tokio::test]
async fn test_enable_pool_with_non_owner_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let pool = setup_user(&sandbox, "pool").await?;

    let result = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_success());

    let result = pool
        .call(contract.id(), "enable_pool")
        .args_json(json!({
            "pool_address": pool.id(),
        }))
        .transact()
        .await?;
    assert!(result.is_failure());
    check_error_msg(result, "Only the owner can call this method");

    Ok(())
}

#[tokio::test]
async fn test_get_pools_updated_total_staked() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    // stake some NEAR with the pool contract
    let stake = contract
        .as_account()
        .call(pool.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let pool_2 = setup_pool(&sandbox, &owner, "blob").await?;
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());

    let result = owner.view(contract.id(), "get_pools").await?;
    let pools: Vec<PoolInfo> = result.json()?;
    assert_eq!(pools.len(), 2);

    let pool_1 = pools.iter().find(|p| &p.pool_id == pool.id());
    assert!(pool_1.is_some());
    assert_eq!(pool_1.unwrap().state, ValidatorState::ENABLED);
    assert_eq!(pool_1.unwrap().total_staked, U128(5 * ONE_NEAR));

    let pool_2 = pools.iter().find(|p| &p.pool_id == pool_2.id());
    assert!(pool_2.is_some());
    assert_eq!(pool_2.unwrap().state, ValidatorState::ENABLED);
    assert_eq!(pool_2.unwrap().total_staked, U128(0));

    Ok(())
}

#[tokio::test]
async fn test_update_total_staked() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    // stake some NEAR with the pool contract
    let stake = contract
        .as_account()
        .call(pool.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let pool_2 = setup_pool(&sandbox, &owner, "blob").await?;
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());

    let total_staked_result = contract.view("get_total_staked").await?;

    let total_staked = total_staked_result.json::<(U128, U64)>()?;
    assert_eq!(total_staked.0, U128(ONE_NEAR * 5));
    Ok(())
}

#[tokio::test]
async fn test_update_total_staked_multi_pool() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;

    // stake some NEAR with the pool contract
    let stake = contract
        .as_account()
        .call(pool.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let pool_2 = setup_pool(&sandbox, &owner, "blob").await?;
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": pool_2.id(),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    // stake some NEAR with the new pool contract
    let stake_2 = contract
        .as_account()
        .call(pool_2.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(3))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake_2.is_success());

    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(update_total_staked.is_success());

    let total_staked_result = contract.view("get_total_staked").await?;
    let total_staked = total_staked_result.json::<(U128, U64)>()?;
    assert_eq!(total_staked.0, U128(ONE_NEAR * 8));
    assert!(total_staked.1 .0 != 0);

    Ok(())
}

#[tokio::test]
async fn test_update_total_staked_with_failure() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract, pool) = setup_contract_with_pool().await?;

    // stake 5 NEAR with the pool
    let stake = contract
        .as_account()
        .call(pool.id(), "deposit_and_stake")
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

    // add a second broken pool
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_address": accounts(5),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    // stake 3 more NEAR with the first pool
    let stake = contract
        .as_account()
        .call(pool.id(), "deposit_and_stake")
        .deposit(NearToken::from_near(3))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // try to update total staked for both pools
    let update_total_staked = owner
        .call(contract.id(), "update_total_staked")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify the update_total_staked transaction failed
    assert!(update_total_staked.is_failure());

    // verify if the first pool was not updated
    let result = owner.view(contract.id(), "get_pools").await?;
    let pools: Vec<PoolInfo> = result.json()?;

    let pool_1 = pools.iter().find(|p| &p.pool_id == pool.id());
    assert!(pool_1.is_some());
    assert_eq!(pool_1.unwrap().state, ValidatorState::ENABLED);
    assert_eq!(pool_1.unwrap().total_staked, U128(5 * ONE_NEAR));

    Ok(())
}
