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
//! A repository's history for the stats and sampling tools, read with gix: commit walks, tree
//! diffs as [`Delta`]s, blob reads, and the test repositories the tools' tests build.
//!
//! **gix, as the library's `git` feature uses** (see `src/library/git.rs`): pure Rust, no C build,
//! and 7-16x faster at a history walk than git2, which these tools used until 2026-11. What
//! moved with it: rename pairing is gix's (50% similarity, the first candidate where libgit2
//! weighs several, binary renames missed), which differs on a few files in a thousand; commits
//! with the same commit time may be walked in another order. Neither changes what a sampled
//! fixture is, which names its commits by id.
//!
//! **A [`Delta`] is what git2's diff delta was.** A change of a path's type (a file that became a
//! symlink) is a deletion and an addition, as git2's tree diff reports it without
//! `include_typechange`; a mode change within a type (`chmod +x`) is a modification. Each side
//! that is absent has the null id and mode 0, and both sides carry the path, so `old.path` of an
//! added file is its path.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use gix::object::tree::diff::ChangeDetached;
pub use gix::{Commit, ObjectId, Repository, Tree};

/// What a [`Delta`] did to its path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Status {
    Added,
    Deleted,
    Modified,
    /// Moved, perhaps with edits; only when rename detection is on.
    Renamed,
}

/// One side of a [`Delta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffFile {
    /// The blob's id; null for the absent side of an addition or deletion.
    pub id: ObjectId,
    /// The tree entry's mode as git writes it (`0o100644`, `0o120000`, ...); 0 when absent.
    pub mode: u16,
    pub path: PathBuf,
}

impl DiffFile {
    /// False for the absent side of an addition or deletion.
    pub fn exists(&self) -> bool {
        !self.id.is_null()
    }
}

/// One changed file between two trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delta {
    pub status: Status,
    pub old: DiffFile,
    pub new: DiffFile,
}

/// A repository path as a filesystem path; one that is not valid on this platform keeps its bytes
/// read as UTF-8, lossily.
fn path_of(path: &gix::bstr::BStr) -> PathBuf {
    gix::path::from_bstr(path)
        .map(std::borrow::Cow::into_owned)
        .unwrap_or_else(|_| PathBuf::from(path.to_string()))
}

/// The file type bits of a mode: a blob, executable or not, is one type; a link and a submodule
/// are others.
fn file_type(mode: gix::object::tree::EntryMode) -> u16 {
    mode.value() & 0o170000
}

/// The files `before` (the empty tree when `None`) and `after` differ in, in tree order, with
/// renames paired when `detect_renames` (gix's default: 50% similarity, no copies).
pub fn tree_diff(
    repo: &Repository,
    before: Option<&Tree>,
    after: &Tree,
    detect_renames: bool,
) -> Result<Vec<Delta>> {
    let rewrites = detect_renames.then(gix::diff::Rewrites::default);
    let options = gix::diff::Options::default().with_rewrites(rewrites);
    let changes = repo.diff_tree_to_tree(before, Some(after), options)?;
    let null = repo.object_hash().null();
    let side =
        |id: ObjectId, mode: gix::object::tree::EntryMode, path: &gix::bstr::BStr| DiffFile {
            id,
            mode: mode.value(),
            path: path_of(path),
        };
    let absent = |path: &gix::bstr::BStr| DiffFile {
        id: null,
        mode: 0,
        path: path_of(path),
    };
    let mut deltas = Vec::new();
    for change in changes {
        match change {
            ChangeDetached::Addition {
                location,
                entry_mode,
                id,
                ..
            } if !entry_mode.is_tree() => deltas.push(Delta {
                status: Status::Added,
                old: absent(location.as_ref()),
                new: side(id, entry_mode, location.as_ref()),
            }),
            ChangeDetached::Deletion {
                location,
                entry_mode,
                id,
                ..
            } if !entry_mode.is_tree() => deltas.push(Delta {
                status: Status::Deleted,
                old: side(id, entry_mode, location.as_ref()),
                new: absent(location.as_ref()),
            }),
            ChangeDetached::Modification {
                location,
                previous_entry_mode,
                previous_id,
                entry_mode,
                id,
            } => {
                let (old, new) = (
                    side(previous_id, previous_entry_mode, location.as_ref()),
                    side(id, entry_mode, location.as_ref()),
                );
                match (previous_entry_mode.is_tree(), entry_mode.is_tree()) {
                    (true, true) => {}
                    (false, false) if file_type(previous_entry_mode) == file_type(entry_mode) => {
                        deltas.push(Delta {
                            status: Status::Modified,
                            old,
                            new,
                        });
                    }
                    // A type change: what was there went, and something else came.
                    (was_tree, is_tree) => {
                        if !was_tree {
                            deltas.push(Delta {
                                status: Status::Deleted,
                                old,
                                new: absent(location.as_ref()),
                            });
                        }
                        if !is_tree {
                            deltas.push(Delta {
                                status: Status::Added,
                                old: absent(location.as_ref()),
                                new: side(id, entry_mode, location.as_ref()),
                            });
                        }
                    }
                }
            }
            ChangeDetached::Rewrite {
                source_location,
                source_entry_mode,
                source_id,
                entry_mode,
                id,
                location,
                ..
            } if !entry_mode.is_tree() => deltas.push(Delta {
                status: Status::Renamed,
                old: side(source_id, source_entry_mode, source_location.as_ref()),
                new: side(id, entry_mode, location.as_ref()),
            }),
            _ => {}
        }
    }
    Ok(deltas)
}

