# Accuracy analysis: ruby-mastodon-mastodon-normal-change

Measured 2026-10-13 on main 3f7c2b64 (`benchmark_optimal_solutions`, `painting_failure_census`,
`mismatch_detail_for_fixture`, `painting_disagreement_detail`). The engine changes; re-measure
before relying on these numbers.

## Numbers

| | |
|---|---|
| Mapping mismatches | 4 (4 visible) of 126 nodes, 126 graded, N:M floor 0 visible |
| Painting, Minimal | 22 bytes wrong of 567: renderer 0, matcher 22 |
| Painting, Full | 54 bytes wrong of 567: renderer 34, matcher 23 |
| Ground-truth invariant violations | none |

## The change

In a serializer, `:position` is removed from `attributes`, and
`belongs_to :account, serializer: REST::AccountSerializer, if: :accepted?` (line 8) becomes
`attribute :account_id, if: :accepted?`, with a new `def account_id` method.

## Mapping

All four on line 8, reason `APTED("qualified_name")`:

- The ground truth deletes `:account` and its `,`, inserts `:account_id`, and keeps the second
  `,` (the one in front of `if:`) as the after `,`. Omnidiff updates `:account` to `:account_id`,
  keeps the first `,`, and deletes the second `,` with `serializer: REST::AccountSerializer`.
- Both readings are defensible. `:account` -> `:account_id` is a cheap rename by edit distance;
  the ground truth reads the association `belongs_to :account` and the attribute
  `attribute :account_id` as different things (it does map `belongs_to` -> `attribute` as an
  update of the call). Which comma survives follows from that choice. `ambiguous`.

## Painting

- Matcher, from the mapping choice above (Minimal 22 bytes, Full 23 bytes): before `:account, `
  (line 8, columns 13-23) is unpainted by omnidiff but Delete in the painting; after `:account_id`
  is Update-narrowed by omnidiff but Insert in the painting; omnidiff paints the second `,`
  (column 58) as Delete where the Minimal painting leaves it unpainted. `ambiguous` (matcher
  owned).
- Full, renderer, 2 bytes: the Full painting's `belongs_to ` -> `attribute ` update includes the
  trailing space on both sides; the rendering stops at the word. `renderer`.
- Full, painting convention, 32 bytes: the Full painting marks the unchanged tail
  `, if: :accepted?` as Move on both sides (before columns 58-74, after 23-39). The rendering with
  the human mapping leaves it unpainted, since the `if:` pair is Identical. This is the "identical
  line tail" case of rule 3 in research/data/quality/text_painting_findings.md, which says such a
  tail stays unpainted. In ruby-mastodon-mastodon-add-func-and-attribute the Full painting leaves
  a shifted `:type` unpainted. `ground-truth` (Marko's call whether Full should paint it).

## Verdict

- Primary cause: `ambiguous`
- Other causes: `ground-truth`, `renderer`
- Where a fix would go: none for the mapping (ambiguous); the 2-byte trailing-space narrowing of
  an Update in the Full renderer.
- Ground truth questionable: yes: the Full painting paints an identical line tail
  (`, if: :accepted?`) as Move, against rule 3 of text_painting_findings.md.
