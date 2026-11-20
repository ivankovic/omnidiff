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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! Changes in a git repository, each file a [`Diff`]: the `git` feature.
//!
//! ```no_run
//! # fn main() -> Result<(), omnidiff::Error> {
//! let repo = omnidiff::git::Repository::open(".")?;
//! for (file, change) in repo.commit("HEAD")?.changed_definitions() {
//!     println!("{} {} in {}", change.kind(), change.qualified_name(), file.path().display());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! **gix, not git2 or the `git` binary.** gix is pure Rust, so the feature adds no C build and no
//! OpenSSL, and it is the fastest of the three at what a change set needs: measured 2026-11-20 on
//! this repository's history, a 3,000-commit walk with rename detection took 1.5 s against
//! git2's 25 s, and a status 0.34 s against 1.42 s. A library should not spawn processes, so the
//! TUI's review picker, which does run `git`, keeps its own code.
//!
//! **Lazy.** A change set lists its files at once, from the trees and the index; a file's blobs
//! are read and diffed the first time [`FileChange::diff`] is called, so a commit of 500 files
//! costs nothing until one is looked at.
//!
//! **Qualified names read the revision.** A side's `go.mod` and `__init__.py` files are looked
//! up in the tree that side came from (the working tree for unstaged changes), not in the
//! checkout, so a commit from a year ago names its definitions as they were named then.
//!
//! Renames follow gix's rename tracking (50% similarity, as git's default), which differs from
//! git's on a few files in a thousand: it takes the first candidate where git weighs four, and it
//! misses renames of binary files.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use gix::object::tree::diff::ChangeDetached;

use super::qualified_name::ProjectFiles;
use super::{DefinitionChange, Diff, Error};
use crate::code::decode_text;

fn git_error(e: impl std::error::Error + Send + Sync + 'static) -> Error {
    Error::Git(Box::new(e))
}

/// A git repository, found from any directory inside it.
#[derive(Clone)]
pub struct Repository {
    repo: gix::ThreadSafeRepository,
}

impl std::fmt::Debug for Repository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Repository")
            .field("git_dir", &self.repo.git_dir())
            .finish()
    }
}

