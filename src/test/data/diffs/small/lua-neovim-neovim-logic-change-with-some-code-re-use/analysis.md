# Accuracy analysis: lua-neovim-neovim-logic-change-with-some-code-re-use

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 15 (13 visible) of 19702 nodes, 19711 graded, N:M floor 7 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `runtime/lua/vim/lsp/client.lua` the commit reworks how dynamic registration is checked:

- The static-registration loop (line 618) iterates `_method_supports_static_registration` and looks
  up `capability` inside the body. `unpack(capability)` becomes `capability[1]`, and
  `registerOptions = { documentSelector = cap.documentSelector }` becomes `cap or {}`.
- `_supports_registration` (lines 939-949) goes through a new `_registration_provider`.
- A new `Client:_provider_value_get` is appended.

description.md: "Requires N:M mapping".

## Mapping

- **N:M groups (7 visible, 2 invisible), `nm-floor`.** The ground truth has all-to-all groups:
  - after line 619's `lsp.protocol.` prefix (`lsp`, `.`, `protocol`, `.` and the two
    `dot_index_expression`s)
  - the `[`/`]` of after line 937
  - `capability_path` at before line 941

  omnidiff inserts or deletes them (`APTED("qualified_name")`; `UnresolvedNode` for
  `capability_path`). A one-to-one mapping cannot satisfy these entries, and they make up the
  whole floor of 7.
- **Re-used identifiers renamed in place or deleted instead of moved (6 visible), `matcher`.** All
  come from `APTED("qualified_name")`:
  - `_request_name_to_server_capability` leaves the `for` header (before line 618) for the new
    `local capability = ...[method]` (after line 619). omnidiff renames it in place to
    `_method_supports_static_registration`.
  - Before line 939's `[method]` index moves into the new `_methods_with_no_registration_options
    [method]` check (after line 937). omnidiff renames it in place to `provider` (after line 941).
  - The loop variable `capability` (before line 618) becomes the `local capability` of after line
    619, and the `cap` of `cap.documentSelector` (before line 629) becomes the `cap` of
    `cap or {}` (after line 628). omnidiff deletes both.

  Ordered tree edit distance cannot move a leaf to a statement in another position, so it renames
  a same-position leaf or gives up. The ground truth follows the identical text, which matches the
  fixture's "code re-use" point.

## Verdict

- Primary cause: `nm-floor` (7 of 13 visible)
- Other causes: `matcher`
- Where a fix would go: for the matcher share, a leaf-level move step after
  `APTED("qualified_name")` that re-pairs deleted/renamed identifiers with identical inserted
  identifiers in the same function. The N:M share needs no fix.
- Ground truth questionable: no
