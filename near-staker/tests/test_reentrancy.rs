use near_sdk::{json_types::U128, Gas, NearToken};
use serde_json::json;
pub mod constants;
pub mod helpers;
use helpers::*;

pub mod event;
mod types;
use tokio::try_join;

#[tokio::test]
async fn test_stake_and_withdraw_reentrancy() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // alice stakes 10 NEAR
    let alice_stake_amount = 10;
    let stake: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner).await;

    let pre_share_price = get_share_price(contract.clone()).await?;
    let staker_pre_near_balance: NearToken = contract.view_account().await?.balance;

    // alice unstakes 2 NEAR 4 times, for a total of 8 NEAR
    let unstakes_count = 4;
    let alice_unstake_amount = 8;
    for _ in 0..unstakes_count {
        let unstake = unstake(
            &contract,
            alice.clone(),
            alice_unstake_amount / unstakes_count,
        )
        .await?;
        assert!(unstake.is_success());
    }

    // advance 4 epochs to allow the unstakes to be withdrawn
    for _ in 0..4 {
        let _ = move_epoch_forward(&sandbox, &contract).await;
    }

    // repeat the stake + withdraw transactions 4 times to test reentrancy
    // in different epochs
    for iteration in 0..unstakes_count {
        let bob_stake_tx = bob
            .call(contract.id(), "stake")
            .deposit(NearToken::from_near(1))
            .gas(Gas::from_tgas(300))
            .transact();

        let alice_withdraw_tx = alice
            .call(contract.id(), "withdraw")
            .args_json(json!({
                "unstake_nonce": (iteration + 1).to_string(),
            }))
            .gas(Gas::from_tgas(300))
            .transact();

        let (bob_stake_result, alice_withdraw_result) = try_join!(bob_stake_tx, alice_withdraw_tx)?;

        // verify stake transaction failed
        assert!(bob_stake_result.is_failure());
        check_error_msg(bob_stake_result, "Contract is not in sync");

        // verify withdraw transaction succeeded
        assert!(alice_withdraw_result.is_success());

        // verify bob max_withdraw is 0, since the stake failed
        let bob_max_withdraw = get_max_withdraw(contract.clone(), bob.clone()).await?;
        assert_eq!(bob_max_withdraw, 0);

        // verify the share price is valid
        let share_price = get_share_price(contract.clone()).await?;
        assert!(share_price >= pre_share_price);

        // verify alice max_withdraw is at least the amount that was not unstaked (2 NEAR)
        let alice_max_withdraw = get_max_withdraw(contract.clone(), alice.clone()).await?;
        assert!(alice_max_withdraw > (alice_stake_amount - alice_unstake_amount) * ONE_NEAR);

        // verify that the staker's NEAR balance did not decrease
        let staker_near_balance: NearToken = contract.view_account().await?.balance;
        assert!(staker_near_balance >= staker_pre_near_balance);

        let _ = move_epoch_forward(&sandbox, &contract).await;
    }

    Ok(())
}

#[tokio::test]
async fn test_unstake_multiple_times() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, pool) = setup_contract_with_pool().await?;
    let alice = setup_user_with_tokens(&sandbox, "alice", 50).await?;
    whitelist_user(&contract, &owner, &alice).await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // alice stakes 100 NEAR
    let alice_stake_amount = 40;
    let _: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;

    // bob stakes 10 NEAR
    let bob_stake_amount = 10;
    let _: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, bob.clone(), bob_stake_amount).await?;

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner).await;

    let unstake_tx = bob
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(bob_stake_amount * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let second_unstake_tx = bob
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(bob_stake_amount * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();

    let third_unstake_tx = bob
        .call(contract.id(), "unstake")
        .args_json(json!({
            "amount": U128::from(bob_stake_amount * ONE_NEAR),
        }))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact();
    let (first_unstake, second_unstake, third_unstake) =
        try_join!(unstake_tx, second_unstake_tx, third_unstake_tx)?;

    assert!(first_unstake.is_success());
    assert!(second_unstake.is_failure());
    assert!(third_unstake.is_failure());

    let post_unstaked_balance = get_account_unstaked_balance(&pool, contract.id().clone()).await?;
    assert!(post_unstaked_balance <= bob_stake_amount * ONE_NEAR + ONE_NEAR);
    Ok(())
}