impl Repository {
    /// Opens the repository `path` is in, as `git` would find it from there.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, Error> {
        let repo = gix::discover(path.as_ref()).map_err(git_error)?;
        Ok(Self {
            repo: repo.into_sync(),
        })
    }

    /// The working tree's root; `None` for a bare repository.
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.work_dir()
    }

    /// What the commit `revision` (`HEAD`, `main~2`, a hash) changed against its first parent;
    /// a root commit against the empty tree.
    pub fn commit(&self, revision: &str) -> Result<ChangeSet, Error> {
        let repo = self.repo.to_thread_local();
        let commit = repo
            .rev_parse_single(revision)
            .map_err(git_error)?
            .object()
            .map_err(git_error)?
            .peel_to_commit()
            .map_err(git_error)?;
        let after = commit.tree_id().map_err(git_error)?.detach();
        let before = match commit.parent_ids().next() {
            Some(parent) => repo
                .find_commit(parent)
                .map_err(git_error)?
                .tree_id()
                .map_err(git_error)?
                .detach(),
            None => repo.empty_tree().id,
        };
        self.trees(before, after)
    }

    /// What changed from revision `from` to revision `to`, as `git diff from to` shows it.
    pub fn range(&self, from: &str, to: &str) -> Result<ChangeSet, Error> {
        let repo = self.repo.to_thread_local();
        let tree = |revision: &str| -> Result<gix::ObjectId, Error> {
            Ok(repo
                .rev_parse_single(revision)
                .map_err(git_error)?
                .object()
                .map_err(git_error)?
                .peel_to_tree()
                .map_err(git_error)?
                .id)
        };
        self.trees(tree(from)?, tree(to)?)
    }

    fn trees(&self, before: gix::ObjectId, after: gix::ObjectId) -> Result<ChangeSet, Error> {
        let repo = self.repo.to_thread_local();
        let options = gix::diff::Options::default().with_rewrites(Some(Default::default()));
        let changes = repo
            .diff_tree_to_tree(
                Some(&repo.find_tree(before).map_err(git_error)?),
                Some(&repo.find_tree(after).map_err(git_error)?),
                options,
            )
            .map_err(git_error)?;
        let projects = (
            self.tree_files(before) as Arc<dyn ProjectFiles + Send + Sync>,
            self.tree_files(after) as Arc<dyn ProjectFiles + Send + Sync>,
        );
        let mut files = Vec::new();
        for change in changes {
            let file = match change {
                ChangeDetached::Addition {
                    location,
                    entry_mode,
                    id,
                    ..
                } if !entry_mode.is_tree() => self.file(
                    FileStatus::Added,
                    None,
                    Some((location, Source::Blob(id))),
                    &projects,
                ),
                ChangeDetached::Deletion {
                    location,
                    entry_mode,
                    id,
                    ..
                } if !entry_mode.is_tree() => self.file(
                    FileStatus::Deleted,
                    Some((location, Source::Blob(id))),
                    None,
                    &projects,
                ),
                ChangeDetached::Modification {
                    location,
                    previous_entry_mode,
                    previous_id,
                    entry_mode,
                    id,
                } if !entry_mode.is_tree() => {
                    let status = if previous_entry_mode.kind() == entry_mode.kind() {
                        FileStatus::Modified
                    } else {
                        FileStatus::TypeChanged
                    };
                    self.file(
                        status,
                        Some((location.clone(), Source::Blob(previous_id))),
                        Some((location, Source::Blob(id))),
                        &projects,
                    )
                }
                ChangeDetached::Rewrite {
                    source_location,
                    source_id,
                    location,
                    id,
                    entry_mode,
                    copy,
                    ..
                } if !entry_mode.is_tree() => self.file(
                    if copy {
                        FileStatus::Copied
                    } else {
                        FileStatus::Renamed
                    },
                    Some((source_location, Source::Blob(source_id))),
                    Some((location, Source::Blob(id))),
                    &projects,
                ),
                _ => continue,
            };
            files.push(file);
        }
        Ok(ChangeSet { files })
    }

    /// What is staged: `HEAD` against the index, as `git diff --cached` shows it.
    pub fn staged(&self) -> Result<ChangeSet, Error> {
        let repo = self.repo.to_thread_local();
        let head = head_tree(&repo)?;
        let index = repo.index_or_empty().map_err(git_error)?;
        let tree_files = self.tree_files(head) as Arc<dyn ProjectFiles + Send + Sync>;
        let projects = (tree_files.clone(), tree_files);
        let mut files = Vec::new();
        repo.tree_index_status(
            &head,
            &index,
            None,
            gix::status::tree_index::TrackRenames::AsConfigured,
            |change, _, _| {
                use gix::diff::index::ChangeRef as C;
                let blob = |id: &gix::oid| Source::Blob(id.to_owned());
                let file = match change {
                    C::Addition { location, id, .. } => self.file(
                        FileStatus::Added,
                        None,
                        Some((location.into_owned(), blob(&id))),
                        &projects,
                    ),
                    C::Deletion { location, id, .. } => self.file(
                        FileStatus::Deleted,
                        Some((location.into_owned(), blob(&id))),
                        None,
                        &projects,
                    ),
                    C::Modification {
                        location,
                        previous_id,
                        id,
                        ..
                    } => self.file(
                        FileStatus::Modified,
                        Some((location.clone().into_owned(), blob(&previous_id))),
                        Some((location.into_owned(), blob(&id))),
                        &projects,
                    ),
                    C::Rewrite {
                        source_location,
                        source_id,
                        location,
                        id,
                        copy,
                        ..
                    } => self.file(
                        if copy {
                            FileStatus::Copied
                        } else {
                            FileStatus::Renamed
                        },
                        Some((source_location.into_owned(), blob(&source_id))),
                        Some((location.into_owned(), blob(&id))),
                        &projects,
                    ),
                };
                files.push(file);
                Ok(std::ops::ControlFlow::Continue(()))
            },
        )
        .map_err(git_error)?;
        files.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(ChangeSet { files })
    }

    /// What is not staged: the index against the working tree, as `git diff` shows it, and the
    /// untracked files `git status` lists (not the ignored ones).
    pub fn unstaged(&self) -> Result<ChangeSet, Error> {
        let repo = self.repo.to_thread_local();
        let workdir = repo
            .workdir()
            .ok_or_else(|| {
                git_error(std::io::Error::other(
                    "a bare repository has no working tree",
                ))
            })?
            .to_path_buf();
        let head = head_tree(&repo)?;
        let projects = (
            self.tree_files(head) as Arc<dyn ProjectFiles + Send + Sync>,
            Arc::new(WorkdirFiles {
                root: workdir.clone(),
            }) as Arc<dyn ProjectFiles + Send + Sync>,
        );
        let items = repo
            .status(gix::progress::Discard)
            .map_err(git_error)?
            .untracked_files(gix::status::UntrackedFiles::Files)
            .index_worktree_rewrites(None)
            .into_index_worktree_iter(Vec::new())
            .map_err(git_error)?;
        let mut files = Vec::new();
        for item in items {
            use gix::status::index_worktree::Item;
            use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};
            let item = item.map_err(git_error)?;
            let worktree = |path: &gix::bstr::BStr| Source::Worktree(workdir.join(path_of(path)));
            let file = match item {
                Item::Modification {
                    entry,
                    rela_path,
                    status,
                    ..
                } => {
                    let blob = Source::Blob(entry.id);
                    match status {
                        EntryStatus::Change(Change::Removed) => self.file(
                            FileStatus::Deleted,
                            Some((rela_path, blob)),
                            None,
                            &projects,
                        ),
                        EntryStatus::Change(Change::Modification { .. })
                        | EntryStatus::Conflict { .. } => {
                            let after = worktree(rela_path.as_ref());
                            self.file(
                                FileStatus::Modified,
                                Some((rela_path.clone(), blob)),
                                Some((rela_path, after)),
                                &projects,
                            )
                        }
                        EntryStatus::Change(Change::Type { .. }) => {
                            let after = worktree(rela_path.as_ref());
                            self.file(
                                FileStatus::TypeChanged,
                                Some((rela_path.clone(), blob)),
                                Some((rela_path, after)),
                                &projects,
                            )
                        }
                        EntryStatus::IntentToAdd => {
                            let after = worktree(rela_path.as_ref());
                            self.file(FileStatus::Added, None, Some((rela_path, after)), &projects)
                        }
                        // Only the file's stat changed, or a submodule moved: no text changed.
                        _ => continue,
                    }
                }
                Item::DirectoryContents { entry, .. }
                    if entry.status == gix::dir::entry::Status::Untracked
                        && entry.disk_kind == Some(gix::dir::entry::Kind::File) =>
                {
                    let after = worktree(entry.rela_path.as_ref());
                    self.file(
                        FileStatus::Added,
                        None,
                        Some((entry.rela_path, after)),
                        &projects,
                    )
                }
                _ => continue,
            };
            files.push(file);
        }
        files.sort_by(|a, b| a.path().cmp(b.path()));
        Ok(ChangeSet { files })
    }

    fn tree_files(&self, tree: gix::ObjectId) -> Arc<TreeFiles> {
        Arc::new(TreeFiles {
            repo: self.repo.clone(),
            tree,
        })
    }

    fn file(
        &self,
        status: FileStatus,
        before: Option<(gix::bstr::BString, Source)>,
        after: Option<(gix::bstr::BString, Source)>,
        projects: &(
            Arc<dyn ProjectFiles + Send + Sync>,
            Arc<dyn ProjectFiles + Send + Sync>,
        ),
    ) -> FileChange {
        let split = |side: Option<(gix::bstr::BString, Source)>| match side {
            Some((path, source)) => (Some(path_of(path.as_ref())), Some(source)),
            None => (None, None),
        };
        let (before_path, before) = split(before);
        let (after_path, after) = split(after);
        FileChange {
            status,
            before_path,
            after_path,
            before,
            after,
            repo: self.repo.clone(),
            projects: projects.clone(),
            diff: OnceLock::new(),
        }
    }
}

