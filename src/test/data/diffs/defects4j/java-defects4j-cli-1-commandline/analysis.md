# Accuracy analysis: java-defects4j-cli-1-commandline

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 106 (72 visible) of 1877 nodes, 1877 graded, N:M floor 0 visible |
| Painting, Minimal | 522 bytes wrong of 16954: renderer 40, matcher 504 |
| Painting, Full | 611 bytes wrong of 16954: renderer 90, matcher 545 |
| Ground-truth invariant violations | 8 |

## The change

Defects4J Cli-1: `CommandLine` replaces its three maps (`options`, `names`, `hashcodeMap`) with
one `Set options`. Imports change (`HashMap`, `Map` out; `Set`, `HashSet` in), lookups go through
a new `resolveOption(String)` that strips hyphens and scans the set, and `hasOption`,
`getOptionObject`, `getOptionValues`, `addOption`, `iterator` and `getOptions` are rewritten to
use it. The best-matching paintings are `Minimal (inner)` and `Full (inner)`.

## Mapping

Three passes are involved: `APTED("fast_fallback")` (the imports), the kind-only hash descent
(`StructurallyIdenticalSubtrees` / `StructurallyIdenticalAncestor`, the field pairing) and
`APTED("qualified_name")` (everything else).

- **Imports (28 visible, 12 invisible).** Before lines 19 and 23, after lines 22 and 23. The ground
  truth deletes `import java.util.HashMap;` and `import java.util.Map;` and inserts
  `import java.util.Set;` and `import java.util.HashSet;`. omnidiff pairs them by position,
  `HashMap` -> `Set` and `Map` -> `HashSet`, as renamed imports, under `APTED("fast_fallback")` (phase 6,
  `apted::for_roots_fallback` in `src/diff.rs`). The shape pass skips imports on purpose
  (`nodes::is_import_kind`), so the leftover imports reach the fallback, which pairs them in
  order. Different classes imported is a delete and an insert, and even as renames the pairing
  is crossed. `matcher`.
- **Field declarations (18 visible, 10 invisible).** The ground truth maps `private Map options =
  new HashMap();` (before line 46) to `private Set options = new HashSet();` (after line 46) and
  deletes `private Map hashcodeMap = new HashMap();` (before line 51). omnidiff deletes line 46
  and pairs line 51 with after line 46 instead, renaming `hashcodeMap` to `options`: the less
  similar twin. The pairing is not APTED's. The kind-only hash descent (phase 1,
  `src/diff/solve_hash_descent.rs`) pairs line 51 with after line 46 by shape
  (`StructurallyIdenticalSubtrees` on the declaration, `StructurallyIdenticalAncestor` below it; 9
  visible, 5 invisible). Before lines 46, 47 and 51 have the same shape and size, and
  `reference_nodes_ordered` (`src/code/metadata.rs`) lists equal-size nodes in reverse document
  order (a stack DFS, then a stable sort by size), so line 51 claims the one free after
  candidate first. `APTED("qualified_name")` then deletes the left-over line 46 (9 visible, 5
  invisible). `matcher`.