/// True for a commit a shallow clone's history stops at. Its parents are named but not there;
/// these tools count it as a root commit, with no parents, as git2 did.
pub fn is_shallow_boundary(repo: &Repository, id: &gix::oid) -> bool {
    repo.shallow_commits()
        .ok()
        .flatten()
        .is_some_and(|commits| commits.iter().any(|commit| commit.as_ref() == id))
}

/// `commit`'s first parent; an error for a root commit, or one whose parent is not there.
pub fn first_parent<'repo>(commit: &Commit<'repo>) -> Result<Commit<'repo>> {
    let parent = commit
        .parent_ids()
        .next()
        .with_context(|| format!("commit {} has no parent", commit.id))?;
    Ok(parent.object()?.peel_to_commit()?)
}

/// The tree of `commit`'s first parent; an error for a root commit.
pub fn first_parent_tree<'repo>(commit: &Commit<'repo>) -> Result<Tree<'repo>> {
    Ok(first_parent(commit)?.tree()?)
}

/// The id of `commit`'s first parent; an error for a root commit.
pub fn first_parent_id(commit: &Commit) -> Result<ObjectId> {
    commit
        .parent_ids()
        .next()
        .map(|id| id.detach())
        .with_context(|| format!("commit {} has no parent", commit.id))
}

/// The commit named by the full hex id `id`.
pub fn find_commit<'repo>(repo: &'repo Repository, id: &str) -> Result<Commit<'repo>> {
    Ok(repo.find_commit(ObjectId::from_hex(id.as_bytes())?)?)
}

/// The entry at `path` inside `tree`; an error when there is none.
fn entry_at<'repo>(tree: &Tree<'repo>, path: &Path) -> Result<gix::object::tree::Entry<'repo>> {
    tree.lookup_entry_by_path(path)?
        .with_context(|| format!("{} is not in tree {}", path.display(), tree.id))
}

/// The raw content of the blob at `path` inside `tree`.
pub fn blob_bytes(repo: &Repository, tree: &Tree, path: &Path) -> Result<Vec<u8>> {
    let entry = entry_at(tree, path)?;
    Ok(repo.find_blob(entry.object_id())?.take_data())
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
    let text = std::str::from_utf8(content).ok()?;
    Some(text.lines().count())
}