/// A repository path as a filesystem path. git stores bytes; one that is not valid on this
/// platform keeps its bytes read as UTF-8, lossily, rather than failing the whole change set.
fn path_of(path: &gix::bstr::BStr) -> PathBuf {
    gix::path::from_bstr(path)
        .map(std::borrow::Cow::into_owned)
        .unwrap_or_else(|_| PathBuf::from(path.to_string()))
}

fn head_tree(repo: &gix::Repository) -> Result<gix::ObjectId, Error> {
    // An unborn branch has no commit yet: everything in the index is new.
    match repo.head_tree_id() {
        Ok(id) => Ok(id.detach()),
        Err(_) if repo.head().is_ok_and(|h| h.is_unborn()) => Ok(repo.empty_tree().id),
        Err(e) => Err(git_error(e)),
    }
}

/// How a file changed between the two sides of a [`ChangeSet`].
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    /// Moved to another path, perhaps with edits.
    Renamed,
    /// Copied to another path, the original kept.
    Copied,
    /// A file became a symlink or the other way around.
    TypeChanged,
}

/// Where one side of a file is read from.
#[derive(Debug, Clone)]
enum Source {
    Blob(gix::ObjectId),
    Worktree(PathBuf),
}

/// The files that changed between two states of a repository.
#[derive(Debug)]
pub struct ChangeSet {
    files: Vec<FileChange>,
}

