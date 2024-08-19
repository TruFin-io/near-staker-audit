use near_sdk::test_utils::accounts;
use near_sdk::{Gas, NearToken};

use constants::*;
use helpers::*;
use types::*;

pub mod constants;
pub mod helpers;
mod types;

#[tokio::test]
async fn test_collect_fees() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    set_fee(&contract, &owner, 100).await?;

    // stake 19 NEAR
    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(19))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    stake.is_success();

    // accrue rewards
    let _ = move_epoch_forward(&sandbox, &contract).await;
    let _ = move_epoch_forward(&sandbox, &contract).await;
    let _ = move_epoch_forward(&sandbox, &contract).await;
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let treasury_balance_pre = get_trunear_balance(&contract, &accounts(1)).await?;

    // calculate expected fees
    let (total_staked, _) = get_total_staked(contract.clone()).await?;
    let tax_exempt_stake = get_tax_exempt_stake(contract.clone()).await?;
    let staker_info = contract
        .view("get_staker_info")
        .await?
        .json::<StakerInfo>()
        .unwrap();
    let fee = staker_info.fee;

    // share price before fee collection
    let (share_price_num, share_price_denom) = share_price_fraction(&contract).await?;
    let share_price = get_share_price(contract.clone()).await?;

    let expected_fees_in_near = (fee as u128) * (total_staked - tax_exempt_stake) / FEE_PRECISION;

    // expected_fees_as_shares = 469861724579980256
    let expected_fees_as_shares = mul_div_with_rounding(
        U256::from(expected_fees_in_near),
        share_price_denom,
        share_price_num / U256::from(SHARE_PRICE_SCALING_FACTOR),
        false,
    );

    let total_supply = get_total_supply(&contract).await?;

    // collect fees
    let fees_collected = alice
        .call(contract.id(), "collect_fees")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(fees_collected.is_success());

    // check share price after fee collection
    let share_price_post_fee_collection = get_share_price(contract.clone()).await?;

    // check for treasury balance increase
    let treasury_balance_post = get_trunear_balance(&contract, &accounts(1)).await?;
    let balance_increase = treasury_balance_post - treasury_balance_pre;
    let new_total_supply = get_total_supply(&contract).await?;

    assert!(share_price_post_fee_collection == share_price);
    assert!(balance_increase == expected_fees_as_shares.as_u128());
    assert!(total_supply + balance_increase == new_total_supply);

    Ok(())
}

#[tokio::test]
async fn test_collect_fees_does_not_mint_if_no_rewards_accrue(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    set_fee(&contract, &owner, 100).await?;

    // stake 19 NEAR
    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(19))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    stake.is_success();

    let treasury_balance_pre = get_trunear_balance(&contract, &accounts(1)).await?;

    // collect fees
    let fees_collected = alice
        .call(contract.id(), "collect_fees")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    fees_collected.logs().iter().for_each(|log| {
        println!("{}", log);
    });
    assert!(fees_collected.is_success());

    // check for treasury balance increase
    let treasury_balance_post = get_trunear_balance(&contract, &accounts(1)).await?;

    assert!(treasury_balance_post == treasury_balance_pre);

    Ok(())
}

#[tokio::test]
async fn test_collect_fees_does_not_mint_shares_if_fee_is_zero(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    // stake 5 NEAR
    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    stake.is_success();

    // accrue rewards
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;
    let _ = move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner.clone()).await;

    let treasury_balance_pre = get_trunear_balance(&contract, &accounts(1)).await?;

    // collect fees
    let fees_collected = alice
        .call(contract.id(), "collect_fees")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(fees_collected.is_success());

    let treasury_balance_post = get_trunear_balance(&contract, &accounts(1)).await?;

    assert!(treasury_balance_post == treasury_balance_pre);
    assert!(treasury_balance_post == 0);

    Ok(())
}

#[tokio::test]
async fn test_collect_fees_if_rewards_accrue_and_initial_stake_unstaked(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    set_fee(&contract, &owner, 50).await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward(&sandbox, &contract).await?;
    move_epoch_forward_and_update_total_staked(&sandbox, &contract, owner).await?;

    // total staked is now 10 NEAR + rewards
    let (total_staked, _) = get_total_staked(contract.clone()).await?;
    let tax_exempt_stake = get_tax_exempt_stake(contract.clone()).await?;
    assert!(total_staked > tax_exempt_stake);

    let _ = unstake(&contract, alice.clone(), 10).await?;

    let (total_staked, _) = get_total_staked(contract.clone()).await?;
    let tax_exempt_stake = get_tax_exempt_stake(contract.clone()).await?;
    assert!(total_staked > tax_exempt_stake);
    assert!(tax_exempt_stake == 0);

    let treasury_balance_pre = get_trunear_balance(&contract, &accounts(1)).await?;
    // collect fees
    let fees_collected = alice
        .call(contract.id(), "collect_fees")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(fees_collected.is_success());

    // check for treasury balance increase
    let treasury_balance_post = get_trunear_balance(&contract, &accounts(1)).await?;

    assert!(treasury_balance_post > treasury_balance_pre);

    Ok(())
}

#[tokio::test]
async fn test_collect_fees_when_contract_not_in_sync_fails(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    // stake 10 NEAR
    let _ = stake(&contract, alice.clone(), 10).await?;

    // move epoch forward but don't update total staked
    move_epoch_forward(&sandbox, &contract).await?;

    // verify that the staker is not in sync
    let (_, contract_epoch) = get_total_staked(contract.clone()).await?;
    let current_epoch = get_current_epoch(&contract).await?;
    assert!(contract_epoch < current_epoch);

    // call collect fees
    let collect_fees_result = alice
        .call(contract.id(), "collect_fees")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    // verify that the collect_fees tx failed
    assert!(collect_fees_result.is_failure());
    check_error_msg(collect_fees_result, "Contract is not in sync");

    Ok(())
}