/// Calls `on_commit` for each of the `max_commits` most recent commits reachable from HEAD that has
/// exactly one parent: merges mix unrelated changes and root commits have no "before".
///
/// Time-ordered (commit time, newest first) because repeated shallow fetches can leave far more
/// history than the nominal depth, and `max_commits` must mean "the most recent".
pub fn walk_single_parent_commits(
    repo_path: &Path,
    max_commits: usize,
    mut on_commit: impl FnMut(&Repository, &Commit) -> Result<()>,
) -> Result<()> {
    let repo = gix::open(repo_path)?;
    let head = repo.head_id()?.detach();
    let walk = repo
        .rev_walk([head])
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .all()?;

    for (visited, info) in walk.enumerate() {
        if visited >= max_commits {
            break;
        }
        let Ok(info) = info else { continue };
        if info.parent_ids.len() != 1 || is_shallow_boundary(&repo, &info.id) {
            continue;
        }
        let Ok(commit) = repo.find_commit(info.id) else {
            continue;
        };
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
    mut on_delta: impl FnMut(&Repository, ObjectId, &Delta) -> Result<()>,
) -> Result<()> {
    walk_single_parent_commits(repo_path, max_commits, |repo, commit| {
        let before_tree = first_parent_tree(commit)?;
        let after_tree = commit.tree()?;
        // Without rename detection a renamed+edited file is an unrelated Added/Deleted pair.
        let deltas = tree_diff(repo, Some(&before_tree), &after_tree, detect_renames)?;
        for delta in &deltas {
            on_delta(repo, commit.id, delta)?;
        }
        Ok(())
    })
}

/// Test repositories, written object by object: no working tree, index or git configuration is
/// read or needed, so a test builds the same history on any machine.
pub mod testing {
    use super::*;

    /// A new repository at `dir` (with a working tree directory, left empty), opened without the
    /// machine's git configuration.
    pub fn init(dir: &Path) -> Result<Repository> {
        let repo = gix::ThreadSafeRepository::init_opts(
            dir,
            gix::create::Kind::WithWorktree,
            gix::create::Options::default(),
            gix::open::Options::isolated(),
        )?;
        Ok(repo.to_thread_local())
    }

    /// Writes `entries` (path, bytes, mode) as a tree, directories made as the paths need them.
    /// A submodule (mode `0o160000`) points at the made-up commit `111...1`; its bytes are unused.
    pub fn tree(repo: &Repository, entries: &[(&str, &[u8], u32)]) -> Result<ObjectId> {
        let mut editor = repo.edit_tree(repo.object_hash().empty_tree())?;
        for (path, bytes, mode) in entries {
            let mode = gix::object::tree::EntryMode::try_from(*mode)
                .map_err(|mode| anyhow::anyhow!("{mode:o} is not a tree entry mode"))?;
            let id = if mode.is_commit() {
                ObjectId::from_hex("1".repeat(40).as_bytes())?
            } else {
                repo.write_blob(bytes)?.detach()
            };
            editor.upsert(*path, mode.kind(), id)?;
        }
        Ok(editor.write()?.detach())
    }

    /// Writes [`tree`] of `files`, each a regular file.
    pub fn tree_of_files(repo: &Repository, files: &[(&str, &[u8])]) -> Result<ObjectId> {
        let entries: Vec<(&str, &[u8], u32)> = files
            .iter()
            .map(|(path, bytes)| (*path, *bytes, 0o100644))
            .collect();
        tree(repo, &entries)
    }

    /// Writes a commit of `tree` on `parent` by `t <t@example.com>` at `seconds` since the epoch,
    /// both as author and committer; with `update_head`, `HEAD` (and the branch it names) is
    /// moved to it, which needs `parent` to be where `HEAD` is.
    pub fn commit(
        repo: &Repository,
        tree: ObjectId,
        parent: Option<ObjectId>,
        seconds: i64,
        update_head: bool,
    ) -> Result<ObjectId> {
        commit_with_message(repo, tree, parent, seconds, update_head, "c")
    }

    /// [`commit`] with `message`.
    pub fn commit_with_message(
        repo: &Repository,
        tree: ObjectId,
        parent: Option<ObjectId>,
        seconds: i64,
        update_head: bool,
        message: &str,
    ) -> Result<ObjectId> {
        let signature = gix::actor::Signature {
            name: "t".into(),
            email: "t@example.com".into(),
            time: gix::date::Time::new(seconds, 0),
        };
        let mut time = gix::date::parse::TimeBuf::default();
        let signature = signature.to_ref(&mut time);
        Ok(if update_head {
            repo.commit_as(signature, signature, "HEAD", message, tree, parent)?
                .detach()
        } else {
            repo.new_commit_as(signature, signature, message, tree, parent)?
                .id
        })
    }
}
