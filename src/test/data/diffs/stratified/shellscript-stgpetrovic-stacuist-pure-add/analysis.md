# Accuracy analysis: shellscript-stgpetrovic-stacuist-pure-add

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 54 nodes, 54 graded, N:M floor 0 visible |
| Painting, Minimal | 67 bytes wrong of 389: renderer 67, matcher 0 |
| Painting, Full | 154 bytes wrong of 389: renderer 154, matcher 0 |
| Ground-truth invariant violations | none |

## The change

`deploy.sh` loses a stray leading space on its comment and `bazel build` lines (both shift one
column left), appends `--strategy=Genrule=local` to the `bazel build` command and adds an `rsync`
line. The mapping is perfect; all disagreement is painting.

## Painting

All runs are present with the human mapping too, so all are the renderer's.

- Minimal, 67 bytes, Move -> None, before line 4: omnidiff paints the unchanged
  `CC="/usr/bin/clang++" BAZEL_CXXOPTS="-std=c++17" bazel build -c opt` as Move because the
  command moved one column left (the leading space was removed). The painting treats a
  whitespace-only dedent as no change. In Minimal the after-side counterpart is not flagged
  (omnidiff leaves it unpainted, as the painting does), so the before and after panels disagree.
  `renderer`.
- Full, 154 bytes, Move -> None: the same 67 bytes on before line 4, plus 87 bytes on after line 4
  (the same text up to and including `//:stacuist_webapp `). `renderer`.

The comment line, which shifted by the same column, is not painted Move, so the treatment is
inconsistent between the two shifted lines.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` move detection: a matched node whose only displacement is a
  column shift from removed leading whitespace (same row index, same parent) should not be painted
  Move; and the Minimal before/after asymmetry for the same pair.
- Ground truth questionable: no
