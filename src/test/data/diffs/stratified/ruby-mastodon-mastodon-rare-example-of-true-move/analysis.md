# Accuracy analysis: ruby-mastodon-mastodon-rare-example-of-true-move

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 108 nodes, 108 graded, N:M floor 0 visible |
| Painting, Minimal | 146 bytes wrong of 978: renderer 6, matcher 152 |
| Painting, Full | 152 bytes wrong of 978: renderer 0, matcher 152 |
| Ground-truth invariant violations | none |

## The change

Same commit as ruby-mastodon-mastodon-move. In the schema-annotation comment of
`account_warning_preset.rb`, the line `#  title      :string           default(""), not null`
moves from line 11 up to line 9, above `created_at` and `updated_at`. description.md: "A rare
example of a true move".

## Mapping

- Lines 9-11, three `comment` nodes. The ground truth maps each comment to its identical twin
  (title moved). Omnidiff pairs them by position (before line 9 with after line 9, ...), so all
  three are `Update`s (reason `StructurallyIdenticalAncestor`).
- Why: only the order of comments changes, so the trees have the same shape and
  `solve_hash_descent` matches the whole file as one structurally identical subtree, then pairs
  every node inside it by position. The identical comment text elsewhere in the same parent is
  not considered. `matcher`: move not detected.

## Painting

- Matcher, both presets: lines 9-11 on both sides. Omnidiff paints three Updates, narrowed past
  common prefixes (`created_at :datetime`, `upd`, `cre`, ...); the paintings have only the
  `title` line as Move. Minimal 146 bytes, Full 152 bytes (Full includes the `#  ` prefix of the
  moved line). `matcher`.
- Renderer, Minimal, latent (6 bytes): with the human mapping, the Minimal rendering paints the
  `#  ` prefix of the moved comment as Move on both sides; the Minimal painting leaves it out.
  Not part of the 146 real bytes, because omnidiff's own Update rendering skips the prefix.
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: `solve_hash_descent` (StructurallyIdenticalSubtrees/Ancestor): prefer an
  identical reordered sibling over positional pairing of leaves.
- Ground truth questionable: no