impl ChangeSet {
    /// Every changed file, text or not.
    pub fn files(&self) -> &[FileChange] {
        &self.files
    }

    /// Every changed definition in every text file of the change set, file by file. A file that
    /// could not be read as text is skipped; [`FileChange::diff`] says why.
    pub fn changed_definitions(&self) -> impl Iterator<Item = (&FileChange, &DefinitionChange)> {
        self.files.iter().flat_map(|file| {
            file.diff()
                .ok()
                .into_iter()
                .flat_map(move |diff| diff.changed_definitions().map(move |change| (file, change)))
        })
    }
}

/// One changed file of a [`ChangeSet`].
pub struct FileChange {
    status: FileStatus,
    before_path: Option<PathBuf>,
    after_path: Option<PathBuf>,
    before: Option<Source>,
    after: Option<Source>,
    repo: gix::ThreadSafeRepository,
    projects: (
        Arc<dyn ProjectFiles + Send + Sync>,
        Arc<dyn ProjectFiles + Send + Sync>,
    ),
    diff: OnceLock<Result<Diff, Error>>,
}

impl std::fmt::Debug for FileChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FileChange")
            .field("status", &self.status)
            .field("before_path", &self.before_path)
            .field("after_path", &self.after_path)
            .finish_non_exhaustive()
    }
}

impl FileChange {
    pub fn status(&self) -> FileStatus {
        self.status
    }

    /// The path in the before side, relative to the repository root; `None` for an added file.
    pub fn before_path(&self) -> Option<&Path> {
        self.before_path.as_deref()
    }

    /// The path in the after side; `None` for a deleted file.
    pub fn after_path(&self) -> Option<&Path> {
        self.after_path.as_deref()
    }

    /// The after side's path, or the before side's for a deleted file.
    pub fn path(&self) -> &Path {
        self.after_path
            .as_deref()
            .or(self.before_path.as_deref())
            .expect("a changed file has a path")
    }

    /// The file's diff, read and computed on the first call. An added or deleted file diffs
    /// against an empty side in its own language.
    pub fn diff(&self) -> Result<&Diff, &Error> {
        self.diff
            .get_or_init(|| {
                let before = self.text(self.before.as_ref(), self.before_path.as_deref())?;
                let after = self.text(self.after.as_ref(), self.after_path.as_deref())?;
                let path = self.path();
                let (before_path, after_path) = (
                    self.before_path.as_deref().unwrap_or(path),
                    self.after_path.as_deref().unwrap_or(path),
                );
                Ok(Diff::from_texts(&before, &after, before_path, after_path)
                    .with_project_files(self.projects.0.clone(), self.projects.1.clone()))
            })
            .as_ref()
    }

    fn text(&self, source: Option<&Source>, path: Option<&Path>) -> Result<String, Error> {
        let Some(source) = source else {
            return Ok(String::new());
        };
        let path = path.unwrap_or(self.path()).to_path_buf();
        let bytes = match source {
            Source::Blob(id) => {
                let repo = self.repo.to_thread_local();
                repo.find_object(*id).map_err(git_error)?.detach().data
            }
            Source::Worktree(file) => std::fs::read(file).map_err(|source| Error::Io {
                path: file.clone(),
                source,
            })?,
        };
        decode_text(&bytes).ok_or(Error::NotText { path })
    }
}

/// A tree's files, for qualified names.
struct TreeFiles {
    repo: gix::ThreadSafeRepository,
    tree: gix::ObjectId,
}

impl TreeFiles {
    fn blob(&self, path: &Path) -> Option<Vec<u8>> {
        let repo = self.repo.to_thread_local();
        let tree = repo.find_tree(self.tree).ok()?;
        let entry = tree.lookup_entry_by_path(path).ok()??;
        if !entry.mode().is_blob() {
            return None;
        }
        Some(entry.object().ok()?.detach().data)
    }
}

impl ProjectFiles for TreeFiles {
    fn is_file(&self, path: &Path) -> bool {
        self.blob(path).is_some()
    }

    fn read(&self, path: &Path) -> Option<String> {
        String::from_utf8(self.blob(path)?).ok()
    }
}

/// The working tree's files, for paths relative to its root.
struct WorkdirFiles {
    root: PathBuf,
}

impl ProjectFiles for WorkdirFiles {
    fn is_file(&self, path: &Path) -> bool {
        self.root.join(path).is_file()
    }

    fn read(&self, path: &Path) -> Option<String> {
        std::fs::read_to_string(self.root.join(path)).ok()
    }
}

#[cfg(test)]
mod tests;
