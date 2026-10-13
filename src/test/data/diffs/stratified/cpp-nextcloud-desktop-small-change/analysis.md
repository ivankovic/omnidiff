# Accuracy analysis: cpp-nextcloud-desktop-small-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 59046 nodes, 59046 graded, N:M floor 0 visible |
| Painting, Minimal | 52 bytes wrong of 229046: renderer 52, matcher 0 |
| Painting, Full | 178 bytes wrong of 229046: renderer 178, matcher 0 |
| Ground-truth invariant violations | 4 |

## The change

Two `qCDebug(lcCseEncryption()) << "use certificate on ..."` lines (594 and 596) switch the logging
category to `lcCseDecryption()` and append `<< selectedCertificate.sha256Fingerprint()`.

## Painting

All runs are the same with the human mapping.

- **Rename narrowed below the word (Minimal, 32 bytes, `None -> Update`, `renderer`).** For
  `lcCseEncryption` -> `lcCseDecryption`, omnidiff paints only the differing letters `En` / `De`. It
  leaves `cryption`, the rest of the changed word, unpainted on both sides of both lines. The
  painting's word rule (invariant [16]) wants the whole word `Encryption` / `Decryption` painted.
- **Painting past the word (Minimal, 20 bytes, `None -> Update`, `ground-truth`).** The Minimal
  painting also paints the unchanged leading word `lcCse`. `invariants.txt` flags this four times
  ([16]: "the differing words are Encryption, and everything else in the identifier should be left
  alone").
- **Unchanged parts of the updated lines (Full, 178 bytes, `None -> Move`, `renderer`).** The Full
  painting matches `(` and `()) << "use certificate on ..."` as identical spans on both lines, so
  they read as Move. Omnidiff leaves this unchanged in-place text unpainted.

## Verdict

- Primary cause: `renderer`
- Other causes: `ground-truth`
- Where a fix would go: identifier-update narrowing in `src/diff/text.rs`, which should work at the
  word level in Minimal, not the character level. Full display of the unchanged in-place parts of an
  updated statement. The `lcCse` part is the painting's.
- Ground truth questionable: yes. The Minimal painting paints the unchanged word `lcCse` of the
  renamed identifier (4 invariant [16] violations).
