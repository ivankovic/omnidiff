# Accuracy analysis: c-genymobile-scrcpy-big-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 101 (68 visible) of 348 nodes, 348 graded, N:M floor 0 visible |
| Painting, Minimal | 394 bytes wrong of 1415: renderer 7, matcher 393 |
| Painting, Full | 556 bytes wrong of 1415: renderer 98, matcher 494 |
| Ground-truth invariant violations | none |

## The change

scrcpy `app/src/uhid/uhid_output.c` (25 -> 27 lines): three includes are added;
`sc_uhid_devices_init(devices)` gains a `keyboard` parameter and its body `devices->count = 0;`
becomes `devices->keyboard = keyboard;`; the two receiver functions `..._add_receiver` and
`..._get_receiver` are removed and a new `sc_uhid_devices_process_hid_output` (after lines
15-27) is added.

## Mapping

- **`sc_uhid_devices_init` deleted and re-inserted, 12 visible + 7 invisible.** The ground truth
  keeps the function (same name on both sides) and updates its signature and body. omnidiff
  deletes before lines 5-8 (`void`, name, parentheses, braces, `devices->count = 0;`) with
  `APTED("qualified_name")`; only the unchanged parameter `struct sc_uhid_devices *devices` is
  kept. A same-named function with a changed body should have been paired; the removed
  functions' parts outbid it in the edit distance. `matcher`.
- **The new parameter's type taken from `get_receiver`, 4 visible + 2 invisible.**
  `struct sc_keyboard_uhid` (after line 11) is mapped to `get_receiver`'s
  `struct sc_uhid_devices` (before line 18) by `StructurallyIdenticalSubtrees` /
  `StructurallyIdenticalAncestor` (`solve_hash_descent`: same shape `struct X`, name Update).
  The ground truth inserts it. `matcher`.
- **Removed functions paired into the new one, 52 visible + 24 invisible.** The ground truth
  deletes `add_receiver` and `get_receiver` and inserts `process_hid_output`. `add_receiver`'s
  parameter `struct sc_uhid_devices *devices` is byte-identical to `process_hid_output`'s and is
  paired by `IdenticalHash`; `BottomUpPropagation` lifts that pair to the two declarators; the
  whole-file residual `APTED("fast_fallback")` (`diff.rs` phase 6) then pairs the rest: the
  names (Update), `void`, punctuation, and `get_receiver`'s `uint16_t id`, `for`/`if` scaffolding
  and braces onto the new function's `uint16_t id`, `if (id == ...)` / `if (devices->keyboard)`.
  The bodies share no statements, only punctuation, `devices->` and `id`. `matcher`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Painting

- **Matcher, 387 bytes Minimal / 458 Full:** the mapping above rendered: `init` painted
  Delete/Insert where the painting updates it, the removed and new functions left partly
  unpainted (None -> Delete / None -> Insert) or Update where the painting deletes/inserts them,
  and the parameter `struct sc_..._uhid` painted Move. `matcher`.
- **Renderer, 7 bytes Minimal / 98 Full (also with the human mapping):** the ` = ` of the updated
  assignment and the new `,` after the old parameter, left unpainted (both presets); in Full,
  the unchanged parts of `init` (`(struct sc_uhid_devices *devices`, `devices->`, `}`) that the
  Full painting marks Move after the 4-line shift from the new includes. `renderer`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`
- Where a fix would go: the `qualified_name` APTED call site: pair top-level functions by
  identical name before letting removed functions compete for their parts; and the residual
  `fast_fallback` APTED, which pairs two removed functions' leftovers into one inserted function
  on punctuation alone
- Ground truth questionable: no
