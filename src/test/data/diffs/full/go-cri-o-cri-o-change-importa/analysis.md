# Accuracy analysis: go-cri-o-cri-o-change-importa

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 10 (6 visible) of 1120 nodes, 1120 graded, N:M floor 0 visible |
| Painting | none (fixture has no painting) |
| Ground-truth invariant violations | none |

## The change

The module `github.com/containers/common` was renamed: import line 12
`"github.com/containers/common/pkg/cgroups"` is removed, and `"go.podman.io/common/pkg/cgroups"`
is added on after line 13. Because the import block is sorted, the new path lands after
`libctrcgroups "github.com/opencontainers/cgroups"` instead of before it.

## Mapping

All 10 mismatches (6 visible, 4 invisible) are this one import spec: the `import_spec`, its
`interpreted_string_literal`, both `"` and the string content, on each side. The ground truth
deletes before line 12 and inserts after line 13. Omnidiff pairs them: the string content is an
Update, and the spec moves one place down. All 10 have reason `StructurallyIdenticalAncestor`
(Update for the string content, Identical for the quotes, MatchButNotIdentical for the spec and the
literal): they were mapped as descendants of a pair that the structural-hash pass matched by shape.
I did not trace which ancestor that pair is.

Both readings are defensible. The import binds the same package name `cgroups`, which the rest
of the file still uses. The path keeps its `common/pkg/cgroups` tail. So "same import, new
path" is a reasonable reading. Equally, a different module path is a different dependency, and
the line moved, so delete+insert is just as reasonable. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began
printing them.

## Verdict

- Primary cause: `ambiguous`
- Other causes: none
- Where a fix would go: none: ambiguous
- Ground truth questionable: no
