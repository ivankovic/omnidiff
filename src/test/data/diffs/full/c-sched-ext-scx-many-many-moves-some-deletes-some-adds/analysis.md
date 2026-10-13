# Accuracy analysis: c-sched-ext-scx-many-many-moves-some-deletes-some-adds

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 3 (3 visible) of 5160 nodes, 5160 graded, N:M floor 0 visible |
| Painting | no painting |
| Ground-truth invariant violations | none |

## The change

sched-ext `scx_lavd/src/bpf/lavd.bpf.h`: the fields of `struct cpu_ctx` are reordered into
cacheline groups, some fields are added or removed, and section comments are edited
(`description.md`: "Excellent test case to demonstrate why move tracking is important."). For
example, `volatile u16 lat_cri; /* latency criticality */` moves from before line 255 to after
line 222, and `online_clk`/`offline_clk` move ahead of the core-compaction section.

## Mapping

All three are before-side comments inside the `field_declaration_list` of `struct cpu_ctx` that
omnidiff deletes (op Delete, reason `APTED("large_flat_subtree")`, `solve_large_flat_subtrees.rs`,
which diffs the large flat field list through APTED's Myers fast path).

- Line 255, `/* latency criticality */`: the trailing comment of the `lat_cri` field. The ground
  truth maps it Identical to the same comment on after line 222. The field itself is mapped as the
  ground truth expects (no mismatch). The comment is a sibling of the field in the list, not part of
  its subtree, so it does not travel with the field's move. It moved up past many reordered
  siblings, out of order for the in-order alignment of the flat list, and was deleted. `matcher`.
- Lines 263 and 290, the section comments `/* Fields for core compaction * */` and
  `/* Temporary cpu masks */`: the ground truth updates them to `... (read-only)` (after lines
  254 and 266). Their text changed, so the hash alignment of the flat list cannot anchor them. They
  sit in gaps of a heavily reordered list (`online_clk`/`offline_clk` now come before the
  compaction comment), and the gaps were replaced rather than paired. `matcher`.

Not traced further: which gap each comment fell into and why no similarity pairing was tried.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `solve_large_flat_subtrees.rs` / APTED's flat Myers path: carry a field's
  trailing same-row comment with the field when it moves, and pair edited comments within a gap by
  similarity
- Ground truth questionable: no
