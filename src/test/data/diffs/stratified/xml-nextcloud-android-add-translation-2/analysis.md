# Accuracy analysis: xml-nextcloud-android-add-translation-2

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 45410 nodes, 45410 graded, N:M floor 0 visible |
| Painting, Minimal | 0 bytes wrong of 214474: renderer 0, matcher 0 |
| Painting, Full | 4 bytes wrong of 214474: renderer 4, matcher 0 |
| Ground-truth invariant violations | none |

## The change

Nextcloud Android `app/src/main/res/values-sv/strings.xml`: one string entry is added, after line
249: `<string name="create_new_encrypted_folder">Ny krypterad mapp</string>`. The rest of the
~45k-node file is unchanged.

## Painting

There are no mapping mismatches and Minimal agrees exactly. Full, 4 bytes, None -> Insert: the
4-space indentation of after line 249. The Full painting includes it in the Insert; omnidiff's
Full rendering starts the range at the `<string>` tag. The indentation is part of a whitespace
`CharData` between sibling elements, not of the added element, and the run is the same with the
human mapping. `renderer`.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Full rendering (extend an Insert of a whole line over that
  line's indentation)
- Ground truth questionable: no
