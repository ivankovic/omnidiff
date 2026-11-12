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
use std::path::Path;

use anyhow::Result;
use git2::{Commit, DiffDelta, DiffFindOptions, Oid, Repository, Sort, Tree};

/// The raw content of the blob at `path` inside `tree`.
pub fn blob_bytes(repo: &Repository, tree: &Tree, path: &Path) -> Result<Vec<u8>> {
    let entry = tree.get_path(path)?;
    let blob = repo.find_blob(entry.id())?;
    Ok(blob.content().to_vec())
}

/// The line count of the blob at `oid` (for `sampling::loc_bucket`), or `None` if it is not UTF-8
/// or its *byte* length falls outside `[min_bytes, max_bytes]`.
pub fn text_loc_if_in_range(
    repo: &Repository,
    oid: Oid,
    min_bytes: usize,
    max_bytes: usize,
) -> Option<usize> {
    let blob = repo.find_blob(oid).ok()?;
    let content = blob.content();
    if content.len() < min_bytes || content.len() > max_bytes {
        return None;
    }
    let text = std::str::from_utf8(content).ok()?;
    Some(text.lines().count())
}

/// Calls `on_commit` for each of the `max_commits` most recent commits reachable from HEAD that has
/// exactly one parent: merges mix unrelated changes and root commits have no "before".
///
/// Time-ordered because repeated shallow fetches can leave far more history than the nominal
/// depth, and `max_commits` must mean "the most recent".
pub fn walk_single_parent_commits(
    repo_path: &Path,
    max_commits: usize,
    mut on_commit: impl FnMut(&Repository, &Commit) -> Result<()>,
) -> Result<()> {
    let repo = Repository::open(repo_path)?;
    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TIME)?;
    walk.push_head()?;

    for (visited, id) in walk.enumerate() {
        if visited >= max_commits {
            break;
        }
        let Ok(id) = id else { continue };
        let Ok(commit) = repo.find_commit(id) else {
            continue;
        };
        if commit.parents().len() != 1 {
            continue;
        }
        on_commit(&repo, &commit)?;
    }

    Ok(())
}

/// Calls `on_delta` for each changed file of the commits [`walk_single_parent_commits`] visits,
/// diffed against the parent; filtering deltas is the caller's job.
pub fn walk_single_parent_commit_diffs(
    repo_path: &Path,
    max_commits: usize,
    detect_renames: bool,
    mut on_delta: impl FnMut(&Repository, Oid, &DiffDelta) -> Result<()>,
) -> Result<()> {
    walk_single_parent_commits(repo_path, max_commits, |repo, commit| {
        let before_tree = commit.parent(0)?.tree()?;
        let after_tree = commit.tree()?;
        let mut diff = repo.diff_tree_to_tree(Some(&before_tree), Some(&after_tree), None)?;
        if detect_renames {
            // Off by default; without it a renamed+edited file is an unrelated Added/Deleted pair.
            let mut find_opts = DiffFindOptions::new();
            find_opts.renames(true);
            diff.find_similar(Some(&mut find_opts))?;
        }

        for delta in diff.deltas() {
            on_delta(repo, commit.id(), &delta)?;
        }
        Ok(())
    })
}
