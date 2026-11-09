# Accuracy analysis: cpp-ollama-ollama-update-commit-hash-7

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 96 nodes, 96 graded, N:M floor 0 visible |
| Painting, Minimal | 2 bytes wrong of 338: renderer 2, matcher 0 |
| Painting, Full | 2 bytes wrong of 338: renderer 2, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`LLAMA_COMMIT` is bumped from `"7f8ef50cce40e3e7e4526a3696cb45658190e69a"` to
`"17f7f4baad8b3a716ee139da7bb56ae984e8c0fa"` (line 2).

## Painting

- **Shared last character (2 bytes in each preset, `None -> Update`, `renderer`).** The two hashes
  happen to end in the same character, `a`. Omnidiff narrows the string Update and leaves that
  character unpainted on both sides. The single painting ("Only one solution") updates the whole
  hash. The runs are the same with the human mapping. This is the "shared suffix inside a word" case
  of text_painting_findings.md rule 3: a coincidental shared character inside one token is not an
  unchanged tail.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: Update narrowing in `src/diff/text.rs`. It should not trim a shared prefix
  or suffix inside a single word or token.
- Ground truth questionable: no
