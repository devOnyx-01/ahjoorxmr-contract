#![cfg(test)]
use super::*;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup<'a>() -> (
    Env,
    AhjoorPaymentsContractClient<'a>,
    Address,
    Address,
    Address,
    Address,
    TokenAdminClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorPaymentsContract, ());
    let client = AhjoorPaymentsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let merchant = Address::generate(&env);
    let buyer = Address::generate(&env);
    let token_addr = env.register_stellar_asset_contract_v2(admin.clone()).address();
    let tac = TokenAdminClient::new(&env, &token_addr);

    client.initialize(&admin, &admin, &0u32);
    client.set_min_collateral(&0i128);
    client.approve_merchant(&merchant);
    tac.mint(&buyer, &100_000);

    (env, client, admin, merchant, buyer, token_addr, tac)
}

#[test]
fn test_set_and_get_buyer_tier() {
    let (env, client, _admin, merchant, buyer, _token, _tac) = setup();

    // Default tier is New
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::New);

    // Set to Trusted
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::Trusted);

    // Upgrade to VIP
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);
    assert_eq!(client.get_buyer_tier(&merchant, &buyer), BuyerTrustTierLevel::VIP);
}

#[test]
fn test_tier_downgrade_takes_effect_immediately() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set VIP tier with high limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::VIP);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::VIP, &50_000i128, &3600u64);

    // Payment succeeds under VIP limit
    let pid = client.create_payment(&buyer, &merchant, &10_000, &token, &None, &None, &None);
    client.complete_payment(&pid);

    // Downgrade to New (strict limit)
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::New);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);

    // New payment with downgraded tier is rejected
    let pid2 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    let result = client.try_complete_payment(&pid2);
    assert!(result.is_err());
}

#[test]
fn test_tier_limit_falls_back_to_global_when_unset() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set buyer tier but no tier-specific limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);

    // Set global default limit
    client.set_default_spend_limit(&merchant, &200i128, &3600u64);

    // Payment exceeding global default should fail
    let pid = client.create_payment(&buyer, &merchant, &300, &token, &None, &None, &None);
    let result = client.try_complete_payment(&pid);
    assert!(result.is_err());
}

#[test]
fn test_per_customer_override_takes_priority_over_tier() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();

    // Set Trusted tier with low limit
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::Trusted);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &100i128, &3600u64);

    // Set per-customer override with high limit
    client.set_customer_spend_limit(&merchant, &buyer, &50_000i128, &3600u64);

    // Payment within per-customer override should succeed
    let pid = client.create_payment(&buyer, &merchant, &10_000, &token, &None, &None, &None);
    client.complete_payment(&pid);
    assert_eq!(client.get_payment(&pid).status, PaymentStatus::Completed);
}

#[test]
fn test_each_tier_has_independent_limit() {
    let (env, client, _admin, merchant, buyer, token, _tac) = setup();
    let buyer2 = Address::generate(&env);
    let tac = soroban_sdk::token::StellarAssetClient::new(&env, &token);
    tac.mint(&buyer2, &100_000);

    // Set different limits per tier
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::New, &100i128, &3600u64);
    client.set_tier_spending_limit(&merchant, &BuyerTrustTierLevel::Trusted, &10_000i128, &3600u64);

    // buyer → New tier, buyer2 → Trusted tier
    client.set_buyer_tier(&merchant, &buyer, &BuyerTrustTierLevel::New);
    client.set_buyer_tier(&merchant, &buyer2, &BuyerTrustTierLevel::Trusted);

    // New-tier buyer fails on large payment
    let p1 = client.create_payment(&buyer, &merchant, &500, &token, &None, &None, &None);
    assert!(client.try_complete_payment(&p1).is_err());

    // Trusted-tier buyer succeeds on same payment
    let p2 = client.create_payment(&buyer2, &merchant, &500, &token, &None, &None, &None);
    client.complete_payment(&p2);
    assert_eq!(client.get_payment(&p2).status, PaymentStatus::Completed);
}
