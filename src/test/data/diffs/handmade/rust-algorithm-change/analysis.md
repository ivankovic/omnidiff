# Accuracy analysis: rust-algorithm-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 44 (30 visible) of 368 nodes, 371 graded, N:M floor 3 visible |
| Painting, Minimal | 159 bytes wrong of 972: renderer 75, matcher 94 |
| Painting, Full | 264 bytes wrong of 972: renderer 135, matcher 170 |
| Ground-truth invariant violations | 1 |

## The change

Handmade. `find_duplicate` swaps a brute-force double loop (`for i ... { for j ... { if
nums[i] == nums[j] { return Some(nums[i]); } } }`, lines 3-9) for a `HashSet` single loop (`for
&num in nums { if seen.contains(&num) { return Some(num); } seen.insert(num); }`, after lines
7-12), adds `use std::collections::HashSet;` and `let mut seen = HashSet::new();`, and rewrites
the comment. `description.md`: "Requires a N:M multi-map for optimal solution" (both old `for`
loops map to the one new loop).

## Mapping

All mismatches come from `APTED("qualified_name")` (one `UnresolvedNode` leaf inside a deleted
subtree), which aligns the new loop with the inner old loop and then pairs across levels.

- If and return deleted and reinserted (9 visible, 7 invisible), before lines 5-7: the ground
  truth maps `if nums[i] == nums[j] { return Some(nums[i]); }` to `if seen.contains(&num) {
  return Some(num); }` (after lines 8-10). omnidiff deletes the old `if`, its block and the
  `return` statement, and inserts the new ones, because the new `if`'s condition was already
  consumed by the next group. `matcher`.
- Condition paired with a loop range (10 visible, 6 invisible): the inner loop's range end
  `nums.len()` (before line 4) is mapped to the new `if` condition `seen.contains(&num)` (after
  line 8), leaf by leaf (`nums`->`seen`, `len`->`contains`, both brackets). The ground truth has
  both as replaced. A cross-level pairing by tree edit cost. `matcher`.
- `len` -> `new` (2 visible): the outer loop's `nums.len()` field (before line 3) is mapped to
  `HashSet::new` (after line 5). `matcher`.
- The loop group (5 visible, 1 invisible, N:M floor 3 visible): the ground truth maps both old
  `for` keywords, both `{` and both `}` to the one new loop's. omnidiff keeps the inner loop's
  `for` and deletes the outer one's, which a one-to-one mapping must do with one of them; but it
  also deletes both old `{` and inserts the new one, and deletes the outer `}`. 3 are the floor
  (`nm-floor`), 2 are the matcher's (`matcher`).
- `in` (2 visible): omnidiff maps the inner loop's `in` (before line 4) to the new loop's `in`
  (after line 7), consistent with mapping that loop's `for`; the ground truth maps the `for`
  keywords but deletes and inserts the headers between them. Both are defensible. `ambiguous`.
- `//` (2 visible): before line 2 and after line 4. The ground truth matches the two
  `line_comment` nodes (`match_but_not_identical`) but deletes and inserts their only child `//`;
  omnidiff maps the comments the same way and keeps `//` identical, which is the self-consistent
  reading of a matched comment. `ground-truth`.

## Painting

Matcher-caused (Minimal 94 bytes, Full 170): the rendering of the groups above: the old `if` /
`return` painted `Delete` and the new ones `Insert` where the painting has them moved (`Delete
-> Move`, `Insert -> Move`), `nums.len()` / `seen.contains(` and `len()` / `new();` painted
`Update` where the painting has `Delete` / `Insert`, and the old and new loop braces.

Renderer-caused (present with the human mapping too):

- Both comments, `None -> Delete` / `None -> Insert` (Minimal 65 bytes, Full 73): the changed
  comment lines (before line 2, after line 4) are not painted at all under either mapping, while
  the painting deletes the old comment and inserts the new one. The comment text is not a leaf in
  the tree (`line_comment` has only the `//` child), so nothing paints it. `renderer`.
- Minimal, `None -> Move`: the moved `if` keyword on before line 5 (2 bytes) is painted on the
  after side only; the two `)` of `Some(...)` (2 bytes) are dropped as lone punctuation.
  `renderer`.
- Minimal, `None -> Update`, `num` on before line 6 and after line 9 (6 bytes): the painting
  paints `nums` -> `num` as an update of the whole name, against its own Minimal convention (only
  the differing characters); invariant [16] flags the after side. `ground-truth`.
- Full, the remaining ~62 bytes: indentation and single spaces around the moved and collapsed
  loop lines, which the Full painting accounts for byte by byte (`Delete` for the removed level of
  indentation, `Move` for the rest) and omnidiff paints `Move` or leaves unpainted. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `nm-floor`, `ground-truth`, `ambiguous`
- Where a fix would go: APTED alignment when two nested loops collapse into one
  (`APTED("qualified_name")`: keep the `if` with the `if`, not the condition with a loop range);
  renderer: painting of changed `line_comment` text in `src/diff/text.rs`
- Ground truth questionable: yes: the `line_comment` nodes are matched while their `//` children
  are deleted/inserted (the painting deletes/inserts the whole comments); Minimal paints `num`
  against `nums` whole (invariant [16])
