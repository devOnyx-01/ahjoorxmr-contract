#![cfg(test)]

use crate::{AhjoorContract, AhjoorContractClient};
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, Env,
};

fn setup_with_members<'a>(n: usize, mint_amount: i128) -> (
    Env,
    AhjoorContractClient<'a>,
    Address,
    Address,
    soroban_sdk::Vec<Address>,
    TokenClient<'a>,
) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorContract, ());
    let client = AhjoorContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let token_admin = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = TokenClient::new(&env, &token_admin);
    let token_admin_client = TokenAdminClient::new(&env, &token_admin);

    let mut members = soroban_sdk::Vec::new(&env);
    for _ in 0..n {
        let addr = Address::generate(&env);
        if mint_amount > 0 {
            token_admin_client.mint(&addr, &mint_amount);
        }
        members.push_back(addr);
    }

    (env, client, admin, token_admin, members, token_client)
}

#[test]
fn test_emergency_loan_grant() {
    let (env, client, admin, token, members, _token_client) = setup_with_members(3, 1000);

    // Initialize with emergency reserve enabled
    let config = crate::RoscaConfig {
        strategy: crate::PayoutStrategy::RoundRobin,
        custom_order: None,
        penalty_amount: 0,
        exit_penalty_bps: 0,
        collective_goal: None,
        member_goals: None,
        fee_bps: 0,
        fee_recipient: None,
        max_defaults: 3,
        grace_period_ledgers: 0,
        use_timestamp_schedule: false,
        round_duration_seconds: 0,
        max_members: None,
        skip_fee: 0,
        max_skips_per_cycle: 0,
        voting_mode: crate::VotingMode::Equal,
        late_fee_bps: 0,
        grace_period_seconds: 0,
        auction_enabled: false,
        auction_window_ledgers: 0,
        reserve_enabled: true,
        reserve_contribution_bps: 200, // 2% surcharge
    };

    client.init(&admin, &members, &100, &token, &3600, &config, &None);

    // Contribute to build reserve
    for member in members.iter() {
        client.contribute(&member, &100);
    }

    // Request emergency loan
    let loan_id = client.request_emergency_loan(&members.get(0).unwrap(), &50, &500);

    // Verify loan was created
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.loan_id, loan_id);
    assert_eq!(loan.amount, 50);
    assert_eq!(loan.repaid_amount, 0);
}

#[test]
fn test_emergency_loan_repayment() {
    let (env, client, admin, token, members, _token_client) = setup_with_members(3, 1000);

    let config = crate::RoscaConfig {
        strategy: crate::PayoutStrategy::RoundRobin,
        custom_order: None,
        penalty_amount: 0,
        exit_penalty_bps: 0,
        collective_goal: None,
        member_goals: None,
        fee_bps: 0,
        fee_recipient: None,
        max_defaults: 3,
        grace_period_ledgers: 0,
        use_timestamp_schedule: false,
        round_duration_seconds: 0,
        max_members: None,
        skip_fee: 0,
        max_skips_per_cycle: 0,
        voting_mode: crate::VotingMode::Equal,
        late_fee_bps: 0,
        grace_period_seconds: 0,
        auction_enabled: false,
        auction_window_ledgers: 0,
        reserve_enabled: true,
        reserve_contribution_bps: 200,
    };

    client.init(&admin, &members, &100, &token, &3600, &config, &None);

    for member in members.iter() {
        client.contribute(&member, &100);
    }

    let borrower = members.get(0).unwrap();
    let loan_id = client.request_emergency_loan(&borrower, &50, &500);

    // Repay loan
    client.repay_emergency_loan(&borrower, &loan_id, &50);

    // Verify loan is fully repaid
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.repaid_amount, 50);
}

#[test]
fn test_duplicate_loan_rejected() {
    let (env, client, admin, token, members, _token_client) = setup_with_members(3, 1000);

    let config = crate::RoscaConfig {
        strategy: crate::PayoutStrategy::RoundRobin,
        custom_order: None,
        penalty_amount: 0,
        exit_penalty_bps: 0,
        collective_goal: None,
        member_goals: None,
        fee_bps: 0,
        fee_recipient: None,
        max_defaults: 3,
        grace_period_ledgers: 0,
        use_timestamp_schedule: false,
        round_duration_seconds: 0,
        max_members: None,
        skip_fee: 0,
        max_skips_per_cycle: 0,
        voting_mode: crate::VotingMode::Equal,
        late_fee_bps: 0,
        grace_period_seconds: 0,
        auction_enabled: false,
        auction_window_ledgers: 0,
        reserve_enabled: true,
        reserve_contribution_bps: 200,
    };

    client.init(&admin, &members, &100, &token, &3600, &config, &None);

    for member in members.iter() {
        client.contribute(&member, &100);
    }

    let borrower = members.get(0).unwrap();
    client.request_emergency_loan(&borrower, &50, &500);

    // Try to request another loan (should fail)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.request_emergency_loan(&borrower, &30, &500);
    }));
    assert!(result.is_err());
}
