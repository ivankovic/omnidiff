# Accuracy analysis: ruby-mastodon-mastodon-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 296 nodes, 296 graded, N:M floor 0 visible |
| Painting, Minimal | 203 bytes wrong of 1562: renderer 6, matcher 209 |
| Painting, Full | 209 bytes wrong of 1562: renderer 0, matcher 209 |
| Ground-truth invariant violations | none |

## The change

In the schema-annotation comment block of `follow_recommendation.rb`, the line
`#  account_id :bigint(8)        primary key` moves from first (line 7) to last (line 9), below
`rank` and `reason`. Nothing else changes.

## Mapping

- Lines 7-9, three `comment` nodes. The ground truth maps each comment to its identical twin
  (account_id moved, rank and reason kept). Omnidiff pairs them by position: before line 7 with
  after line 7, and so on, so all three are `Update`s (reason `StructurallyIdenticalAncestor`).
- Why: the only change is the order of three comments, so the two trees have the same shape and
  `solve_hash_descent` matches the whole file as a structurally identical subtree. Inside it every
  node is paired by position, and a leaf with different text becomes an update. The identical
  comment text on the other side is not considered. `matcher`: a move inside a structurally
  identical subtree is not detected.

## Painting

- Matcher, both presets: lines 7-9 on both sides. Omnidiff paints three Updates (narrowed to the
  text after the common prefix, e.g. `ank ...` vs `eason ...`); the paintings have only the
  `account_id` line as Move and the other two lines unpainted. Minimal 203 bytes, Full 209 bytes
  (Full also counts the `#  ` prefix of the moved line, which the Full painting includes).
  `matcher`.
- Renderer, Minimal, latent (6 bytes): with the human mapping, the Minimal rendering paints the
  `#  ` prefix of the moved comment as Move on both sides; the Minimal painting leaves it out.
  Omnidiff's own rendering does not show this run (it narrows its Update past the prefix), so it
  is not part of the 203 real bytes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `solve_hash_descent` (StructurallyIdenticalSubtrees/Ancestor): before
  pairing leaves by position, prefer an identical leaf elsewhere in the same parent (reordered
  siblings).
- Ground truth questionable: no
