# Accuracy analysis: python-aboutcode-org-license-expression-excellent-test-case

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 251 (186 visible) of 27091 nodes, 27091 graded, N:M floor 0 visible |
| Painting, Minimal | no painting |
| Painting, Full | no painting |
| Ground-truth invariant violations | none |

## The change

`etc/scripts/utils_thirdparty.py` gets a broad cleanup:

- The supported Python versions move from 3.7-3.10 to 3.9-3.13. `PYTHON_VERSIONS` (line 117),
  `PYTHON_DOT_VERSIONS_BY_VER` (lines 119-124) and `ABIS_BY_PYTHON_VERSION` (lines 134-139) drop
  the `37`/`38` entries and gain `311`-`313` entries.
- Comprehension variables are renamed, `except Exception:` becomes `except:`, and `raise
  ValueError` checks become `assert`.
- A two-line f-string message (lines 861-862) is joined into one.
- `yield from self.wheels` becomes a `for` loop.

description.md calls the version lists "an interesting case of modifying the list with version
numbers being removed and added, which requires logical analysis not just syntax". It also says
two strings should map to one.

## Mapping

- **Version lists shifted by position (179 visible, 60 invisible), `matcher`.** The ground truth
  deletes `"37"`/`"38"`, keeps `"39"`/`"310"` identical, and inserts `"311"`-`"313"`. It does this
  in the tuple on line 117 and in both dicts (entries `"37": "3.7"` ... and
  `"37": ["cp37", "cp37m", "abi3"]` ...). omnidiff aligns the entries by position:
  - line 117: `"37"` -> `"39"`, `"38"` -> `"310"`, `"39"` -> `"311"`, `"310"` -> `"312"`, and
    inserts the last one.
  - the dicts: entry `"37": "3.7"` -> `"39": "3.9"`, and so on down the list.
  - the ABI dict: `"cp37"` -> `"cp39"` and so on.

  The leaf pairs are Identical/Update from the whole-file residual `APTED("fast_fallback")`. The
  mismatches on whole deleted or inserted entries (Insert/Delete with children) print the same
  reason, so the whole group is this one pass. A reader sees every line of the three tables edited instead of two lines removed at the top and
  three added at the bottom. Ordered edit distance with unit costs makes the shift cheap, since the
  quotes, `:` and `abi3` all match. Keeping the identical keys needs a key-aware or LCS-first
  alignment of the elements.
- **Joined f-string message deleted (6 visible, 4 invisible), `matcher`.** Before lines 861-862 are
  `msg = f"No text for license {filename} in expression "` followed by a separate expression
  statement, `f"{self.license_expression!r} from {self}"`. After, it is one assignment with a
  single f-string. The ground truth maps the assignment, its `=`, the first string's start/content,
  and the second string's `{`/`}`/end into the new string, many-to-one but not all-to-all (floor
  0). omnidiff deletes the before assignment and the second statement's pieces
  (`APTED("qualified_name")`, op Delete).
- **`yield from` to `for ... yield` (1 visible, 1 invisible), `matcher`.** The ground truth maps
  `yield` (before line 1487) into the new loop body (after line 1489). omnidiff deletes it
  (`APTED("qualified_name")`): the statement moved one level down into a new `for` block.

Reasons for the "with children" mismatches re-read on 2026-10-13 after the test helper began printing them.

## Verdict

- Primary cause: `matcher`
- Other causes: none
- Where a fix would go: element alignment for the residual APTED (`APTED("fast_fallback")`) on
  tuple and dict literals. Anchor identical elements or keys (LCS) before the positional edit
  distance.
- Ground truth questionable: no
