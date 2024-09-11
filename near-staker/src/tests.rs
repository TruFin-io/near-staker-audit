use super::*;
use crate::math::*;
use near_sdk::test_utils::{accounts, get_logs, VMContextBuilder};
use near_sdk::{testing_env, AccountId};
use std::any::Any;
use std::panic;

/// Helper function to create a context with a given predecessor account ID
/// Mock the context and attach the predecessor account ID
/// environment variables: https://docs.near.org/build/smart-contracts/anatomy/environment#environment-variables
fn get_context(predecessor_account_id: AccountId) -> VMContextBuilder {
    let mut builder = VMContextBuilder::new();
    builder.predecessor_account_id(predecessor_account_id);
    builder
}

/// Helper function to specify the signer of the transaction
fn specify_signer(account_number: usize) -> AccountId {
    let owner = accounts(account_number);
    let context = get_context(owner.clone());
    testing_env!(context.build());
    owner
}

fn fetch_event(logs: &str) -> (Vec<serde_json::Value>, String) {
    let event_json = logs.trim_start_matches("EVENT_JSON:");
    let event: serde_json::Value = serde_json::from_str(event_json).unwrap();
    (
        event["data"].as_array().unwrap().to_vec(),
        event["event"].as_str().unwrap().to_owned(),
    )
}

fn check_error_message(result: Result<(), Box<dyn Any + Send>>, message: &str) {
    match result {
        Ok(_) => panic!("Expected an error, but got a success result"),
        Err(err) => {
            if let Some(msg) = err.downcast_ref::<String>() {
                assert_eq!(msg, message);
            } else {
                panic!("Unexpected error occurred");
            }
        }
    }
}

#[test]
fn test_staker_initialised_event() {
    NearStaker::new(accounts(0), accounts(1), accounts(2));

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[0]);

    assert_eq!(event, "staker_initialised_event");
    assert_eq!(data[0]["owner"].as_str().unwrap(), accounts(0));
    assert_eq!(data[0]["treasury"].as_str().unwrap(), accounts(1));
    assert_eq!(
        data[0]["default_delegation_pool"].as_str().unwrap(),
        accounts(2)
    );
    assert_eq!(data[0]["fee"].as_u64().unwrap() as u16, 0);
    assert_eq!(data[0]["distribution_fee"].as_u64().unwrap() as u16, 0);
    assert_eq!(
        data[0]["min_deposit"].as_str().unwrap(),
        "1000000000000000000000000"
    );
}

#[test]
fn test_get_public_owner_id() {
    // sign as owner
    let owner = specify_signer(0);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    assert_eq!(staker.owner_id, owner);

    // non-owner calls pub struct data
    specify_signer(4);
    assert_eq!(staker.owner_id, owner);
}

#[test]
fn test_set_treasury() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    let new_treasury: AccountId = accounts(5);

    staker.set_treasury(new_treasury.clone());

    assert_eq!(staker.treasury, new_treasury);

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "set_treasury_event");
    assert_eq!(data[0]["new_treasury"].as_str().unwrap(), new_treasury);
    assert_eq!(data[0]["old_treasury"].as_str().unwrap(), accounts(1));

    // assert the treasury has a TruNEAR account
    assert!(staker.token.accounts.contains_key(&new_treasury));
}

#[test]
fn test_set_treasury_called_by_non_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // non-owner tries to call only-owner method
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_treasury(accounts(2));
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_set_fee() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let new_fee: u16 = 100;

    staker.set_fee(new_fee);

    assert_eq!(staker.fee, new_fee);

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "set_fee_event");
    assert_eq!(data[0]["new_fee"].as_u64().unwrap() as u16, new_fee);
    assert_eq!(data[0]["old_fee"].as_u64().unwrap() as u16, 0);
}

