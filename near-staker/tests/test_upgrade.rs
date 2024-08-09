use helpers::*;
use near_sdk::{
    base64::{engine::general_purpose, Engine},
    serde_json::json,
    Gas,
};

pub mod constants;
pub mod helpers;
mod types;

#[tokio::test]
async fn test_upgrade_and_migrate() -> Result<(), Box<dyn std::error::Error>> {
    // deploy an older version of the contract
    let (owner, _, contract) =
        setup_contract_with_code("./tests/upgrades/near_staker-upgrade.wasm".to_string()).await?;

    // compile the new contract
    let upgrade_contract_wasm = near_workspaces::compile_project("./").await?;

    // upgrade the contract and migrate the contract state
    let upgrade = owner
        .call(contract.id(), "upgrade")
        .args_json(json!({
           "code": general_purpose::STANDARD.encode(&upgrade_contract_wasm),
           "migrate": true
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;
    assert!(upgrade.is_success());

    // verify that the upgraded contract can access a state variable
    let is_owner = contract
        .view("is_owner")
        .args_json(json!({
            "account_id": owner.id(),
        }))
        .await?
        .json::<bool>()
        .unwrap();
    assert!(is_owner);

    Ok(())
}

#[tokio::test]
async fn test_upgrade_by_non_owner_fails() -> Result<(), Box<dyn std::error::Error>> {
    // deploy an older version of the contract
    let (_, sandbox, contract) =
        setup_contract_with_code("./tests/upgrades/near_staker-upgrade.wasm".to_string()).await?;

    let alice = setup_user(&sandbox, "alice").await?;

    // compile the new contract
    let upgrade_contract_wasm = near_workspaces::compile_project("./").await?;

    // non owner tries to upgrade the contract and fails
    let upgrade = alice
        .call(contract.id(), "upgrade")
        .args_json(json!({
            "code": general_purpose::STANDARD.encode(&upgrade_contract_wasm),
            "migrate": true
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(upgrade.is_failure());
    check_error_msg(upgrade, "Only the owner can call this method");

    Ok(())
}

#[tokio::test]
async fn test_call_migrate_function_fails() -> Result<(), Box<dyn std::error::Error>> {
    // deploy an older version of the contract
    let (owner, sandbox, contract) =
        setup_contract_with_code("./tests/upgrades/near_staker-upgrade.wasm".to_string()).await?;

    let alice = setup_user(&sandbox, "alice").await?;

    // compile the new contract
    let upgrade_contract_wasm = near_workspaces::compile_project("./").await?;

    // owner upgrades the contract
    let upgrade = owner
        .call(contract.id(), "upgrade")
        .args_json(json!({
           "code": general_purpose::STANDARD.encode(&upgrade_contract_wasm),
           "migrate": true
        }))
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(upgrade.is_success());

    // alice tries to call migrate and fails
    let migrate = alice
        .call(contract.id(), "migrate")
        .gas(Gas::from_tgas(300))
        .transact()
        .await?;

    assert!(migrate.is_failure());
    check_error_msg(migrate, "Invalid caller");

    Ok(())
}
