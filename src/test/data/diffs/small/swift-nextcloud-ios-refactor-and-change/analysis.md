# Accuracy analysis: swift-nextcloud-ios-refactor-and-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 19 (13 visible) of 7380 nodes, 7380 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `iOSClient/Networking/E2EE/NCNetworkingE2EEUpload.swift`, `upload(...)` gains an
`externalBannerToken` parameter. The "BANNER ENCRYPTION" block (before lines 38-54) moves about 100
lines down (after lines 140-162) and is restructured:

- `bannerToken = showUploadBanner(...)` now sits in the `else` of a new
  `if let externalBannerToken`;
- its trailing closure becomes an `onButtonTap:` argument;
- its `title:`/`subtitle:`/`systemImage:` arguments move into a new `LucidBanner.shared.update(...)`
  call;
- the comment becomes `// BANNER`.

## Mapping

- **Moved and rewrapped assignment not paired (12 visible, 6 invisible), `matcher`.** The ground
  truth maps the before `bannerToken = showUploadBanner(...)` assignment (line 40) to the one in the
  new `else` branch (after line 145), with its skeleton: `bannerToken`, `=`, `showUploadBanner`,
  `(`, the commas, `)`, and the closure's `{`/`}`. omnidiff deletes all of these
  (`APTED("qualified_name")`, op Delete). The unchanged pieces inside, such as the closure's
  `if let` statements and the `NSLocalizedString(...)` arguments, are mapped (they are not
  mismatched). So omnidiff finds the identical subtrees that moved, but not the changed statement
  that contains them. That statement changed position, depth (inside `if ... else`) and argument
  list all at once, and the APTED run over the function only pairs nodes in a compatible order and
  depth.
- **Comment `// BANNER ENCRYPTION` -> `// BANNER` deleted (1 visible), `matcher`.** It moved with
  the block. The ground truth updates it. omnidiff deletes it and inserts the new one.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: a moved-and-modified statement pass after `APTED("qualified_name")`: pair a
  deleted statement with an inserted one when most of its moved children landed inside it (bottom-up
  propagation across a move)
- Ground truth questionable: no
