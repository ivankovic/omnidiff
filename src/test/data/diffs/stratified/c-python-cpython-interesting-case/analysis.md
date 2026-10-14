# Accuracy analysis: c-python-cpython-interesting-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 219 (143 visible) of 2118 nodes, 2108 graded, N:M floor 0 visible |
| Painting, Minimal | 1010 bytes wrong of 10680: renderer 321, matcher 1020 |
| Painting, Full | 1204 bytes wrong of 10680: renderer 615, matcher 1136 |
| Ground-truth invariant violations | 3 (one invariant [2], two invariant [16]) |

## The change

`Python/record_functions.c.h` is generated. The commit makes these changes:

- It adds `_PyOpcode_RecordFunction_NOS_GEN_FUNC` (lines 30-41 after), which has an
  `if (PyGen_Check(obj)) { ... }` body.
- It replaces `_PyOpcode_RecordFunction_CALLER_CODE` (lines 55-62 before, last function) with
  `_PyOpcode_RecordFunction_CODE` (lines 68-71 after, last function). The body of the new
  function is only `*recorded_value = (PyObject *)NULL; Py_INCREF(...)`.
- It renumbers the `#define _RECORD_*_INDEX` list: `TOS` and `CALLER_CODE` go, `NOS_GEN_FUNC`
  comes in.
- It changes three entries of `_PyOpcode_RecordFunctionIndices`.
- It shrinks `_PyOpcode_RecordFunctions[8]` to `[7]`.

Marko's `description.md` says that the whitespace around the `#define` numbers cannot be painted
uniquely in Full, and marks that part unsolved.

## Mapping

- **CALLER_CODE paired with NOS_GEN_FUNC instead of CODE (107 visible, 63 invisible).** The ground
  truth reads the last function as a rename. `_PyOpcode_RecordFunction_CALLER_CODE` becomes
  `_PyOpcode_RecordFunction_CODE` in the same position, and the painting deletes only `CALLER_`. The
  body `*recorded_value = (PyObject *)code;` (before line 59) is lifted out of the `if` to line 69,
  with `code` -> `NULL` in a 1:2 all-to-all group. `NOS_GEN_FUNC` is new. omnidiff maps
  `CALLER_CODE` to `NOS_GEN_FUNC`. This starts in `GreedyAnchorBlock`
  (`APTED("greedy_anchor_block")`): it pairs CALLER_CODE's `if` body (before lines 57-61) with
  NOS_GEN_FUNC's `if` body (after lines 34-40), and the two share `*recorded_value = (PyObject
  *)...;` and `Py_INCREF(*recorded_value);`. CODE has no `if`, so it is never a candidate.
  `BottomUpPropagation` then pairs the two `if` statements, the function bodies and the function
  definitions, and the terminal fallback (`APTED("fast_fallback")`) fills in the declarator, the
  `if` condition and the closing `}` by position (the printed reasons on the NOS_GEN_FUNC side,
  after lines 30-41, show exactly this split). The ground truth's reading is supported by the name
  and the position. omnidiff's pairing moves the function three slots up, to an unrelated name, on
  the strength of one shared block shape. The rest of this group follows from that choice:
  NOS_GEN_FUNC's 53 tokens are reported as matched, not inserted, and CALLER_CODE's `if`,
  declaration and braces are matched, not deleted. `matcher`.
- **Renumbered `#define`s deleted and re-inserted (8 visible, 4 invisible).** The ground truth
  matches `#define _RECORD_NOS_INDEX 4` -> `... 2`, `CALLABLE 5 -> 4`, `BOUND_METHOD 6 -> 5` and
  `4OS 7 -> 6` (before lines 67-70 -> after lines 74, 76-78; `#define` and the name identical).
  omnidiff deletes the four and inserts the new ones, with op Delete and reason
  `APTED("fast_fallback")`. This is the terminal fallback (`apted::for_roots_fallback` ->
  `resolve_residual_forest_via_myers_lcs`, called at `diff.rs:354`). It runs Myers on full
  subtree hashes, so a definition whose value changed never pairs as identical. The unequal gap
  around it is then replaced whole. `matcher`.
- **Index-table entries paired by position (12 visible, 4 invisible).**
  `[RETURN_VALUE] = _RECORD_CALLER_CODE_INDEX,` (before line 74) is paired with
  `[SEND_GEN] = _RECORD_NOS_GEN_FUNC_INDEX,` (after line 82), `APTED("fast_fallback")`. Both are
  the third entry, but the opcode keys differ. The ground truth deletes one and inserts the
  other. `matcher`.
