# Accuracy analysis: lua-luakit-luakit-actual-test-change-merging-two-tests-into-one

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 107 (70 visible) of 1527 nodes, 1527 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`tests/async/test_undoclose.lua` has two tests, `T.test_undo_close_works` (before lines 17-54) and
`T.test_undo_close_restores_tab_history` (before lines 56-83). The commit merges them into one test
under the second name (after lines 17-68):

- The merged body is mostly the first test: the menu checks, close, and undo-close.
- It takes the "navigate to about:blank" step and the tail from the second test: undo-close, check,
  navigate back, restore.
- `assert(a == b)` becomes `assert.is_equal(a, b)`.
- Two `require 'x'` calls get parentheses.

## Mapping

The ground truth pairs the after function with the first test (`assignment_statement:2`) and
deletes the second test's wrapper. It maps second-test statements into the merged body where they
survive. Every mismatch comes from the whole-file residual pass `APTED("fast_fallback")` or from
`MovedSubtree`.

- **Neither before function wrapper is matched (4 visible, 5 invisible), `matcher`.** omnidiff
  deletes the first test's assignment, `function_definition`, `parameters` and `block` (before line
  17), and the second test's `end` (before line 83). The after wrapper is inserted, and its body
  statements are matched one by one as moves. No earlier pass pairs the `T.x = function () ... end`
  assignments: the name matches the second test, and the body mostly matches the first. The residual
  APTED pass then deletes the wrappers.
- **Updated statements of the first test deleted (21 visible, 10 invisible), `matcher`.** Because
  their enclosing block is unmatched, the edited statements are not paired:
  - `assert(w.tabs:current() == 2)` (line 21) and `assert(w.view.uri == "about:blank")` (line 53),
    which become `assert.is_equal(...)`
  - the reworded comments on lines 30, 41 and 67
  - the `()` of `w:undo_close_tab()` (line 69)

  omnidiff deletes them (op Delete, `APTED("fast_fallback")`). The ground truth updates them in
  place.
- **Leaves scattered across unrelated statements (37 visible, 18 invisible), `matcher`.** After
  lines 31-32 (`w.view.uri = "about:blank"` and `test.wait_for_view(w.view)`) are inserted in the
  ground truth, and before lines 47, 61 and 64 are deleted. omnidiff builds after line 31 from
  pieces of two before statements: `w.view.uri` from before line 47 (`assert(w.view.uri == uri)`)
  and `"about:blank"` from before line 64. It takes after line 32 from before line 61. It also maps
  before line 64's `w.view.uri` into after line 48. The pieces of after lines 31-32 (32 visible, 16
  invisible) come from `MovedSubtree`: `w.view.uri`, `"about:blank"` and the whole
  `test.wait_for_view(w.view)` call were left unmapped on both sides by the fallback, and phase 7
  pairs each byte-identical fragment as a move. Only the line-48 piece (5 visible, 2 invisible) is
  `APTED("fast_fallback")`. The ground truth's choice to insert rather than move these
  lines is a judgement call: identical text exists at before lines 63-65. But a clean move would be
  the alternative, not these leaf fragments.
- **`(1)` moved between the two `undo_close_tab` calls (6 visible, 4 invisible), `matcher`.** The
  ground truth keeps statement order: `w:undo_close_tab(1)` (before line 42) -> `w:undo_close_tab()`
  (after line 45) loses the `1`, and `w:undo_close_tab()` (before line 69) -> `w:undo_close_tab(1)`
  (after line 54) gains it. omnidiff moves the whole `(1)` argument list from line 42 to line 54
  (`MovedSubtree`) and deletes the `()` of line 69.
- **`'luassert.spy'` -> `"luassert.spy"` quotes deleted (2 visible), `matcher`.** The ground truth
  updates the `'` delimiters of line 8 to `"`. omnidiff deletes and inserts them
  (`APTED("fast_fallback")`).

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: container matching before `APTED("fast_fallback")`. A pass that pairs the
  merged `T.x = function` assignment with the before function that shares most of its body would
  let the block-level passes run inside it. Also `MovedSubtree`, which should not move an argument
  list between two calls that are both matched in place, nor assemble two new lines from fragments
  of three different before statements (after lines 31-32).
- Ground truth questionable: slightly. The after function's `end` maps to the second test's `end`
  (before line 83), while its `function` keyword and the `function_definition` map to the first
  test's, and the first test's `end` is deleted. The before lines 63-65 to after lines 30-32 step is
  deleted and inserted, not moved, even though the text is identical. Both choices are defensible.
