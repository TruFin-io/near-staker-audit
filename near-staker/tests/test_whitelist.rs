use near_sdk::test_utils::accounts;
use serde_json::json;

pub mod helpers;

use helpers::*;

#[tokio::test]
async fn test_add_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let response = owner
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;
    assert!(response.is_success());

    assert!(contract
        .view("is_agent")
        .args_json(json!({"agent_id": alice.id()}))
        .await?
        .json::<bool>()
        .unwrap());

    Ok(())
}

#[tokio::test]
async fn test_remove_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;
    let _ = owner
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    let response = owner
        .call(contract.id(), "remove_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_success());
    assert!(!contract
        .view("is_agent")
        .args_json(json!({"agent_id": alice.id()}))
        .await?
        .json::<bool>()
        .unwrap());

    Ok(())
}

#[tokio::test]
async fn test_owner_is_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let is_agent = contract
        .view("is_agent")
        .args_json(json!({"agent_id": owner.id()}))
        .await?
        .json::<bool>()
        .unwrap();

    assert!(is_agent);

    Ok(())
}

#[tokio::test]
async fn test_caller_not_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let response = alice
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_failure());
    check_error_msg(response, "Caller is not an agent");

    Ok(())
}

#[tokio::test]
async fn test_cannot_add_owner_as_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let response = owner
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": owner.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_failure());
    check_error_msg(response, "Owner cannot be added as an agent");

    Ok(())
}

#[tokio::test]
async fn test_cannot_remove_owner_as_agent() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let response = owner
        .call(contract.id(), "remove_agent")
        .args_json(json!({
            "agent_id": owner.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_failure());
    check_error_msg(response, "Owner cannot be removed as an agent");

    Ok(())
}

#[tokio::test]
async fn test_cannot_add_an_agent_that_already_exists() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;
    let _ = owner
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    let response = owner
        .call(contract.id(), "add_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_failure());
    check_error_msg(response, "Agent already exists");

    Ok(())
}

#[tokio::test]
async fn test_cannot_remove_an_agent_that_does_not_exist() -> Result<(), Box<dyn std::error::Error>>
{
    let (owner, sandbox, contract) = setup_contract().await?;
    let alice = setup_user(&sandbox, "alice").await?;

    let response = owner
        .call(contract.id(), "remove_agent")
        .args_json(json!({
            "agent_id": alice.id(),
        }))
        .transact()
        .await?;

    assert!(response.is_failure());
    check_error_msg(response, "Agent does not exist");

    Ok(())
}

#[tokio::test]
async fn test_add_user_to_whitelist() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let result = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": accounts(1),
        }))
        .transact()
        .await?;

    let event_json = get_event(result.logs());

    assert!(result.is_success());
    assert!(contract
        .view("is_whitelisted")
        .args_json(json!({
            "user_id": accounts(1),
        }))
        .await?
        .json::<bool>()
        .unwrap());
    assert_eq!(event_json["event"], "whitelist_state_changed_event");

    Ok(())
}

#[tokio::test]
async fn test_add_user_to_blacklist() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let result = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .transact()
        .await?;

    let event_json = get_event(result.logs());

    assert!(result.is_success());
    assert!(contract
        .view("is_blacklisted")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .await?
        .json::<bool>()
        .unwrap());
    assert_eq!(event_json["event"], "whitelist_state_changed_event");

    Ok(())
}

#[tokio::test]
async fn test_only_owner_can_add_to_users_list() -> Result<(), Box<dyn std::error::Error>> {
    let (_, sandbox, contract) = setup_contract().await?;

    let result = setup_user(&sandbox, "alice")
        .await?
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    assert!(!contract
        .view("is_blacklisted")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .await?
        .json::<bool>()
        .unwrap());
    check_error_msg(result, "Caller is not an agent");

    Ok(())
}

#[tokio::test]
async fn test_clear_user_status() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(3),
        }))
        .transact()
        .await?;

    let result = owner
        .call(contract.id(), "clear_user_status")
        .args_json(json!({
            "user_id": accounts(3),
        }))
        .transact()
        .await?;

    let event_json = get_event(result.logs());

    assert!(result.is_success());
    assert!(!contract
        .view("is_blacklisted")
        .args_json(json!({
            "user_id": accounts(3),
        }))
        .await?
        .json::<bool>()
        .unwrap());
    assert_eq!(event_json["event"], "whitelist_state_changed_event");

    Ok(())
}

#[tokio::test]
async fn test_cannot_clear_user_status_if_it_has_already_been_cleared(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .transact()
        .await?;

    let _ = owner
        .call(contract.id(), "clear_user_status")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .transact()
        .await?;

    let result = owner
        .call(contract.id(), "clear_user_status")
        .args_json(json!({
            "user_id": accounts(2),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "User status already cleared");

    Ok(())
}

#[tokio::test]
async fn test_only_owner_can_clear_user_status() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(4),
        }))
        .transact()
        .await?;

    let result = setup_user(&sandbox, "alice")
        .await?
        .call(contract.id(), "clear_user_status")
        .args_json(json!({
            "user_id": accounts(4),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "Caller is not an agent");

    Ok(())
}

#[tokio::test]
async fn test_user_cannot_be_whitelisted_and_blacklisted_at_the_same_time(
) -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(5),
        }))
        .transact()
        .await?;

    let result = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": accounts(5),
        }))
        .transact()
        .await?;

    let event_json = get_event(result.logs());

    assert!(result.is_success());

    assert!(contract
        .view("is_whitelisted")
        .args_json(json!({
            "user_id": accounts(5),
        }))
        .await?
        .json::<bool>()
        .unwrap());

    assert!(!contract
        .view("is_blacklisted")
        .args_json(json!({
            "user_id": accounts(5),
        }))
        .await?
        .json::<bool>()
        .unwrap());

    assert_eq!(event_json["event"], "whitelist_state_changed_event");

    Ok(())
}

#[tokio::test]
async fn test_cannot_add_duplicate_users_to_whitelist() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": accounts(4),
        }))
        .transact()
        .await?;

    let result = owner
        .call(contract.id(), "add_user_to_whitelist")
        .args_json(json!({
            "user_id": accounts(4),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "User already whitelisted");

    Ok(())
}

#[tokio::test]
async fn test_cannot_add_duplicate_users_to_blacklist() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _sandbox, contract) = setup_contract().await?;

    let _ = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(3),
        }))
        .transact()
        .await?;

    let result = owner
        .call(contract.id(), "add_user_to_blacklist")
        .args_json(json!({
            "user_id": accounts(3),
        }))
        .transact()
        .await?;

    assert!(result.is_failure());
    check_error_msg(result, "User already blacklisted");

    Ok(())
}
