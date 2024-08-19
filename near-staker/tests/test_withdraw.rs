use near_sdk::{json_types::U128, serde_json::json, Gas, NearToken};

pub mod helpers;
use helpers::*;

#[tokio::test]
async fn test_withdraw() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = stake(&contract, alice.clone(), 10).await?;
    assert!(stake.is_success());

    let unstake = unstake(&contract, alice.clone(), 2).await?;
    assert!(unstake.is_success());

    for _ in 0..4 {
        move_epoch_forward(&sandbox, &contract).await?;
    }

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_success());

    let fees = NearToken::from_millinear(5);

    // storage costs should be refunded during the withdraw
    let storage_cost: U128 = contract.view("get_storage_cost").await?.json().unwrap();

    assert!(
        alice.view_account().await?.balance.as_yoctonear() - pre_balance.as_yoctonear()
            >= 2 * ONE_NEAR + storage_cost.0 - fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_with_pre_unstake() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let _ = stake(&contract, alice.clone(), 10).await?;

    let _ = unstake(&contract, alice.clone(), 2).await?;

    for _ in 0..4 {
        move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await?;
    }
    // unstake again to perform a withdraw
    let _ = unstake(&contract, alice.clone(), 2).await?;

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_success());

    let fees = NearToken::from_millinear(5);
    // storage costs should be refunded during the withdraw
    let storage_cost: U128 = contract.view("get_storage_cost").await?.json().unwrap();
    assert!(
        alice.view_account().await?.balance.as_yoctonear() - pre_balance.as_yoctonear()
            >= 2 * ONE_NEAR + storage_cost.0 - fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_no_money_is_transferred_if_withdraw_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    // add a second pool
    let second_pool = setup_pool(&sandbox, &owner, "blob").await?;
    let add_pool = owner
        .call(contract.id(), "add_pool")
        .args_json(json!({
            "pool_id": second_pool.id(),
        }))
        .transact()
        .await?;
    assert!(add_pool.is_success());

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = alice
        .call(contract.id(), "stake_to_specific_pool")
        .deposit(NearToken::from_near(5))
        .args_json(json!({
            "pool_id": second_pool.id()
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let unstake = alice
        .call(contract.id(), "unstake_from_specific_pool")
        .args_json(json!(
            {"amount": U128::from(2 * ONE_NEAR),
            "pool_id": second_pool.id()}
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(unstake.is_success());

    for _ in 0..4 {
        move_epoch_forward(&sandbox, &contract).await?;
    }

    // break the second pool
    sandbox
        .patch_state(second_pool.id(), b"STATE".as_slice(), b"".as_slice())
        .await?;

    let pre_balance = alice.view_account().await?.balance;
    let pre_staker_balance = contract.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    // The withdraw will succeed, but nothing will be withdrawn
    assert!(withdraw.is_success());

    let fees = NearToken::from_millinear(5);
    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            <= fees.as_yoctonear()
    );
    assert!(
        contract.view_account().await?.balance.as_yoctonear() - pre_staker_balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_while_unstake_not_ready_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = stake(&contract, alice.clone(), 10).await?;
    assert!(stake.is_success());

    let unstake = unstake(&contract, alice.clone(), 2).await?;
    assert!(unstake.is_success());

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "Withdraw not ready");

    let fees = NearToken::from_millinear(5);
    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_with_incorrect_nonce_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = stake(&contract, alice.clone(), 10).await?;
    assert!(stake.is_success());

    let unstake = unstake(&contract, alice.clone(), 2).await?;
    assert!(unstake.is_success());

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(0),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "Invalid nonce");

    let fees = NearToken::from_millinear(5);
    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_with_incorrect_user_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = stake(&contract, alice.clone(), 10).await?;
    assert!(stake.is_success());

    let unstake = unstake(&contract, alice.clone(), 2).await?;
    assert!(unstake.is_success());

    let bob = setup_user_with_tokens(&sandbox, "bob", 50).await?;
    whitelist_user(&contract, &owner, &bob).await?;
    let pre_balance = bob.view_account().await?.balance;

    let withdraw = bob
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "Sender must have requested the unlock");

    let fees = NearToken::from_millinear(5);
    assert!(
        pre_balance.as_yoctonear() - bob.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_twice_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let stake = stake(&contract, alice.clone(), 10).await?;
    assert!(stake.is_success());

    let unstake = unstake(&contract, alice.clone(), 2).await?;
    assert!(unstake.is_success());

    for _ in 0..4 {
        move_epoch_forward(&sandbox, &contract).await?;
    }

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_success());

    let fees = NearToken::from_millinear(5);
    assert!(
        alice.view_account().await?.balance.as_yoctonear() - pre_balance.as_yoctonear()
            >= 2 * ONE_NEAR - fees.as_yoctonear()
    );

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "Invalid nonce");

    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_user_not_whitelisted_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "User not whitelisted");

    let fees = NearToken::from_millinear(5);
    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_while_contract_paused_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let pausing_contract = owner
        .call(contract.id(), "pause")
        .gas(Gas::from_tgas(5))
        .transact()
        .await?;
    assert!(pausing_contract.is_success());

    let pre_balance = alice.view_account().await?.balance;

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_failure());
    check_error_msg(withdraw, "Contract is paused");

    let fees = NearToken::from_millinear(5);

    assert!(
        pre_balance.as_yoctonear() - alice.view_account().await?.balance.as_yoctonear()
            < fees.as_yoctonear()
    );
    Ok(())
}

#[tokio::test]
async fn test_withdraw_withdraws_all() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;

    let _ = stake(&contract, alice.clone(), 10).await?;

    let _ = unstake(&contract, alice.clone(), 2).await?;

    let bob = setup_user_with_tokens(&sandbox, "bob", 50).await?;
    whitelist_user(&contract, &owner, &bob).await?;

    let _ = stake(&contract, bob.clone(), 10).await?;

    let _ = unstake(&contract, bob.clone(), 4).await?;

    for _ in 0..4 {
        move_epoch_forward(&sandbox, &contract).await?;
    }

    let pre_balance_alice = alice.view_account().await?.balance.as_yoctonear();
    let pre_balance_staker = contract.view_account().await?.balance.as_yoctonear();

    let withdraw = alice
        .call(contract.id(), "withdraw")
        .args_json(json!({
            "unstake_nonce": U128::from(1),
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(withdraw.is_success());

    let fees = NearToken::from_millinear(5);

    assert!(
        contract.view_account().await?.balance.as_yoctonear()
            >= pre_balance_staker + 4 * ONE_NEAR - fees.as_yoctonear()
    );

    // storage costs should be refunded during the withdraw
    let storage_cost: U128 = contract.view("get_storage_cost").await?.json().unwrap();
    assert!(
        alice.view_account().await?.balance.as_yoctonear() - pre_balance_alice
            >= 2 * ONE_NEAR + storage_cost.0 - fees.as_yoctonear()
    );
    Ok(())
}
