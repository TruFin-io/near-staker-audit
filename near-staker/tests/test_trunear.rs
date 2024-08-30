use near_sdk::json_types::U128;
use near_sdk::serde_json::json;
use near_sdk::test_utils::accounts;
use near_sdk::{Gas, NearToken};

pub mod event;
pub mod helpers;
mod types;

use event::*;
use helpers::*;
use types::*;

#[tokio::test]
async fn test_ft_total_supply_is_zero_after_deployment() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract, _) = setup_contract_with_pool().await?;

    let total_supply = get_total_supply(&contract).await?;

    // verify the initial supply is 0
    assert_eq!(total_supply, 0);

    Ok(())
}

#[tokio::test]
async fn test_ft_metadata() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;

    let response = contract.view("ft_metadata").args_json(json!({})).await?;
    let metadata: FungibleTokenMetadata = response.json().unwrap();

    // verify token metadata
    assert_eq!(
        metadata,
        FungibleTokenMetadata {
            spec: "ft-1.0.0".to_string(),
            name: "TruNEAR Token".to_string(),
            symbol: "TruNEAR".to_string(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 24,
        }
    );

    Ok(())
}

#[tokio::test]
async fn test_ft_total_supply() -> Result<(), Box<dyn std::error::Error>> {
    let alice_supply = 10 * ONE_NEAR;
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let total_supply = get_total_supply(&contract).await?;

    assert!(total_supply == alice_supply);

    Ok(())
}

#[tokio::test]
async fn test_ft_balance_of() -> Result<(), Box<dyn std::error::Error>> {
    let alice_supply = 10 * ONE_NEAR;
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let alice_balance = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": alice.id()
        }))
        .await?
        .json::<U128>()
        .unwrap();

    assert!(alice_balance.0 == alice_supply);

    Ok(())
}

#[tokio::test]
async fn test_ft_balance_of_unregistered_account() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;
    let balance = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": accounts(2)
        }))
        .await?
        .json::<U128>()
        .unwrap();

    assert!(balance.0 == 0);

    Ok(())
}

#[tokio::test]
async fn test_ft_balance_of_stake_twice() -> Result<(), Box<dyn std::error::Error>> {
    let alice_supply = 10 * ONE_NEAR;
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let stake_2 = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(5))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake_2.is_success());

    let alice_balance = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": alice.id()
        }))
        .await?
        .json::<U128>()
        .unwrap();

    assert!(alice_balance.0 == alice_supply);

    Ok(())
}

#[tokio::test]
async fn test_ft_transfer() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_user(&sandbox, "bob").await?;

    // alice stakes to receive TruNEAR
    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    // bob must first register before he can be transferred TruNEAR
    let register = bob
        .call(contract.id(), "storage_deposit")
        .args_json(json!(
            {
                "account_id": bob.id(),
                "registration_only": true
            }
        ))
        .deposit(NearToken::from_near(1))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(register.is_success());

    // call ft_transfer attaching 1 yoctoNEAR as deposit
    let response = alice
        .call(contract.id(), "ft_transfer")
        .args_json(json!({
            "receiver_id": bob.id(),
            "amount": U128(2 * ONE_NEAR),
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;
    assert!(response.is_success());

    // verify the tokens were transferred from Alice to Bob
    let alice_balance = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": alice.id()
        }))
        .await?
        .json::<U128>()
        .unwrap();
    assert!(alice_balance == U128(8 * ONE_NEAR));

    let bob_balance = contract
        .view("ft_balance_of")
        .args_json(json!({
            "account_id": bob.id()
        }))
        .await?
        .json::<U128>()
        .unwrap();
    assert!(bob_balance == U128(2 * ONE_NEAR));

    // verify the ft_transfer event was emitted
    let event_log = response.logs().into_iter().next().unwrap();
    let event: Event<TransferEvent> =
        serde_json::from_str(event_log.strip_prefix("EVENT_JSON:").unwrap()).unwrap();

    verify_nep141_event(
        event,
        "ft_transfer",
        vec![TransferEvent {
            old_owner_id: alice.id().to_string(),
            new_owner_id: bob.id().to_string(),
            amount: (2 * ONE_NEAR).to_string(),
            memo: None,
        }],
    );

    Ok(())
}

#[tokio::test]
async fn test_ft_resolve_transfer() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract, _) = setup_contract_with_pool().await?;

    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;
    let bob = setup_user(&sandbox, "bob").await?;

    let response = alice
        .call(contract.id(), "ft_resolve_transfer")
        .args_json(json!({
            "sender_id": alice.id(),
            "receiver_id": bob.id(),
            "amount": U128(1 * ONE_NEAR),
        }))
        .transact()
        .await?;

    // verify that ft_resolve_transfer exists by checking that
    // its execution failed because it's private.
    assert!(response.is_failure());
    check_error_msg(response, "Method ft_resolve_transfer is private");

    Ok(())
}

#[tokio::test]
async fn test_storage_balance_of() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let storage_bounds = contract.view("storage_balance_bounds").await?;
    let bounds: StorageBalanceBounds = storage_bounds.json().unwrap();

    // Alice's storage balance should be equal to the minimum storage balance
    let alice_balance = contract
        .view("storage_balance_of")
        .args_json(json!({
            "account_id": alice.id()
        }))
        .await?;

    let balance_struct: StorageBalance = alice_balance.json().unwrap();
    assert_eq!(balance_struct.total, bounds.min);
    assert_eq!(balance_struct.available, NearToken::from_near(0));

    Ok(())
}

#[tokio::test]
async fn test_storage_balance_of_unregistered_account() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;

    let balance = contract
        .view("storage_balance_of")
        .args_json(json!({
            "account_id": accounts(3)
        }))
        .await?;

    let balance_struct: Option<StorageBalance> = balance.json().unwrap();
    assert!(balance_struct.is_none());
    Ok(())
}

#[tokio::test]
async fn test_storage_withdraw_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let result = alice
        .call(contract.id(), "storage_withdraw")
        .args_json(json!({
            "amount": NearToken::from_near(0)
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;

    assert!(result.is_failure());

    Ok(())
}

#[tokio::test]
async fn test_storage_bounds() -> Result<(), Box<dyn std::error::Error>> {
    let (_, _, contract) = setup_contract().await?;
    // Fungible Token implementation sets storage_balance_bounds.min == storage_balance_bounds.max,
    let storage_bounds = contract.view("storage_balance_bounds").await?;
    let bounds: StorageBalanceBounds = storage_bounds.json().unwrap();
    assert!(Some(bounds.min) == bounds.max);

    Ok(())
}

#[tokio::test]
async fn test_unregister_account_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract, _) = setup_contract_with_pool().await?;
    let alice = setup_whitelisted_user(&owner, &contract, "alice").await?;

    let stake = alice
        .call(contract.id(), "stake")
        .deposit(NearToken::from_near(10))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(stake.is_success());

    let unregister = alice
        .call(contract.id(), "storage_unregister")
        .args_json(json!({
            "force": false
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;

    assert!(unregister.is_failure());

    let unregister = alice
        .call(contract.id(), "storage_unregister")
        .args_json(json!({
            "force": true
        }))
        .deposit(NearToken::from_yoctonear(1))
        .transact()
        .await?;

    // We do not allow users to unregister so the call will still fail
    assert!(unregister.is_failure());

    Ok(())
}
