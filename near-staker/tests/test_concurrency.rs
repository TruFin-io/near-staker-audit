use near_sdk::json_types::U128;
use near_sdk::{Gas, NearToken};
use serde_json::json;
pub mod constants;
pub mod helpers;
use helpers::*;

pub mod event;
mod types;
use tokio::try_join;

#[tokio::test]
async fn test_simultaneous_stake_unstake_yields_constant_total_staked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // alice stakes 10 NEAR
    let alice_stake_amount = 10;
    let stake: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;
    assert!(stake.is_success());

    // bob stakes 10 NEAR
    let stake = bob
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    let (pre_total_staked, _) = get_total_staked(contract.clone()).await?;

    // alice and bob stake and unstake respectively 2 NEAR 4 times, for a total of 8 NEAR
    let unstakes_count = 4;
    for _ in 0..unstakes_count {
        let bob_deposit_tx = bob
            .call(contract.id(), "stake")
            .deposit(NearToken::from_near(2))
            .gas(Gas::from_tgas(300))
            .transact();

        let alice_unstake_tx = alice
            .call(contract.id(), "unstake")
            .args_json(json!({
                "amount": U128::from(2 * ONE_NEAR),
            }))
            .deposit(NearToken::from_near(3))
            .gas(Gas::from_tgas(300))
            .transact();

        let (bob_deposit_result, alice_unstake_result) =
            try_join!(bob_deposit_tx, alice_unstake_tx)?;

        let (total_staked, _) = get_total_staked(contract.clone()).await?;

        if bob_deposit_result.is_failure() {
            assert!(alice_unstake_result.is_success());
            check_error_msg(bob_deposit_result, "Contract is currently executing");
            assert!(pre_total_staked - total_staked >= NearToken::from_near(2).as_yoctonear());
        } else {
            assert!(bob_deposit_result.is_success());
            assert!(alice_unstake_result.is_failure());
            check_error_msg(alice_unstake_result, "Contract is currently executing");
            assert!(total_staked - pre_total_staked >= NearToken::from_near(2).as_yoctonear());
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_simultaneous_stake_unstake_and_update_total_staked_results_in_nondeterministic_total_staked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_whitelisted_user(&owner, &contract, "bob").await?;

    // alice stakes 10 NEAR
    let alice_stake_amount = 10;
    let stake: near_workspaces::result::ExecutionFinalResult =
        stake(&contract, alice.clone(), alice_stake_amount).await?;
    assert!(stake.is_success());

    // bob stakes 10 NEAR
    let stake = bob
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    // alice and bob simultaneously stake and unstake 2 NEAR 4 times, for a total of 8 NEAR
    let unstakes_count = 4;
    let (pre_total_staked, _) = get_total_staked(contract.clone()).await?;

    for _ in 0..unstakes_count {
        let bob_deposit_tx = bob
            .call(contract.id(), "stake")
            .deposit(NearToken::from_near(2))
            .gas(Gas::from_tgas(300))
            .transact();

        let update_total_staked = owner
            .call(contract.id(), "update_total_staked")
            .gas(Gas::from_tgas(300))
            .transact();

        let alice_unstake_tx = alice
            .call(contract.id(), "unstake")
            .args_json(json!({
                "amount": U128::from(2 * ONE_NEAR),
            }))
            .deposit(NearToken::from_near(3))
            .gas(Gas::from_tgas(300))
            .transact();

        let (alice_unstake_result, update_total_staked, bob_deposit_result) =
            try_join!(alice_unstake_tx, update_total_staked, bob_deposit_tx)?;
        assert!(bob_deposit_result.is_failure());
        assert!(alice_unstake_result.is_failure());
        assert!(update_total_staked.is_success());

        let (total_staked, _) = get_total_staked(contract.clone()).await?;

        // depending on the order of the transactions, the total staked amount may be different.
        // the deposit and unstake might exactly cancel each other (minus gas) and result in the same total staked amount
        // however the new total staked amount can be exactly off by the amount tested e.g. +/- 2 NEAR in this case.
        if pre_total_staked > total_staked {
            assert!(pre_total_staked - total_staked < NearToken::from_near(2).as_yoctonear());
        } else {
            assert!(total_staked - pre_total_staked < NearToken::from_near(2).as_yoctonear());
        }
    }

    // depending on the non-deterministic order of the transactions, the share price may or may not change.

    Ok(())
}
