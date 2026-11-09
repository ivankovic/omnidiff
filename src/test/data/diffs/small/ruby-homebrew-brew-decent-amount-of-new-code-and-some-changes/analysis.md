# Accuracy analysis: ruby-homebrew-brew-decent-amount-of-new-code-and-some-changes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 24 (17 visible) of 6229 nodes, 6229 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`cmd/deps.rb` is made `typed: strict`: a `DepsCombineMode` enum and many `sig { ... }` blocks
are added. `deps_for_dependents` changes from `(dependents, recursive: false, &block)` to
`(dependents, deps_combine_mode:, recursive:)`, its caller computes the mode first, and several
`|d| ... d` blocks become `{ ... it ... }`.

## Mapping

All 24 mismatches come from APTED inside method bodies (reason `APTED("qualified_name")`, the
call that `solve_qualified_name_groups` makes per matched method). Three places:

- **Caller, before line 199 -> after lines 213-214** (6 visible, 3 invisible).
  `deps_for_dependents(dependents, recursive:, &(args.union? ? :| : :&))` becomes
  `deps_for_dependents(dependents, deps_combine_mode:, recursive:)`. The ground truth keeps
  `recursive:` as `recursive:` (after column 73) and inserts `deps_combine_mode:`. Omnidiff pairs
  the before `recursive:` pair with the new `deps_combine_mode:` pair by position (Update of the
  `hash_key_symbol`). The ground truth also maps the conditional `args.union? ? :| : :&` to the
  new conditional on after line 213; omnidiff deletes it, so its `?` and `:` show as deleted.
  `matcher`: positional pairing in APTED where an argument was inserted in front, and a move into
  a new statement not detected.
- **Signature, before line 261 -> after line 293** (5 visible, 2 invisible). The ground truth maps
  `recursive: false` to the after `recursive:` (second keyword parameter), with
  `deps_combine_mode:` and the removal of `&block` as insert and delete. Omnidiff maps
  `recursive: false` to `deps_combine_mode:` and renames the `block` identifier of `&block` to
  `recursive` (Update). Same positional pairing as above. `matcher`.
- **`dot_code`, before lines 284-286 -> after line 320** (6 visible, 2 invisible). The
  `do |d| graph_deps(d, dep_graph:, recursive:) end` block becomes
  `{ graph_deps(it, dep_graph:, recursive:) }`. The ground truth maps the inner call, its
  argument list, `graph_deps`, the brackets, commas and `d -> it`. Omnidiff leaves the before call,
  its `argument_list`, `graph_deps`, `(`, `d`, both `,` and `)` unmapped (Delete). The
  `dep_graph:` and `recursive:` pairs inside it are not in the mismatch list, so they are mapped
  as expected. Why APTED gives up the call while keeping its arguments is undetermined from the
  evidence; the change of container kinds (`do_block`/`body_statement` -> `block`/`block_body`)
  is the only structural difference at this place. `matcher`.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: APTED call in `solve_qualified_name_groups`
  (src/diff/solve_syntax_aware_matching.rs): positional pairing of keyword arguments and
  parameters when one is inserted in front (identical `recursive:` text is ignored for a
  position-based rename); a `do ... end` -> `{ }` block conversion that drops the inner call.
- Ground truth questionable: no
