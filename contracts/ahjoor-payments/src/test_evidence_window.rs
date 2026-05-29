#![cfg(test)]
extern crate alloc;
use super::*;
use soroban_sdk::token::Client as TokenClient;
use soroban_sdk::token::StellarAssetClient as TokenAdminClient;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    vec, Address, BytesN, Env, String,
};

struct TestSetup<'a> {
    env: Env,
    client: AhjoorPaymentsContractClient<'a>,
    admin: Address,
    fee_recipient: Address,
    token_addr: Address,
    token_client: TokenClient<'a>,
    token_admin_client: TokenAdminClient<'a>,
}

fn setup<'a>() -> TestSetup<'a> {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(AhjoorPaymentsContract, ());
    let client = AhjoorPaymentsContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(admin.clone())
        .address();
    let token_client = TokenClient::new(&env, &token_addr);
    let token_admin_client = TokenAdminClient::new(&env, &token_addr);

    TestSetup {
        env,
        client,
        admin,
        fee_recipient,
        token_addr,
        token_client,
        token_admin_client,
    }
}

impl<'a> TestSetup<'a> {
    fn init(&self) {
        self.client.initialize(&self.admin, &self.fee_recipient, &0);
    }
}

#[test]
fn test_submit_dispute_evidence() {
    let s = setup();
    s.init();

    let customer = Address::generate(&s.env);
    let merchant = Address::generate(&s.env);
    s.token_admin_client.mint(&customer, &1000);

    let payment_id = s.client.create_payment(
        &customer,
        &merchant,
        &100,
        &s.token_addr,
    );

    // Dispute the payment
    s.client.dispute_payment(&customer, &payment_id, &String::from_str(&s.env, "defective"));

    // Submit evidence
    let evidence_hash = BytesN::<32>::from_array(&s.env, &[1u8; 32]);
    s.client.submit_dispute_evidence(
        &payment_id,
        &evidence_hash,
        &soroban_sdk::Symbol::new(&s.env, "ipfs"),
    );

    // Get evidence record
    let evidence = s.client.get_dispute_evidence(&payment_id);
    assert_eq!(evidence.payment_id, payment_id);
    assert_eq!(evidence.customer_evidence.len(), 1);
}

#[test]
fn test_evidence_limit_reached() {
    let s = setup();
    s.init();

    let customer = Address::generate(&s.env);
    let merchant = Address::generate(&s.env);
    s.token_admin_client.mint(&customer, &1000);

    let payment_id = s.client.create_payment(
        &customer,
        &merchant,
        &100,
        &s.token_addr,
    );

    s.client.dispute_payment(&customer, &payment_id, &String::from_str(&s.env, "defective"));

    // Submit max evidence
    for i in 0..5 {
        let evidence_hash = BytesN::<32>::from_array(&s.env, &[i as u8; 32]);
        s.client.submit_dispute_evidence(
            &payment_id,
            &evidence_hash,
            &soroban_sdk::Symbol::new(&s.env, "ipfs"),
        );
    }

    // Try to submit one more (should fail)
    let evidence_hash = BytesN::<32>::from_array(&s.env, &[99u8; 32]);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        s.client.submit_dispute_evidence(
            &payment_id,
            &evidence_hash,
            &soroban_sdk::Symbol::new(&s.env, "ipfs"),
        );
    }));
    assert!(result.is_err());
}
