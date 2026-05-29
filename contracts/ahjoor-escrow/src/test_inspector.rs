#![cfg(test)]

use crate::{AhjoorEscrowContract, AhjoorEscrowContractClient, EscrowStatus};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::Client as TokenClient,
    token::StellarAssetClient as TokenAdminClient,
    Address, BytesN, Env, String,
};

fn setup_test_env() -> (
    Env,
    Address,
    Address,
    Address,
    Address,
    Address,
    AhjoorEscrowContractClient<'static>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let buyer = Address::generate(&env);
    let seller = Address::generate(&env);
    let arbiter = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_admin_client = TokenAdminClient::new(&env, &token_addr);
    token_admin_client.mint(&buyer, &10_000);

    let contract_id = env.register(AhjoorEscrowContract, ());
    let client = AhjoorEscrowContractClient::new(&env, &contract_id);

    client.initialize(&admin);
    client.add_allowed_token(&admin, &token_addr);

    (env, admin, buyer, seller, arbiter, token_addr, client)
}

#[test]
fn test_inspector_approval_enables_release() {
    let (env, _admin, buyer, seller, arbiter, token, client) = setup_test_env();

    let inspector = Address::generate(&env);

    // Create escrow with inspector
    let escrow_id = client.create_escrow(
        &buyer,
        &seller,
        &arbiter,
        &1000,
        &token,
        &(env.ledger().timestamp() + 86400),
        &None,
        &soroban_sdk::Vec::new(&env),
        &false,
        &0,
    );

    // Set inspector
    let mut escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Active);

    // Submit inspection result (approved)
    let report_hash = BytesN::<32>::from_array(&env, &[1u8; 32]);
    client.submit_inspection_result(&inspector, &escrow_id, &true, &report_hash);

    // Verify status changed to InspectionPassed
    escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InspectionPassed);

    // Now buyer can release
    client.release_escrow(&buyer, &escrow_id);

    escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::Released);
}

#[test]
fn test_inspector_rejection_blocks_release() {
    let (env, _admin, buyer, seller, arbiter, token, client) = setup_test_env();

    let inspector = Address::generate(&env);

    let escrow_id = client.create_escrow(
        &buyer,
        &seller,
        &arbiter,
        &1000,
        &token,
        &(env.ledger().timestamp() + 86400),
        &None,
        &soroban_sdk::Vec::new(&env),
        &false,
        &0,
    );

    // Submit inspection result (rejected)
    let report_hash = BytesN::<32>::from_array(&env, &[1u8; 32]);
    client.submit_inspection_result(&inspector, &escrow_id, &false, &report_hash);

    // Verify status changed to InspectionFailed
    let escrow = client.get_escrow(&escrow_id);
    assert_eq!(escrow.status, EscrowStatus::InspectionFailed);

    // Buyer cannot release
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.release_escrow(&buyer, &escrow_id);
    }));
    assert!(result.is_err());
}