#[test]
fn test_set_fee_called_by_non_owner_fails() {
    // sign as non-owner
    specify_signer(3);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // non-owner tries to call only-owner method
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_fee(40);
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_set_fee_above_fee_precision_fails() {
    // sign as non-owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // try to set fee above fee precision
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_fee(FEE_PRECISION + 1);
        }),
        "Fee cannot be larger than fee precision",
    );
}

#[test]
fn test_set_distribution_fee() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let new_distribution_fee: u16 = 200;

    staker.set_distribution_fee(new_distribution_fee);

    assert_eq!(staker.distribution_fee, new_distribution_fee);

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "set_distribution_fee_event");
    assert_eq!(
        data[0]["new_distribution_fee"].as_u64().unwrap() as u16,
        new_distribution_fee
    );
    assert_eq!(data[0]["old_distribution_fee"].as_u64().unwrap() as u16, 0);
}

#[test]
fn test_set_distribution_fee_called_by_non_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    // non-owner tries to call only-owner method
    // non-owner tries to call only-owner method
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_distribution_fee(40);
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_set_distribution_fee_above_fee_precision_fails() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // try to set fee above fee precision
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_distribution_fee(FEE_PRECISION + 1);
        }),
        "Fee cannot be larger than fee precision",
    );
}

#[test]
fn test_set_min_deposit() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let min_deposit: U128 = U128::from(ONE_NEAR * 2);

    staker.set_min_deposit(min_deposit);

    assert_eq!(staker.min_deposit, u128::from(min_deposit));

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "set_min_deposit_event");

    let event_new_deposit: String =
        serde_json::from_value(data[0]["new_min_deposit"].clone()).unwrap();
    assert_eq!(min_deposit.0.to_string(), event_new_deposit);

    let event_old_deposit: String =
        serde_json::from_value(data[0]["old_min_deposit"].clone()).unwrap();
    assert_eq!(ONE_NEAR.to_string(), event_old_deposit);
}

#[test]
fn test_set_min_deposit_called_by_non_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let min_deposit: U128 = U128::from(ONE_NEAR);

    // non-owner tries to call only-owner method
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_min_deposit(min_deposit);
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_set_min_deposit_below_one_near_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let min_deposit: U128 = U128::from(100);
    // set min_deposit below one near
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_min_deposit(min_deposit);
        }),
        "Minimum deposit amount is too small",
    );
}

#[test]
fn test_check_owner() {
    // sign as owner
    specify_signer(0);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    staker.check_owner();
}

#[test]
fn test_check_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.check_owner();
        }),
        "Only the owner can call this method",
    );
}
#[test]
fn test_is_owner() {
    // sign as owner
    specify_signer(0);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    assert!(staker.is_owner(accounts(0)));
}

#[test]
fn test_is_owner_with_non_owner() {
    // sign as owner
    specify_signer(0);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    assert!(!staker.is_owner(accounts(1)));
}

#[test]
fn test_check_not_paused_called_by_owner() {
    // sign as owner
    specify_signer(0);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    let result = std::panic::catch_unwind(move || {
        let _ = &staker.check_not_paused();
    });
    assert!(result.is_ok());
}

#[test]
fn test_check_not_paused_called_by_non_owner() {
    // sign as non-owner
    specify_signer(4);
    let staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    staker.check_not_paused();
}

#[test]
fn test_check_not_paused_fails() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.check_not_paused();
        }),
        "Contract is paused",
    );
}

#[test]
fn test_check_not_paused_after_unpausing() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();
    staker.unpause();

    staker.check_not_paused();
}

#[test]
fn test_pause() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();

    assert!(staker.is_paused);

    // assert event was emitted
    let (_, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "paused_event");
}

#[test]
fn test_pause_called_by_non_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.pause();
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_unpause() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();

    staker.unpause();

    assert!(!staker.is_paused);

    // assert event was emitted
    let (_, event) = fetch_event(&get_logs()[2]);

    assert_eq!(event, "unpaused_event");
}

#[test]
fn test_unpause_called_by_non_owner_fails() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();

    // change signer to non-owner
    specify_signer(4);

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.unpause();
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_pause_already_paused_fails() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.pause();

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.pause();
        }),
        "Contract is paused",
    );
}

