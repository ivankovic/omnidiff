# Accuracy analysis: c-linux-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 11898 nodes, 11898 graded, N:M floor 0 visible |
| Painting, Minimal | 31 bytes wrong of 50090: renderer 31, matcher 0 |
| Painting, Full | 84 bytes wrong of 50090: renderer 84, matcher 0 |
| Ground-truth invariant violations | none |

## The change

linux `arch/riscv/kvm/vcpu_pmu.c`: Spectre hardening. `#include <linux/nospec.h>` is added, four
`cidx = array_index_nospec(cidx, ...);`-style lines are inserted, and three existing index
expressions are wrapped in `array_index_nospec(..., MAX)`: `sbi_event_code` (before line 90 ->
after lines 91-92) and `i + ctr_base` twice (before lines 562, 633 -> after 567-568, 639-640),
each rewrapped onto two lines.

## Painting

The mapping has no mismatches; all runs are the same with the human mapping.

- **Wrapped expressions painted Move, Minimal 29 / Full 84 bytes (Move -> None).** The
  painting leaves an expression that was only wrapped in a new call unpainted (only the new
  `array_index_nospec(` and `, MAX)` are inserted). omnidiff paints the wrapped `sbi_event_code`
  Move in Minimal, and in Full also `i + ctr_base` and the line tails `];` / `;` behind the
  wrapped expressions. `renderer`.
- **Minimal, None -> Insert, 2 bytes:** the `,` after the wrapped `i + ctr_base` (after lines
  567, 639). The painting includes it in the inserted `, RISCV_KVM_MAX_COUNTERS`; Minimal drops a
  lone separator at the end of a line, the rest of the insert being on the next row. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text`: an expression wrapped in a new call (its parent changed,
  its text and order did not) should not paint Move; Minimal punctuation filter for a separator
  that belongs to a multi-row insert
- Ground truth questionable: no
