/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, version 3 of the License.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! The review queue: places in fixtures someone has asked a human to look at again, kept in
//! `review_queue.tsv` at the repository root (git-ignored, one person's to-do list, not the
//! corpus's). A census or an agent writes it; the tool reads it.
//!
//! One item per line, tab-separated: `case`, `sites`, `note`. `sites` is space-separated
//! `b<line>` / `a<line>` (1-based, as a gutter shows them), e.g. `b290 a290`; each names the
//! whole line on that side. Blank lines and lines starting with `#` are skipped, and so is a line
//! that does not parse, so a hand-edited file never costs the session.
//!
//! The items show in the `V` popup after the case's invariant violations, numbered `R`, and jump
//! like them; `x` there drops one from the file. The `o` picker's `Review` column counts them.

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use omnidiff::code::Code;
use omnidiff::test::helper::human_mapping::HumanTextSpan;
use omnidiff::test::helper::human_mapping::invariants::{GroundTruthViolation, ViolationSite};

use crate::state::{InvariantEntry, site_detail};

/// One line of the queue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReviewItem {
    pub(crate) case: String,
    /// `(side, 0-based row)`, side 0 = before as in `ViolationSite`.
    pub(crate) sites: Vec<(usize, usize)>,
    pub(crate) note: String,
    /// The line as written, which is how `remove` finds it again.
    pub(crate) line: String,
}

pub(crate) fn review_queue_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("review_queue.tsv")
}

fn parse_line(line: &str) -> Option<ReviewItem> {
    if line.trim().is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.splitn(3, '\t');
    let case = fields.next()?.trim();
    let sites = fields
        .next()?
        .split_whitespace()
        .map(|site| {
            let side = match site.as_bytes().first()? {
                b'b' => 0,
                b'a' => 1,
                _ => return None,
            };
            let row: usize = site[1..].parse().ok()?;
            Some((side, row.checked_sub(1)?))
        })
        .collect::<Option<Vec<_>>>()?;
    if case.is_empty() {
        return None;
    }
    Some(ReviewItem {
        case: case.to_string(),
        sites,
        note: fields.next().unwrap_or("").trim().to_string(),
        line: line.to_string(),
    })
}

pub(crate) fn parse_queue(text: &str) -> Vec<ReviewItem> {
    text.lines().filter_map(parse_line).collect()
}

/// The queue on disk; empty when there is none.
pub(crate) fn load_queue() -> Vec<ReviewItem> {
    fs::read_to_string(review_queue_path())
        .map(|text| parse_queue(&text))
        .unwrap_or_default()
}

/// Open items per case, for the `o` picker's `Review` column.
pub(crate) fn queue_counts(items: &[ReviewItem]) -> HashMap<String, usize> {
    let mut counts = HashMap::new();
    for item in items {
        *counts.entry(item.case.clone()).or_insert(0) += 1;
    }
    counts
}

/// `case`'s items as `V` popup rows. A site past the end of its file is dropped rather than
/// failing the popup: the fixture may have been edited since the item was written.
pub(crate) fn review_entries(
    items: &[ReviewItem],
    case: &str,
    before: &Code,
    after: &Code,
) -> Vec<InvariantEntry> {
    items
        .iter()
        .filter(|item| item.case == case)
        .map(|item| {
            let sites: Vec<ViolationSite> = item
                .sites
                .iter()
                .filter_map(|&(side, row)| {
                    let code = if side == 0 { before } else { after };
                    let text = code.contents.split('\n').nth(row)?;
                    let start_column = text.len() - text.trim_start().len();
                    Some(ViolationSite {
                        side,
                        span: HumanTextSpan {
                            start_row: row,
                            start_column,
                            end_row: row,
                            end_column: text.trim_end().len().max(start_column),
                        },
                    })
                })
                .collect();
            let details = sites
                .iter()
                .map(|site| site_detail(*site, before, after))
                .collect();
            InvariantEntry {
                violation: GroundTruthViolation {
                    invariant: 0,
                    painting: None,
                    message: item.note.clone(),
                    sites,
                },
                details,
                review: Some(item.line.clone()),
            }
        })
        .collect()
}

/// `text` without the first line equal to `line`.
pub(crate) fn without_line(text: &str, line: &str) -> String {
    let mut removed = false;
    let mut out = String::with_capacity(text.len());
    for current in text.split_inclusive('\n') {
        if !removed && current.trim_end_matches(['\n', '\r']) == line {
            removed = true;
            continue;
        }
        out.push_str(current);
    }
    out
}

/// Drops the item written as `line` from the queue on disk.
pub(crate) fn remove_from_queue(line: &str) -> Result<()> {
    let path = review_queue_path();
    let text = fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
    fs::write(&path, without_line(&text, line)).with_context(|| format!("writing {path:?}"))
}