#[test]
fn test_unpaused_already_unpaused_fails() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.unpause();
        }),
        "Contract is not paused",
    );
}

#[test]
fn test_set_default_delegation_pool() {
    // sign as owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    staker.add_pool(accounts(3));
    staker.set_default_delegation_pool(accounts(3));

    assert_eq!(staker.default_delegation_pool, accounts(3));

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[2]);

    assert_eq!(event, "set_default_delegation_pool_event");
    assert_eq!(
        data[0]["new_default_delegation_pool"].as_str().unwrap(),
        accounts(3)
    );
    assert_eq!(
        data[0]["old_default_delegation_pool"].as_str().unwrap(),
        accounts(2)
    );
}

#[test]
fn test_set_default_delegation_pool_not_called_by_owner_fails() {
    // sign as non-owner
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // non-owner tries to call only-owner method
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_default_delegation_pool(accounts(2));
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_set_non_existent_default_delegation_pool_fails() {
    // sign as non-owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    // tries to set default pool to a non-registered pool
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_default_delegation_pool(accounts(3));
        }),
        "Delegation pool does not exist",
    );
}

#[test]
fn test_set_disabled_default_delegation_pool_fails() {
    // sign as non-owner
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    staker.add_pool(accounts(4));
    staker.disable_pool(accounts(4));
    // tries to set default pool to a non-registered pool
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_default_delegation_pool(accounts(4));
        }),
        "Delegation pool not enabled",
    );
}

#[test]
fn test_share_price_with_zero_shares_supply() {
    specify_signer(0);
    let total_staked: u128 = 0;
    let shares_supply: u128 = 0;
    let tax_exempt_stake: u128 = 0;
    let fee: u16 = 0;

    let (numm, denom) =
        NearStaker::internal_share_price(total_staked, shares_supply, tax_exempt_stake, fee);

    assert_eq!(numm, U256::from(SHARE_PRICE_SCALING_FACTOR));
    assert_eq!(denom, U256::from(1));
}

#[test]
fn test_share_price_with_total_staked_matching_share_supply() {
    specify_signer(0);
    let total_staked = 100 * ONE_NEAR;
    let shares_supply = 100 * ONE_NEAR;
    let tax_exempt_stake: u128 = 0;
    let fee: u16 = 0;

    let (numm, denom) =
        NearStaker::internal_share_price(total_staked, shares_supply, tax_exempt_stake, fee);

    // verify the share price numerator and denominator
    let exepected_num = mul256(
        total_staked,
        FEE_PRECISION as u128 * SHARE_PRICE_SCALING_FACTOR,
    );
    let expected_denom = mul256(shares_supply, FEE_PRECISION as u128);
    assert_eq!(numm, exepected_num);
    assert_eq!(denom, expected_denom);

    // verify that the share price is 1.0
    let share_price = (numm / denom).as_u128();
    assert_eq!(share_price, SHARE_PRICE_SCALING_FACTOR);
}

#[test]
fn test_share_price_with_total_staked_greater_than_share_supply() {
    specify_signer(0);
    let total_staked = 246 * ONE_NEAR;
    let shares_supply = 200 * ONE_NEAR;
    let tax_exempt_stake: u128 = 0;
    let fee: u16 = 0;

    let (numm, denom) =
        NearStaker::internal_share_price(total_staked, shares_supply, tax_exempt_stake, fee);

    // verify the share price numerator and denominator
    let exepected_num = mul256(
        total_staked,
        FEE_PRECISION as u128 * SHARE_PRICE_SCALING_FACTOR,
    );
    let expected_denom = mul256(shares_supply, FEE_PRECISION as u128);
    assert_eq!(numm, exepected_num);
    assert_eq!(denom, expected_denom);

    // verify that the share price is 1.23
    let share_price = (numm / denom).as_u128();
    assert_eq!(share_price, 1230000000000000000000000);
}

