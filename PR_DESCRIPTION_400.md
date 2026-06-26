# fix: enforce acceptance check in complete_merge and add GroupStatus::Merged guard

## Summary

`accept_merge` previously executed the full merge in a single step — copying members, emitting completion events, and recording `accepted = true` — all without a separate consent gate. This meant any caller who knew a `proposal_id` could call the function before Group B admin had consented, bypassing the two-step approval flow entirely.

This PR fixes the vulnerability by splitting the operation into two distinct functions with proper guards:

1. **`accept_merge`** — consent-only. Sets `proposal.accepted = true` and emits `emit_merge_accepted`. Does no member mutation.
2. **`complete_merge`** — execution step. Verifies `proposal.accepted == true` (panics with `ExtError2::MigrationNotApproved = 108` otherwise), checks `GroupStatus != Merged` to block re-execution, copies members into the group's member/payout lists, sets `GroupStatus::Merged` permanently, and removes the proposal from storage to prevent replay.

## Changes

### `contracts/ahjoor-rosca/src/lib.rs`

- **`accept_merge`**: Stripped down to consent-only step. Removed `new_members` parameter, all member-copying logic, `GroupStatus` and `GroupMergedInto` writes. Now only flips `proposal.accepted = true` and emits `emit_merge_accepted`.
- **`complete_merge`** _(new function)_: Full merge execution. Guards in order:
  1. `GroupStatus::Merged` check → panics `"Group already merged"` (prevents re-execution)
  2. `!proposal.accepted` check → `panic_with_error!(ExtError2::MigrationNotApproved)` (enforces consent)
  3. `paid_members` non-empty check → panics `"Merge only permitted between rounds"`
  4. `combined_count > max_members` check → panics `"Combined member count exceeds max_members"`
  - Appends `new_members` to `Members` and `PayoutOrder`
  - Sets `DataKey2::GroupStatus` → `GroupStatus::Merged`
  - Sets `DataKey2::GroupMergedInto`
  - Removes proposal from `DataKey2::MergeProposals` (replay prevention)
  - Emits `emit_merge_completed` and `emit_group_marked_merged`
- **`get_group_status`** _(new function)_: Public read-only accessor for `GroupStatus`, used by tests.

### `contracts/ahjoor-rosca/src/test_group_split.rs`

Three new tests under the `// ── #400` section:

| Test | What it verifies |
|---|---|
| `test_merge_requires_acceptance` | `complete_merge` on an unaccepted proposal panics |
| `test_complete_merge_double_execution_blocked` | Second `complete_merge` call on an already-merged group panics |
| `test_complete_merge_sets_status_merged` | Successful merge sets `GroupStatus::Merged` on the source group |

## Security Impact

Before this fix, an attacker with knowledge of a pending `proposal_id` could call the old `accept_merge` at any time to immediately merge groups without Group B admin ever consenting. The merged state was irreversible (`GroupStatus::Merged` blocks rollback). This fix closes that bypass: `complete_merge` requires `accepted == true` as a hard prerequisite, and the `GroupStatus::Merged` guard ensures the operation is idempotent.

## Acceptance Criteria Coverage

- [x] `complete_merge` on a proposal with `accepted = false` returns `MigrationNotApproved` (108)
- [x] `complete_merge` called twice on the same accepted proposal errors on second call
- [x] Successful merge sets `GroupStatus::Merged` on the source group
- [x] Test `test_merge_requires_acceptance` added to `test_group_split.rs`

closes #400
