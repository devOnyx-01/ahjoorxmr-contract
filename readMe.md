# Ahjoor - Decentralized ROSCA Platform

**Ahjoor** is a decentralized Rotating Savings and Credit Association (ROSCA) platform built on the Stellar blockchain. It empowers communities and savings groups to pool funds and take turns receiving the collective pot with complete transparency, security, and no middlemen.

## Overview

ROSCAs are one of the oldest and most widely used savings systems in the world, yet they still rely entirely on trust and manual processes. Ahjoor brings this tradition on-chain using Stellar's fast, low-cost blockchain infrastructure to provide:

- **Trustless Savings Circles**: Automated round management with no central authority
- **Transparent Operations**: All participants can verify contributions and payouts on-chain
- **Secure Funds**: Cryptographically secured group wallets and contribution records
- **Cost-Effective**: Built on Stellar's efficient, low-fee blockchain infrastructure
- **Scalable**: Designed to support many groups running simultaneously

## Use Cases

- Community savings circles (Ajo, Esusu, Susu, Tanda, Chit Funds)
- Corporate employee savings programs
- Diaspora remittance and group savings
- Micro-lending and credit-building for underbanked communities
- Multi-party escrow and collective fund management

## Quick Start

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [Stellar CLI](https://developers.stellar.org/docs/build/smart-contracts/getting-started/setup)
- Make (optional, for convenience commands)

### Installation

```bash
# Fork the repository
# Then clone your fork into your local environment
git clone https://github.com/Ahjoor/ahjoor-contract.git
cd ahjoor-contracts

# Add wasm32 target
rustup target add wasm32-unknown-unknown
```

### Build

```bash
# Using Make
make build
```

OR

```bash
# Using cargo
cargo build --target wasm32-unknown-unknown --release
```

```bash
# Or directly with Stellar CLI
stellar contract build
```

### Test

```bash
# Run all tests
make test
```

### Coverage

```bash
# Install once
cargo install cargo-llvm-cov --locked

# Enforce thresholds (line >= 90%, branch/region >= 85%)
make coverage
```

### Format & Lint

```bash
# Format code
make fmt
```

```bash
# OR
cargo fmt
```

```bash
# Check formatting
make fmt-check
```

```bash
# OR
cargo check --all
```

```bash
# Run clippy lints
make lint
```

## Architecture

Ahjoor's smart contracts handle:

- **Group Management**: Create and manage multiple independent ROSCA groups
- **Access Control**: Only pre-registered participants can contribute to a group
- **Contribution Tracking**: Immutable record of who has paid each round
- **Automated Payouts**: Scheduled recipients claim the full pot when their round is due
- **Round Progression**: Time-based round advancement using Stellar ledger timestamps

## State Archival & TTL

Stellar/Soroban utilizes State Archival to manage network storage footprint. Idle contracts and data entries will eventually be archived. Ahjoor handles state preservation automatically when members interact with it (e.g. `init` or `contribute`). However, if the contract goes unused for a long period, participants should occasionally call the `bump_storage()` function to manually extend the time-to-live (TTL) of the contract's instance storage and avoid sudden state archival.

## Technology Stack

- **Blockchain**: Stellar (Soroban smart contracts)
- **Language**: Rust
- **SDK**: Soroban SDK v21.0.0
- **Token Standard**: SEP-41 / Stellar Asset Contract (XLM or any compatible token)
- **Testing**: Soroban test utilities

## FAQ

**Q: What happens if I miss a contribution round?**

A: You receive a penalty. After `max_defaults` consecutive missed rounds you are suspended from the group.

**Q: Can I pay my contribution in parts?**

A: Yes. The contract supports partial installments — contribute any amount up to the remaining balance and the round tracks your cumulative total.

**Q: What tokens are accepted?**

A: Only tokens on the admin-controlled whitelist (`ahjoor-token-whitelist`). XLM and any SEP-41 compatible token can be whitelisted.

**Q: What is the maximum protocol fee?**

A: 5% (500 basis points), hard-capped and enforced on-chain. Admins cannot set a higher fee.

**Q: What happens if a dispute arbiter goes inactive?**

A: After the configured timeout (default 7 days), anyone can call `enforce_dispute_timeout(escrow_id)` to release funds to the pre-configured default winner.

**Q: How do I prevent my contract state from being archived?**

A: Call `bump_storage()` periodically (recommended every ~30 days of inactivity). If archival does occur, state can be restored via `stellar contract restore`.

## Resources

- [Stellar Documentation](https://developers.stellar.org/)
- [Soroban Smart Contracts](https://soroban.stellar.org/)
- [Stellar CLI Reference](https://developers.stellar.org/docs/tools/developer-tools)

  ## Community

- [Telegram Group Chat](https://t.me/ahjoor)
