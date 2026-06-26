# fix: enforce reveal_until deadline and settle guard with typed errors

## Summary

`reveal_slot_bid` contained a `commit_until` guard but its `reveal_until` guard used `panic!("Reveal phase has closed")` — a plain string panic rather than a typed contract error. `settle_sealed_slot_auction` similarly used `panic!("Reveal phase is still open")`. Both paths should surface `ExtError2::AuctionWindowClosed = 103` so callers can distinguish the error programmatically.

A bidder who missed the reveal window could call `reveal_slot_bid` and, if the guard were absent or mis-coded, inject a valid bid into `DataKey3::SealedRevealedBids(round)` retroactively and influence `settle_sealed_slot_auction` outcome.

## Changes

### `contracts/ahjoor-rosca/src/lib.rs`

| Location | Before | After |
|---|---|---|
| `reveal_slot_bid` — post-reveal window | `panic!("Reveal phase has closed")` | `panic_with_error!(&env, ExtError2::AuctionWindowClosed)` |
| `settle_sealed_slot_auction` — pre-settle guard | `panic!("Reveal phase is still open")` | `panic_with_error!(&env, ExtError2::AuctionWindowClosed)` |

Both changes are one-liner replacements with no logic change — the guard conditions (`now > state.reveal_until` and `timestamp <= state.reveal_until`) were already correct.

### `contracts/ahjoor-rosca/src/test_sealed_slot_auction.rs`

Three new tests added under the `// ── #392` section:

| Test | What it verifies |
|---|---|
| `test_late_reveal_rejected` | `reveal_slot_bid` after `reveal_until` returns an error (AuctionWindowClosed) |
| `test_early_settle_rejected` | `settle_sealed_slot_auction` called inside the reveal window returns an error |
| `test_reveal_within_window_accepted` | A reveal within the window succeeds and appears in `SealedRevealedBids` |

## Acceptance Criteria Coverage

- [x] `reveal_slot_bid` called after `reveal_until` returns `ExtError2::AuctionWindowClosed`
- [x] `settle_sealed_slot_auction` called before `reveal_until` returns an appropriate error
- [x] A bid revealed within the window is accepted and appears in `DataKey3::SealedRevealedBids`
- [x] `test_late_reveal_rejected` added to `test_sealed_slot_auction.rs`
- [x] `test_early_settle_rejected` added to `test_sealed_slot_auction.rs`

closes #392