#[test]
fn test_share_price_with_fees() {
    specify_signer(0);
    let total_staked = 346 * ONE_NEAR;
    let shares_supply = 200 * ONE_NEAR;
    let tax_exempt_stake = 246 * ONE_NEAR;
    let fee: u16 = 500; // 5%

    let (numm, denom) =
        NearStaker::internal_share_price(total_staked, shares_supply, tax_exempt_stake, fee);

    // verify the share price numerator and denominator
    let expected_fees = 5 * ONE_NEAR;
    let exepected_num = mul256(
        total_staked - expected_fees,
        SHARE_PRICE_SCALING_FACTOR * FEE_PRECISION as u128,
    );
    let expected_denom = mul256(shares_supply, FEE_PRECISION as u128);
    assert_eq!(numm, exepected_num);
    assert_eq!(denom, expected_denom);

    // verify that the share price is 1.705
    let share_price = (numm / denom).as_u128();
    assert_eq!(share_price, 1705000000000000000000000);
}

#[test]
fn test_convert_to_assets() {
    specify_signer(0);

    // set share price numerator and denominator so that share price is 3
    let price_num = mul256(6 * ONE_NEAR, SHARE_PRICE_SCALING_FACTOR);
    let price_denom = U256::from(2 * ONE_NEAR);
    assert_eq!(
        (price_num / price_denom).as_u128(),
        3 * SHARE_PRICE_SCALING_FACTOR
    );

    // convert 1000 shares to assets
    let shares: u128 = 1000 * ONE_NEAR;
    let assets = NearStaker::convert_to_assets(shares, price_num, price_denom, true);

    // verify the expected amount of assets
    assert_eq!(assets, 3000 * ONE_NEAR);
}

#[test]
fn test_convert_to_assets_with_rounding() {
    specify_signer(0);

    // set the share price numerator slightly greater than the share price denominator
    let price_num = mul256(ONE_NEAR + 10, SHARE_PRICE_SCALING_FACTOR);
    let price_denom = U256::from(ONE_NEAR + 9);

    // verify that the share price is exactly 1.0 when rounded down
    assert_eq!(
        (price_num / price_denom).as_u128(),
        SHARE_PRICE_SCALING_FACTOR
    );

    // calculate the assets for 1 share rounding up and down
    let shares: u128 = ONE_NEAR;
    let assets_rounded_up = NearStaker::convert_to_assets(shares, price_num, price_denom, true);
    let assets_rounded_down = NearStaker::convert_to_assets(shares, price_num, price_denom, false);

    // verify that assets round up and down correctly
    assert_eq!(assets_rounded_up, ONE_NEAR + 1);
    assert_eq!(assets_rounded_down, ONE_NEAR);
}

#[test]
fn test_set_pending_owner() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let new_owner = accounts(3);

    staker.set_pending_owner(new_owner.clone());

    assert_eq!(staker.pending_owner, Some(new_owner.clone()));

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[1]);

    assert_eq!(event, "set_pending_owner_event");
    assert_eq!(data[0]["pending_owner"].as_str().unwrap(), new_owner);
    assert_eq!(data[0]["current_owner"].as_str().unwrap(), accounts(0));
}

#[test]
fn test_set_pending_owner_twice() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    staker.set_pending_owner(accounts(3));
    assert_eq!(staker.pending_owner, Some(accounts(3)));

    staker.set_pending_owner(accounts(4));
    assert_eq!(staker.pending_owner, Some(accounts(4)));
}

#[test]
fn test_set_pending_owner_called_by_non_owner_fails() {
    specify_signer(4);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.set_pending_owner(accounts(3));
        }),
        "Only the owner can call this method",
    );
}

