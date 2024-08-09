pub mod helpers;
use helpers::*;
use near_sdk::test_utils::accounts;
use serde_json::json;

#[tokio::test]
async fn test_init_twice_fails() -> Result<(), Box<dyn std::error::Error>> {
    let (owner, _, contract) = setup_contract().await?;

    // call init a second time
    let init = contract
        .call("new")
        .args_json(json!({
            "owner_id": owner.id(),
            "treasury": accounts(1),
            "default_delegation_pool": accounts(2),
        }))
        .transact()
        .await?;

    // verify that the call failed with the expected error message
    assert!(init.is_failure());
    check_error_msg(init, "The contract has already been initialized");

    Ok(())
}
