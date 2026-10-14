# Accuracy analysis: python-ansible-ansible-ridiculously-long-yaml-in-string-constant-and-actual-code-changes

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 5 (4 visible) of 21306 nodes, 21306 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

In `lib/ansible/plugins/connection/ssh.py`, the commit adds `private_key` and
`private_key_passphrase` options to the long YAML `DOCUMENTATION` string, adds imports, and adds a
new `_populate_agent` method. In `_build_command`, the old pair

```
key = self.get_option('private_key_file')
if key:
    <IdentityFile args>
```

(before lines 800-803) becomes `if self.get_option('private_key'): try: key =
self._populate_agent() ...` followed by `elif key := self.get_option('private_key_file'):`, which
holds the old body (after lines 878-890).

## Mapping

- **`key` assignment folded into the walrus (3 visible), `matcher`.** The ground truth has a
  multi-map group (2 before `key`s to 1 after) for the walrus target `key :=` (after line 888). The
  `key` of before line 800 (or line 801's condition) should pair with it, and the new
  `key = self._populate_agent()` (after line 880) is new. omnidiff maps before line 800's `key` to
  after line 880's `key` (`APTED("qualified_name")`, Identical). The old assignment statement and
  the new one share the shape `key = <call>`, so the edit distance pairs their targets. Nothing in
  the group is matched. The floor is 0, so a one-to-one mapping could satisfy this.
- **Old `if` deleted (1 visible, 1 invisible), `matcher`.** The ground truth keeps the before
  `if key:` statement as the after `if self.get_option('private_key'):` (identical `if` keyword).
  The old body ends up in the new `elif`. omnidiff deletes the before `if_statement` and its `if`
  (`APTED("qualified_name")`, op Delete). With the body gone to the `elif`, pairing the outer `if`
  is a judgement call, and the ground truth's choice is reasonable: it keeps the identical keyword.

The long YAML string change produces no mismatches.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: `APTED("qualified_name")` (src/diff/solve_syntax_aware_matching.rs): an
  assignment whose right-hand side moved into a walrus (`named_expression`) should take its target
  along
- Ground truth questionable: no
