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
//! History sampling needs a stable path order and treats file-type replacements as separate
//! deletions and additions, so they never become in-place source edits.
//! Diff readers use gix's traversal parents, which treat shallow boundaries as roots.

use std::path::Path;

use anyhow::{Context, Result};
use gix::{ObjectId, Repository, Tree, bstr::ByteSlice as _, diff::tree_with_rewrites::Change};

/// The raw content of the blob at `path` inside `tree`.
pub fn blob_bytes(repo: &Repository, tree: &Tree<'_>, path: &Path) -> Result<Vec<u8>> {
    let entry = tree
        .lookup_entry_by_path(path)?
        .with_context(|| format!("path {} not found in tree", path.display()))?;
    let blob = repo.find_blob(entry.id())?;
    Ok(blob.detach().data)
}

/// The line count of the blob at `oid` (for `sampling::loc_bucket`), or `None` if it is not UTF-8
/// or its *byte* length falls outside `[min_bytes, max_bytes]`.
pub fn text_loc_if_in_range(
    repo: &Repository,
    oid: ObjectId,
    min_bytes: usize,
    max_bytes: usize,
) -> Option<usize> {
    let blob = repo.find_blob(oid).ok()?;
    let content = &blob.data;
    if content.len() < min_bytes || content.len() > max_bytes {
        return None;
    }
    let text = content.to_str().ok()?;
    Some(text.lines().count())
}

/// File changes ordered by path, with type replacements represented as deletion then addition.
pub fn file_changes(
    repo: &Repository,
    before: Option<&Tree<'_>>,
    after: &Tree<'_>,
    options: gix::diff::Options,
) -> Result<Vec<Change>> {
    let mut changes = Vec::new();
    for change in repo.diff_tree_to_tree(before, after, options)? {
        if change.entry_mode().is_tree() {
            continue;
        }
        if let Change::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
        } = &change
            && previous_entry_mode.kind() != entry_mode.kind()
            && !(previous_entry_mode.is_blob() && entry_mode.is_blob())
        {
            changes.push(Change::Deletion {
                location: location.clone(),
                entry_mode: *previous_entry_mode,
                id: *previous_id,
                relation: None,
            });
            changes.push(Change::Addition {
                location: location.clone(),
                entry_mode: *entry_mode,
                id: *id,
                relation: None,
            });
        } else {
            changes.push(change);
        }
    }
    changes.sort_by(|a, b| a.location().cmp(b.location()));
    Ok(changes)
}

/// Calls `on_delta` for each changed file in single-parent commits among the `max_commits` most
/// recent commits reachable from HEAD, diffed against the parent; filtering is the caller's job.
///
/// Time-ordered because repeated shallow fetches can leave far more history than the nominal
/// depth, and `max_commits` must mean "the most recent".
pub fn walk_single_parent_commit_diffs(
    repo_path: &Path,
    max_commits: usize,
    detect_renames: bool,
    mut on_delta: impl FnMut(&Repository, ObjectId, &Change) -> Result<()>,
) -> Result<()> {
    let repo = gix::open(repo_path)?;
    let walk = repo
        .head_id()?
        .ancestors()
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .all()?;
    let options =
        gix::diff::Options::default().with_rewrites(detect_renames.then(Default::default));

    for info in walk.take(max_commits) {
        let Ok(info) = info else { continue };
        let Ok(commit) = repo.find_commit(info.id) else {
            continue;
        };

        // Merges mix unrelated changes; roots and shallow boundaries have no "before".
        if info.parent_ids.len() != 1 {
            continue;
        }
        let parent = repo.find_commit(info.parent_ids[0])?;

        let before_tree = parent.tree()?;
        let after_tree = commit.tree()?;
        for delta in file_changes(&repo, Some(&before_tree), &after_tree, options)? {
            on_delta(&repo, info.id, &delta)?;
        }
    }

    Ok(())
}
