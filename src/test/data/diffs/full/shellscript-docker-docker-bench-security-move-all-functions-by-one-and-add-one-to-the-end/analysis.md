# Accuracy analysis: shellscript-docker-docker-bench-security-move-all-functions-by-one-and-add-one-to-the-end

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 334 (227 visible) of 19036 nodes, 19036 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`tests/5_container_runtime.sh` gets a new first check (swarm mode, `id="5.1"`), and every
existing `check_5_N` is renumbered to `check_5_{N+1}` with its `local id` bumped. Read by name,
the headers `check_5_1() {` ... `check_5_31() {` stay where they are, each body moves down one
function, and one header, `check_5_32() {`, is added at the end; read by content, every function
is renamed and a new function is added at the top. Two numbers inside bodies also change
(`255` -> `265`, `1024` -> `1025`).

## Mapping

All reasons are `StructurallyIdenticalSubtrees` / `StructurallyIdenticalAncestor`
(`solve_hash_descent`).

- **Function headers, the name reading** (95 visible, 33 invisible). The ground truth maps each
  `function_definition` and its `check_5_N`, `(`, `)` by name, before N -> after N, and its body
  (`compound_statement`) by content to function N+1; the after `check_5_32` header is new and the
  after `check_5_1` body (swarm) is new. Omnidiff pairs whole functions by structure, before
  `check_5_N` -> after `check_5_{N+1}`, so every name is an Update (`check_5_1` -> `check_5_2`),
  the parentheses cross over, and the new function is the first one. Both readings are
  defensible: the fixture's name says the bodies moved, but renumbering functions is the same
  edit seen the other way, and omnidiff's form keeps each function whole. `ambiguous`.
- **Two short functions crossed** (132 visible, 74 invisible). `check_5_22` and `check_5_23`
  (before lines 907-937) have the same shape (16 lines of `local` assignments, `note`,
  `logcheckresult`). The ground truth maps them to after `check_5_23` and `check_5_24`. Omnidiff
  maps before `check_5_22` (the `--privileged` check) to after `check_5_24` (the `--user=root`
  check, line 941) and before `check_5_23` to after `check_5_23` (line 924): the two bodies are
  swapped. Every leaf inside is then paired with the wrong twin, so all strings (`desc`,
  `remediation`, ...) become updates, although an exact copy of each body exists in the other
  function. This is wrong under either reading of the headers. `matcher`: structural pairing
  ignores leaf values when several candidates have the same shape.

## Verdict

- Primary cause: `matcher`
- Other causes: `ambiguous`
- Where a fix would go: `solve_hash_descent` (StructurallyIdenticalSubtrees): when several
  candidate subtrees have the same structure, rank by leaf-value agreement (the crossed bodies
  agree on almost every string with the other candidate).
- Ground truth questionable: no (its header/body split is a deliberate reading, but note that it
  maps a `function_definition` to one after function and its body to the next one)