- **`_PyOpcode_RecordFunctions` declaration shell lost (16 visible, 5 invisible).** The table's
  entries are matched as the ground truth says, but the declaration around them (`const
  _Py_RecordFuncPtr _PyOpcode_RecordFunctions[8] = {`, the commas, `};`, before lines 106-115)
  is deleted and re-inserted (op Delete, `APTED("fast_fallback")`). The ground truth matches it
  and updates `8` -> `7`. The same name and type should have pinned the shell. Neither
  `BottomUpPropagation` run nor the terminal fallback pairs it. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

Matcher-owned runs (absent when the human mapping is rendered):

- Minimal 510 / Full 406 bytes, the CALLER_CODE pairing. NOS_GEN_FUNC (after lines 30-41) is
  None -> Insert. CALLER_CODE's `if (`, `owner`, `) {`, the `PyCodeObject *code = ...` line and
  `}` (before lines 57-61) are None -> Delete. CODE (after lines 68-71) is Insert -> None.
  `matcher`.
- Minimal 116 / Full 128 bytes, the table shell (before lines 106 and 115, after lines 112 and 120,
  plus commas in Full), Delete/Insert -> None. `matcher`.
- Minimal 83 / Full 99 bytes, the RETURN_VALUE/SEND_GEN entries (before line 74, after line 82),
  None -> Delete/Insert. `matcher`.
- Minimal 8 bytes: the space before each renumbered value. omnidiff deletes or inserts whole
  `#define` lines, and the painting leaves the space unpainted. `matcher`.
- Minimal 4 bytes: ` 2` and ` 3` on the deleted lines 65-66
  (`#define _RECORD_TOS_INDEX 2`, `..._CALLER_CODE_INDEX 3`). omnidiff paints them Delete. The
  painting deletes only `#define <name>` and leaves the value unpainted, while the inserted line 75
  has its ` 3` painted Insert. `ground-truth`.
- Full 22 bytes: the `#define` values and their spaces. omnidiff paints whole lines Delete/Insert.
  The Full painting leaves every value unpainted (invariant [2], the part `description.md` marks
  unsolved). `ground-truth`.

Renderer-owned runs (still there when the human mapping is rendered):

- Both presets, 228 bytes, None -> Move. The painting marks the matched `#define <name>` heads on
  before lines 67-70 and after lines 74, 76-78 as Move, because they are renumbered. The heads are
  identical and in the same relative order, so the renderer paints nothing. `renderer`.
- Minimal 8 bytes, None -> Update: the values `4`-`7` / `2`, `4`-`6`. The ground truth has no
  entry for any `preproc_arg` node, so the human mapping cannot carry their Update to the
  renderer. `grading-artifact`.
- Full 257 bytes, Update -> None: the whole `function_declarator` of CALLER_CODE (before line 55,
  132) and CODE (after line 68, 125), parameter list included, is painted Update. The Full
  painting deletes only `CALLER_`. The renderer over-paints the unchanged parameters. The
  painting itself breaks invariant [16] (a renamed identifier is painted whole in Full).
  `renderer`, with `ground-truth` for the identifier part.
- Minimal 30 / Full 62 bytes, Move -> None: CODE's body (after lines 69-70) under the human
  mapping. These statements are lifted out of CALLER_CODE's `if`, so the renderer paints them
  Move. The painting leaves them unpainted. `renderer`.
- Minimal 23 bytes, small narrowing differences. The first is `_CALLER_C` on before line 55
  (9 bytes): the painting deletes `CALLER_` inside the identifier. The second is `TOS` -> `NOS`
  on before line 73 and after line 81 (4 bytes): the renderer narrows to one character, and the
  painting paints `TOS`/`NOS` whole. The third is the inserted `_GEN_FUNC` infix on after line 90
  (10 bytes). Full 2 bytes: a displaced `,` on before line 83 and after line 90, painted Move.
  `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ground-truth`, `grading-artifact`
- Where a fix would go: `GreedyAnchorBlock` (`diff/solve_greedy_anchor_blocks.rs`). It accepts
  an `if`-body anchor whose only shared content is a `Py_INCREF` line and so overrides a
  same-position, near-same-name function. Also the terminal fallback
  (`resolve_residual_forest_via_myers_lcs`), which replaces renumbered `#define`s and the
  `_PyOpcode_RecordFunctions` shell whole instead of pairing them by name. On the renderer side:
  the Full Update painting of a whole `function_declarator`, and in-order matched nodes that the
  painting marks Move.
- Ground truth questionable: yes. There are 3 invariant violations. Under [2], the Full painting
  leaves the `#define` values unpainted that Minimal paints; `description.md` acknowledges this.
  Under [16], Full paints only `CALLER_` of the renamed `_PyOpcode_RecordFunction_CALLER_CODE` /
  `_CODE` pair. Also, the Minimal painting of the deleted lines 65-66 leaves their values
  unpainted, while the inserted line 75 has its value painted. The tree ground truth lists no
  `preproc_arg` node at all.