#[test]
fn test_claim_ownership() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let new_owner = accounts(3);

    staker.set_pending_owner(new_owner.clone());

    specify_signer(3);
    staker.claim_ownership();

    assert_eq!(staker.owner_id, new_owner);
    assert_eq!(staker.pending_owner, None);

    // assert event was emitted
    let (data, event) = fetch_event(&get_logs()[0]);

    assert_eq!(event, "ownership_claimed_event");
    assert_eq!(data[0]["old_owner"].as_str().unwrap(), accounts(0));
    assert_eq!(data[0]["new_owner"].as_str().unwrap(), new_owner);
}

#[test]
fn test_claim_ownership_when_no_pending_owner_is_set_fails() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));

    check_error_message(
        std::panic::catch_unwind(move || {
            staker.claim_ownership();
        }),
        "No pending owner set",
    );
}

#[test]
fn test_claim_ownership_with_non_pending_owner_fails() {
    specify_signer(0);
    let mut staker = NearStaker::new(accounts(0), accounts(1), accounts(2));
    let new_owner = accounts(3);

    staker.set_pending_owner(new_owner);
    check_error_message(
        std::panic::catch_unwind(move || {
            staker.claim_ownership();
        }),
        "Only the pending owner can claim ownership",
    );
}

#[test]
fn test_mul256_simple() {
    assert_eq!(mul256(10_000, 20_000), U256::from(200_000_000));
}

#[test]
fn test_mul256_with_zero() {
    assert_eq!(mul256(0, 12345), U256::from(0));
    assert_eq!(mul256(12345, 0), U256::from(0));
}

#[test]
fn test_mul256_with_one() {
    assert_eq!(mul256(1, 12345), U256::from(12345));
    assert_eq!(mul256(12345, 1), U256::from(12345));
}

#[test]
fn test_mul256_large_numbers() {
    assert_eq!(mul256(u128::MAX, 2), U256::from(u128::MAX) * U256::from(2));
}

#[test]
fn test_mul256_max_values() {
    assert_eq!(
        mul256(u128::MAX, u128::MAX),
        U256::from(u128::MAX) * U256::from(u128::MAX)
    );
}

#[test]
fn test_mul_div_with_rounding_up() {
    let x = U256::from(10);
    let y = U256::from(2);
    let denominator = U256::from(6);
    let result = mul_div_with_rounding(x, y, denominator, true);
    assert_eq!(result, U256::from(4));
}

#[test]
fn test_mul_div_with_rounding_down() {
    let x = U256::from(10);
    let y = U256::from(2);
    let denominator = U256::from(6);
    let result = mul_div_with_rounding(x, y, denominator, false);
    assert_eq!(result, U256::from(3));
}

#[test]
fn test_mul_div_with_rounding_zero_factor() {
    let zero = U256::from(0);
    let one = U256::from(1);
    assert_eq!(mul_div_with_rounding(zero, one, one, false), zero);
    assert_eq!(mul_div_with_rounding(one, zero, one, false), zero);
    assert_eq!(mul_div_with_rounding(zero, zero, one, false), zero);
    assert_eq!(mul_div_with_rounding(zero, one, one, true), zero);
    assert_eq!(mul_div_with_rounding(one, zero, one, true), zero);
    assert_eq!(mul_div_with_rounding(zero, zero, one, true), zero);
}

#[test]
fn test_mul_div_with_rounding_small_numbers() {
    let zero = U256::from(0);
    let one = U256::from(1);
    let two = U256::from(2);
    let three = U256::from(3);

    assert_eq!(mul_div_with_rounding(two, one, two, true), one);
    assert_eq!(mul_div_with_rounding(three, one, two, false), one);
    assert_eq!(mul_div_with_rounding(three, one, two, true), two);
    assert_eq!(mul_div_with_rounding(three, two, two, false), three);

    assert_eq!(mul_div_with_rounding(two, one, three, false), zero);
    assert_eq!(mul_div_with_rounding(two, one, three, true), one);
    assert_eq!(mul_div_with_rounding(three, one, three, false), one);
    assert_eq!(mul_div_with_rounding(three, two, three, false), two);
}

