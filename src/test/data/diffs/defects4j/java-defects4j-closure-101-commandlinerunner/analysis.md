# Accuracy analysis: java-defects4j-closure-101-commandlinerunner

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 6853 nodes, 6853 graded, N:M floor 0 visible |
| Painting, Minimal | 3 bytes wrong of 36368: renderer 3, matcher 0 |
| Painting, Full | 18 bytes wrong of 36368: renderer 18, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Closure-101: `CommandLineRunner.createOptions` sets `options.closurePass` from the flag directly:
`if (flags.process_closure_primitives) { options.closurePass = true; }` (before lines 433-435)
becomes `options.closurePass = flags.process_closure_primitives;` (after line 434). The
assignment moves out of the deleted `if`, and the condition moves into its right-hand side.

## Painting

Mapping is exact. Every disagreement survives with the human mapping, so all of it is the
renderer's (matcher 0 bytes).

- Both presets, the assignment's `;` (before line 434, after line 434), None -> Move, 1 byte per
  side: the paintings carry the `;` along with the moved `options.closurePass =`; omnidiff paints
  the moved tokens but leaves their `;` unpainted. `renderer`.
- Minimal, the deleted `if`'s closing `}` (before line 435), None -> Delete, 1 byte: the painting
  paints it Delete together with `if (` and `) {`; omnidiff leaves this bracket unpainted although
  its construct was deleted. `renderer`.
- Full, whitespace, 16 bytes: the indentation of the deleted `if` line and of the moved
  assignment (Delete / Move in the painting), and the space before `=` / before the moved
  condition (Move). omnidiff's Full rendering leaves this gap whitespace unpainted. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `src/diff/text.rs`: painting of punctuation and gap whitespace that
  belongs to a moved or deleted construct.
- Ground truth questionable: no.
