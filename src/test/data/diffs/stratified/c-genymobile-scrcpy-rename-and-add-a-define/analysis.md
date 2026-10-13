# Accuracy analysis: c-genymobile-scrcpy-rename-and-add-a-define

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 0 (0 visible) of 46 nodes, 46 graded, N:M floor 0 visible |
| Painting, Minimal | 4 bytes wrong of 672: renderer 4, matcher 0 |
| Painting, Full | 0 bytes wrong of 672: renderer 0, matcher 0 |
| Ground-truth invariant violations | none |

## The change

scrcpy events header: `SC_EVENT_STREAM_STOPPED` is renamed `SC_EVENT_DEVICE_DISCONNECTED`
(line 2) and a new `#define SC_EVENT_DEMUXER_ERROR (SDL_USEREVENT + 5)` is added (after line 6).

## Painting

- **Minimal, None -> Update, 4 bytes:** the final `ED` of `STOPPED` (before line 2) and of
  `DISCONNECTED` (after line 2). The Minimal painting updates the whole changed words
  `STREAM_STOPPED` -> `DEVICE_DISCONNECTED`; omnidiff's narrowed Update also trims the common
  suffix `ED`, which falls inside a word. The same with the human mapping. `renderer`.

Full agrees with the painting; the mapping has no mismatches.

## Verdict

- Primary cause: `renderer`
- Other causes: none
- Where a fix would go: `diff::text` Update narrowing: trim common prefixes/suffixes only at word
  boundaries (`text_painting_findings.md` rule 3 makes the same point about in-word suffixes)
- Ground truth questionable: no
