#![cfg(test)]
use super::*;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, BytesN, Env};

fn make_hash(env: &Env, seed: u8) -> BytesN<32> {
    BytesN::from_array(env, &[seed; 32])
}

fn setup_ban<'a>() -> (Env, AhjoorPaymentsContractClient<'a>, Address, Address, Address, TokenClient<'a>, TokenAdminClient<'a>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorPaymentsContract, ());
    let client = AhjoorPaymentsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let token_client = TokenClient::new(&env, &token_addr);
    let token_admin_client = TokenAdminClient::new(&env, &token_addr);

    client.initialize(&admin, &admin, &0u32);
    // Set min_collateral to 0 so approve_merchant doesn't require a deposit in tests
    client.set_min_collateral(&0i128);

    (env, client, admin, merchant, token_addr, token_client, token_admin_client)
}

// ---------------------------------------------------------------------------
// Test: suspension — payments paused, expiry auto-lifts
// ---------------------------------------------------------------------------
#[test]
fn test_suspension_blocks_payments() {
    let (env, client, admin, merchant, token_addr, _token_client, token_admin_client) = setup_ban();

    // Approve merchant first
    client.approve_merchant(&merchant);
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Active);

    // Suspend for 3600 seconds
    client.suspend_merchant(&admin, &merchant, &make_hash(&env, 1), &3600u64);
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Suspended);

    // Payment should be blocked
    let customer = Address::generate(&env);
    token_admin_client.mint(&customer, &1000);
    let result = client.try_create_payment(&customer, &merchant, &100, &token_addr, &None, &None, &None);
    assert!(result.is_err());
}

// ---------------------------------------------------------------------------
// Test: ban + appeal + approval path
// ---------------------------------------------------------------------------
#[test]
fn test_ban_appeal_approval() {
    let (env, client, admin, merchant, _token_addr, _token_client, _token_admin_client) = setup_ban();

    client.approve_merchant(&merchant);
    client.ban_merchant(&admin, &merchant, &make_hash(&env, 2));
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Banned);

    // Merchant submits appeal
    client.submit_appeal(&merchant, &make_hash(&env, 3));
    let appeal = client.get_merchant_appeal(&merchant).unwrap();
    assert!(!appeal.resolved);

    // Admin approves
    client.approve_appeal(&admin, &merchant);
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Active);

    let appeal = client.get_merchant_appeal(&merchant).unwrap();
    assert!(appeal.resolved);
    assert!(appeal.approved);
}

// ---------------------------------------------------------------------------
// Test: ban + appeal + rejection + cooldown
// ---------------------------------------------------------------------------
#[test]
fn test_ban_appeal_rejection_cooldown() {
    let (env, client, admin, merchant, _token_addr, _token_client, _token_admin_client) = setup_ban();

    let cooldown = 7200u64;
    client.set_appeal_rejection_cooldown(&admin, &cooldown);

    client.approve_merchant(&merchant);
    client.ban_merchant(&admin, &merchant, &make_hash(&env, 4));

    client.submit_appeal(&merchant, &make_hash(&env, 5));
    client.reject_appeal(&admin, &merchant);

    // Merchant still banned
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Banned);

    // Immediate re-appeal blocked by cooldown
    let result = client.try_submit_appeal(&merchant, &make_hash(&env, 6));
    assert!(result.is_err());

    // Advance past cooldown
    env.ledger().with_mut(|l| l.timestamp += cooldown + 1);

    // Now appeal is allowed
    client.submit_appeal(&merchant, &make_hash(&env, 7));
    let appeal = client.get_merchant_appeal(&merchant).unwrap();
    assert!(!appeal.resolved);
}

// ---------------------------------------------------------------------------
// Test: one-active-appeal guard
// ---------------------------------------------------------------------------
#[test]
#[should_panic(expected = "An active appeal already exists")]
fn test_duplicate_appeal_rejected() {
    let (env, client, admin, merchant, _token_addr, _token_client, _token_admin_client) = setup_ban();

    client.approve_merchant(&merchant);
    client.ban_merchant(&admin, &merchant, &make_hash(&env, 8));

    client.submit_appeal(&merchant, &make_hash(&env, 9));
    // Second appeal while first is unresolved — should panic
    client.submit_appeal(&merchant, &make_hash(&env, 10));
}

// ---------------------------------------------------------------------------
// Test: reinstate_merchant clears ban
// ---------------------------------------------------------------------------
#[test]
fn test_reinstate_merchant() {
    let (env, client, admin, merchant, _token_addr, _token_client, _token_admin_client) = setup_ban();

    client.approve_merchant(&merchant);
    client.ban_merchant(&admin, &merchant, &make_hash(&env, 11));
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Banned);

    client.reinstate_merchant(&admin, &merchant);
    assert_eq!(client.get_merchant_status(&merchant), MerchantStatus::Active);
    assert!(client.is_merchant_approved(&merchant));
}