- **`getOptionObject`'s guard, the `if` (3 visible, 2 invisible).** Before line 93 `if
  (!options.containsKey(opt))` becomes `Option option = resolveOption(opt);` + `if (option ==
  null)` (after lines 91-92). The ground truth keeps the `if`; omnidiff deletes `if` and the
  condition's parentheses and inserts the new `if`. `matcher`.
- **`getOptionObject`'s guard, the `(opt)` arguments (6 visible, 2 invisible).** The ground
  truth's tree deletes `(opt)` of `containsKey(opt)` and inserts `(opt)` of `resolveOption(opt)`;
  omnidiff pairs them. Every one of the four paintings paints this `opt` as a `Move` from before
  line 93 to after line 91, which `invariants.txt` reports as 8 violations of invariant 9 (a
  `Move` painted on nodes the tree reads Delete/Insert). omnidiff agrees with the paintings and
  disagrees with the tree here. `ground-truth`.
- **`getType()` receiver (2 visible, 2 invisible).** Before line 98 `((Option)
  options.get(opt)).getType()` -> after line 97 `option.getType()`. omnidiff pairs the cast's
  type `Option` with the variable `option` (a `type_identifier` with an `identifier`); the
  ground truth deletes one and inserts the other. `matcher`.
- **`getOptionValues` header (10 visible, 2 invisible).** The ground truth inserts `Option key =
  resolveOption( opt );` (after line 148) whole. omnidiff assembles it from three before lines:
  `Option` from the `opt` of `opt = Util.stripLeadingHyphens(opt);` (line 149, an `identifier`
  paired with a `type_identifier`), `key` from `String key = opt;` (line 151) and `( opt )` from
  `names.containsKey(opt)` (line 152). `matcher`.
- **`key` in `getValues()` (1 visible, 2 invisible).** Before line 160 `((Option)
  options.get(key)).getValues()` -> after line 152 `key.getValues()`. The ground truth maps
  `key` to `key`; omnidiff deletes it. `matcher`.
- **`iterator()` (4 visible, 2 invisible).** Before line 299 `hashcodeMap.values().iterator()` ->
  `options.iterator()`. The ground truth deletes `hashcodeMap.values()` and inserts `options`;
  omnidiff renames `hashcodeMap` to `options` and deletes `.values()`, so it maps the first `.`
  where the ground truth maps the second. `options` replaces `hashcodeMap` as the iterated
  collection, so a rename reading is defensible. `ambiguous`.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Painting

Matcher-caused (Minimal about 496 bytes, Full about 545; absent with the human mapping):

- 228 bytes in both presets, Move -> None: the two comment lines between the fields (before
  lines 49-50, after lines 48-49). Because before line 51 is paired with after line 46, the
  comments are crossed by a matched pair and painted `Move`.
- 92 bytes, None -> Delete/Insert: the four imports, painted as kept-with-rename instead of
  deleted and inserted.
- Minimal 89, Full 100 bytes: the field declarations painted the wrong way round (line 46 Delete,
  line 51 kept; `options` / `= new` Update/Move on after line 46).
- The rest follow the other groups above: `getOptionObject` (before 93/98, after 91-92/97),
  `getOptionValues` (before 149-152/160, after 148/152) and `iterator()` (line 299).

Renderer-caused (Minimal 40 bytes, Full 90; also with the human mapping):

- Minimal 12 bytes, None -> Delete/Insert: the `{` and `}` of wholly deleted and inserted blocks
  (before lines 154-156, 281-287; after lines 164-180). The Minimal painting paints these braces;
  `MINIMAL` leaves structural punctuation unpainted. `renderer`.
- Minimal 6 bytes, Update -> Delete: `Key` of `containsKey` -> `contains` (before lines 69 and
  158). omnidiff narrows to an Update; the painting deletes the suffix. `renderer`.
- Minimal 14 bytes, Move -> None: `opt)` on line 69/67 and `key` on 160/152, re-parented into a
  new call or receiver; the Minimal painting leaves them unpainted. `renderer`.
- Full about 67 bytes, Move -> None: kept tokens displaced by an edit beside them (`;`,
  `.getType();`, `(key))`, `.getValues();`, `key.getValues();`), painted `Move` by `FULL`'s
  `paint_displaced_moves`; the Full painting leaves them unpainted. `renderer`.
- Full 15 bytes the other way, None -> Move/Insert: the `(opt)` of `options.put(key, opt)` ->
  `options.add(opt)` (line 288), which the Full painting paints `Move`, and the gap space and
  `));` on after line 67. `renderer`.
- 6 bytes in both presets on `opt` (before 93, after 91): the painting's `Move` that the human
  tree contradicts (see invariants). `ground-truth`.

## Verdict

- Primary cause: `matcher`
- Other causes: `renderer`, `ground-truth`, `ambiguous`
- Where a fix would go: the kind-only hash descent (`src/diff/solve_hash_descent.rs`,
  `hash_tree_matching::solve_with_hash_map`), which takes same-shape before candidates in reverse
  document order and so gives the field twin to line 51; `APTED("fast_fallback")`, which pairs
  imports of different classes by position; and `APTED("qualified_name")`, which pairs
  `identifier` with `type_identifier` and assembles a new statement from several old ones
- Ground truth questionable: yes: all four paintings paint `opt` (before line 93 -> after line 91)
  as a `Move` while the tree mapping deletes and inserts it (8 invariant-9 violations)
