# Accuracy analysis: java-defects4j-cli-38-defaultparser

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 11 (8 visible) of 6290 nodes, 6301 graded, N:M floor 8 visible |
| Painting, Minimal | 57 bytes wrong of 41316: renderer 36, matcher 82 |
| Painting, Full | 78 bytes wrong of 41316: renderer 40, matcher 71 |
| Ground-truth invariant violations | none |

## The change

Defects4J Cli-38 in `DefaultParser.isShortOption`: `return options.hasShortOption(optName);` (before
line 310) becomes `if (options.hasShortOption(optName)) { return true; }` (after lines 310-313)
followed by a new fallback `return optName.length() > 0 &&
options.hasShortOption(String.valueOf(optName.charAt(0)));` (after line 315) for concatenated
short options.

## Mapping

- **The old `return` split in two (8 visible, 3 invisible).** The ground truth maps the old
  statement 1:2: its `options.hasShortOption(optName)` to the new `if` condition (after line
  310) and to the fallback's call, and its `return` / `;` to both `return true;` (after line 312)
  and the fallback `return` (after line 315), as all-to-all groups. omnidiff, one-to-one, maps it
  to the fallback `return` only and inserts the condition call and `return true;`. Every visible
  mismatch is in these groups; the N:M floor is 8 visible, exactly the count. `nm-floor`.

## Painting

- N:M-caused, Minimal 55 / Full about 77 bytes (absent with the human mapping): the new condition
  `options.hasShortOption(optName)` and `return ` / `;` of `return true;` painted Insert where the
  paintings paint them `Move` (copies of the old statement), and the old and fallback `return`
  and `);` left unpainted where the paintings paint `Move`. `nm-floor`.
- Renderer-caused (also with the human mapping), Minimal 2 / Full 1 byte: `{` and `}` of the new
  `if` block (after lines 311, 313), which the Minimal painting paints, and a gap space on after
  line 315 under Full. `renderer`.

## Verdict

- Primary cause: `nm-floor`
- Other causes: `renderer`
- Where a fix would go: N:M support (`solve_multi_maps`) for a `return` split into a guarded
  `return` and a fallback `return`
- Ground truth questionable: no