#[test]
fn test_mul_div_with_rounding_large_numbers_and_exact_result() {
    let x = U256::from_dec_str("123456789012345678901234567890").unwrap();
    let y = U256::from_dec_str("98765432109876543210987654321").unwrap();
    let denominator = U256::from(2);

    // verify x * y / denominator is an exact result
    assert_eq!((x * y) % denominator, U256::from(0));
    // verify rounding up and down produces the same result
    let expected_result =
        U256::from_dec_str("6096631556851089761309251636681146166611873190055563176345").unwrap();
    assert_eq!(
        mul_div_with_rounding(x, y, denominator, true),
        expected_result
    );
    assert_eq!(
        mul_div_with_rounding(x, y, denominator, false),
        expected_result
    );
}

#[test]
fn test_mul_div_with_rounding_overflow_fails() {
    let x = U256::from_dec_str(
        "57896044618658097711785492504343953926634992332820282019728792003956564819968",
    )
    .unwrap();
    let y = U256::from(2);
    let denominator = U256::from(1);

    // verify x is the min number that will overflow when multiplied by 2
    assert_eq!(x, U256::MAX / U256::from(2) + U256::from(1));

    let result = panic::catch_unwind(|| mul_div_with_rounding(x, y, denominator, false));

    // verify that the operation did overflow
    assert!(result.is_err());
    let error = result.unwrap_err();
    let message = error.downcast_ref::<&str>().unwrap();
    assert_eq!(*message, "arithmetic operation overflow");
}

#[test]
fn test_mul_div_with_rounding_division_by_zero_fails() {
    let x = U256::from(1000);
    let y = U256::from(2);
    let denominator = U256::from(0);

    let result = panic::catch_unwind(|| mul_div_with_rounding(x, y, denominator, false));

    // verify that the operation did overflow
    assert!(result.is_err());
    let error = result.unwrap_err();
    let message = error.downcast_ref::<&str>().unwrap();
    assert_eq!(*message, "division by zero");
}

#[test]
fn test_internal_calculate_distribution_amount_with_large_allocation() {
    let global_share_price_num = U256::from(SHARE_PRICE_SCALING_FACTOR);
    let global_share_price_denom = U256::from(1);
    let allocation = Allocation {
        near_amount: u128::MAX,
        share_price_num: U256::from(SHARE_PRICE_SCALING_FACTOR),
        share_price_denom: U256::from(2),
    };
    let dist_amount = NearStaker::internal_calculate_distribution_amount(
        &allocation,
        global_share_price_num,
        global_share_price_denom,
    );
    assert_eq!(dist_amount, u128::MAX);
}

#[test]
fn test_internal_calculate_distribution_amount_with_large_share_price() {
    // Assume 20M deposit at a share price of 10.0
    let global_share_price_num = U256::from_dec_str("200000000000000000000000000000000").unwrap()
        * U256::from(FEE_PRECISION as u128)
        * U256::from(SHARE_PRICE_SCALING_FACTOR);
    let global_share_price_denom = U256::from_dec_str("20000000000000000000000000000000").unwrap()
        * U256::from(FEE_PRECISION as u128);
    let allocation = Allocation {
        near_amount: u128::MAX,
        share_price_num: U256::from(SHARE_PRICE_SCALING_FACTOR),
        share_price_denom: U256::from(1),
    };
    let expected_result: u128 = 306254130228844617117037146688591390310;
    let dist_amount = NearStaker::internal_calculate_distribution_amount(
        &allocation,
        global_share_price_num,
        global_share_price_denom,
    );
    assert_eq!(dist_amount, expected_result);
}

#[test]
fn test_saturating_sub() {
    let result = 20000000000000000000000000u128.saturating_sub(19999999999999999999999999u128);
    assert_eq!(result, 1);
}

#[test]
fn test_saturating_sub_with_overflow() {
    let result = 19999999999999999999999999u128.saturating_sub(20000000000000000000000000u128);
    assert_eq!(result, 0);
}
