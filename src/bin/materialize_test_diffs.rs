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
//! Turns the pointers sampled by `sample_test_diffs` into `before.<ext>.test` / `after.<ext>.test`
//! fixtures under `src/test/data/samples/`, not `src/test/data/diffs/`: every test sweeps
//! `diffs/` unfiltered and real files are expensive to diff, so a fixture enters `diffs/` only when
//! a human promotes it from `human_solver`.
//!
//! A row whose `before_path` column is set (the `crosslang` dataset: a file moved to another
//! language) takes its before side from that path, and names `before.<ext>.test` after it.
//!
//! Each fixture also gets a `source.json` (its `sample.csv` row, which `human_solver` reads back
//! on promotion) and a `README.md` (see `omnidiff::stats::license`) recording provenance and the
//! license the content is actually under: it is someone else's code, not covered by omnidiff's
//! AGPL-3.0.
//!
//! Safe to re-run: byte-identical content is left alone and a missing README.md is backfilled.
//!
//! `--changesets` materialises the rows of `src/test/data/changesets.csv` (`sample_test_diffs
//! --changesets`) instead, under `src/test/data/samples/changesets/<name>/`: `before/` and
//! `after/` hold every file the changeset changes at its repository path with `.test` appended -
//! an added file only under `after/`, a removed one only under `before/` - and each untouched
//! file a signal points at under both. Which files those are is `stats::changeset::load_changes`'s
//! say, as it was the sampler's: symbolic links and submodules are skipped, never followed, and a
//! path that is not plain and relative is refused. `source.json` and `README.md` are as above,
//! the first written through [`ChangesetSource`]. Re-running leaves a fixture that already holds
//! exactly these files alone, and refuses one that holds anything else rather than overwrite it.
use anyhow::{Result, bail};
use clap::Parser;
use git2::{Oid, Repository, Tree};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

use omnidiff::stats::changeset;
use omnidiff::stats::license;

const DEFAULT_REPO_ROOTS: &[&str] = &[
    "/var/tmp/research/tiny/repositories",
    "/var/tmp/research/small/repositories",
    "/var/tmp/research/full/repositories",
];

#[derive(Parser)]
struct Args {
    /// CSV produced by `sample_test_diffs` (language, repository, commit, path).
    #[arg(long)]
    sample_csv: Option<PathBuf>,

    /// Root directory to search for a `<repository>` checkout. May be repeated; searched in
    /// order, first match wins. Defaults to the tiny/small/full research checkouts.
    #[arg(long)]
    repos_dir: Vec<PathBuf>,

    /// Directory to write `<name>/{before,after}.<ext>.test` fixtures into. Defaults to
    /// `src/test/data/samples/`.
    #[arg(long)]
    output_dir: Option<PathBuf>,

    /// Only materialize rows for this language (matching the CSV's `language` column).
    #[arg(long)]
    language: Option<String>,

    /// Also backfill a missing `README.md` (provenance/license attribution) into already
    /// promoted rows' `src/test/data/diffs/<dataset>/<promoted_to>/` directories. `REJECTED` rows
    /// have no directory and are never backfilled.
    #[arg(long)]
    include_triaged: bool,

    /// Materialize changesets (see the module documentation): `--sample-csv` defaults to
    /// `src/test/data/changesets.csv` and `--output-dir` to `src/test/data/samples/changesets/`.
    #[arg(long)]
    changesets: bool,
}

#[derive(Serialize)]
struct Row {
    language: String,
    repository: String,
    commit: String,
    path: String,
    /// The before side's path when it is not `path`: a file moved to another language (the
    /// `crosslang` dataset). Empty otherwise, and then left out of `source.json`.
    #[serde(skip_serializing_if = "String::is_empty")]
    before_path: String,
    /// Which research dataset this row was sampled from (`sample_test_diffs`'s `--dataset`).
    /// Provenance only: it does not pick which `repos_dir` root is searched.
    dataset: String,
    /// `human_solver`'s triage state (`SAMPLED`/`PROMOTED`/`REJECTED`).
    status: String,
    /// The `diffs/<dataset>/` case name this row was promoted to; empty unless `PROMOTED`.
    #[serde(skip)]
    promoted_to: String,
}

impl Row {
    /// The `before_path` column, `None` when the before side is at `path` too.
    fn moved_from(&self) -> Option<&str> {
        (!self.before_path.is_empty()).then_some(self.before_path.as_str())
    }
}

enum Resolution {
    Create(PathBuf),
    AlreadyPresent(PathBuf),
}

fn default_sample_csv() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("sample.csv")
}

fn default_output_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("samples")
}

fn diffs_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("diffs")
}

fn read_rows(path: &Path) -> Result<Vec<Row>> {
    let mut reader = csv::Reader::from_path(path)?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        rows.push(Row {
            language: record[0].to_string(),
            repository: record[1].to_string(),
            commit: record[2].to_string(),
            path: record[3].to_string(),
            before_path: record.get(9).unwrap_or("").to_string(),
            promoted_to: record[4].to_string(),
            // Rows without the column were all sampled from the small checkout.
            dataset: record.get(5).unwrap_or("small").to_string(),
            // As `sample_test_diffs::default_status`: never `REJECTED` without the column.
            status: record.get(6).map(str::to_string).unwrap_or_else(|| {
                if record[4].is_empty() {
                    "SAMPLED".to_string()
                } else {
                    "PROMOTED".to_string()
                }
            }),
        });
    }
    Ok(rows)
}

fn main() -> Result<()> {
    let args = Args::parse();

    let repo_roots: Vec<PathBuf> = if args.repos_dir.is_empty() {
        DEFAULT_REPO_ROOTS.iter().map(PathBuf::from).collect()
    } else {
        args.repos_dir
    };
    let mut created = 0;
    let mut already_present = 0;
    let mut skipped = 0;

    if args.changesets {
        let sample_csv = args
            .sample_csv
            .unwrap_or_else(|| default_sample_csv().with_file_name("changesets.csv"));
        let output_dir = args
            .output_dir
            .unwrap_or_else(|| default_output_dir().join("changesets"));
        for row in changeset::read_rows(&sample_csv)? {
            if row.status != "SAMPLED" {
                continue;
            }
            match materialize_changeset(&row, &repo_roots, &output_dir) {
                Ok(Resolution::Create(dir)) => {
                    println!("created {:?}", dir);
                    created += 1;
                }
                Ok(Resolution::AlreadyPresent(dir)) => {
                    println!("already present {:?}", dir);
                    already_present += 1;
                }
                Err(e) => {
                    eprintln!("skipping {}@{}: {:?}", row.repository, row.commit, e);
                    skipped += 1;
                }
            }
        }
        println!(
            "Created {}, already present {}, skipped {}",
            created, already_present, skipped
        );
        return Ok(());
    }

    let sample_csv = args.sample_csv.unwrap_or_else(default_sample_csv);
    let output_dir = args.output_dir.unwrap_or_else(default_output_dir);
    let rows = read_rows(&sample_csv)?;

    for row in &rows {
        if let Some(filter) = args.language.as_deref()
            && row.language != filter
        {
            continue;
        }
        let result = if row.status == "SAMPLED" {
            materialize_row(row, &repo_roots, &output_dir)
        } else if args.include_triaged && row.status == "PROMOTED" {
            backfill_promoted_readme(row, &repo_roots)
        } else {
            continue;
        };

        match result {
            Ok(Resolution::Create(dir)) => {
                println!("created {:?}", dir);
                created += 1;
            }
            Ok(Resolution::AlreadyPresent(dir)) => {
                println!("already present {:?}", dir);
                already_present += 1;
            }
            Err(e) => {
                eprintln!(
                    "skipping {}@{} {}: {:?}",
                    row.repository, row.commit, row.path, e
                );
                skipped += 1;
            }
        }
    }

    println!(
        "Created {}, already present {}, skipped {}",
        created, already_present, skipped
    );

    Ok(())
}

fn find_repo_path(roots: &[PathBuf], repository: &str) -> Option<PathBuf> {
    roots
        .iter()
        .map(|root| root.join(repository))
        .find(|candidate| candidate.join(".git").exists())
}

fn blob_text(repo: &Repository, tree: &Tree, path: &Path) -> Result<String> {
    Ok(String::from_utf8(omnidiff::stats::git::blob_bytes(
        repo, tree, path,
    )?)?)
}

/// Directory name: `<language>-x-<repository>-<commit>-<filename>`, all lowercase, with the
/// repository's trailing `.git`, the path's directories, and the file's extension stripped (the
/// extension is carried by the fixture filenames instead). The commit is abbreviated to 8 hex
/// characters.
fn base_name(row: &Row) -> String {
    let language = row.language.to_lowercase();
    let repository = row
        .repository
        .strip_suffix(".git")
        .unwrap_or(&row.repository)
        .to_lowercase();
    let commit = &row.commit[..8.min(row.commit.len())];
    let filename = Path::new(&row.path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_lowercase().replace('.', "-"))
        .unwrap_or_default();

    format!("{language}-x-{repository}-{commit}-{filename}")
}

/// Finds (or claims) the directory to write this row's fixture into: an existing directory with
/// identical content is returned as already present, and one holding different content is
/// skipped for the next of `-2`, `-3`, ...
fn resolve_target(
    output_dir: &Path,
    base_name: &str,
    (before_ext, after_ext): (&str, &str),
    before: &[u8],
    after: &[u8],
) -> Result<Resolution> {
    for attempt in 1..1000 {
        let name = if attempt == 1 {
            base_name.to_string()
        } else {
            format!("{base_name}-{attempt}")
        };
        let dir = output_dir.join(&name);
        if !dir.exists() {
            return Ok(Resolution::Create(dir));
        }

        let before_path = dir.join(format!("before.{before_ext}.test"));
        let after_path = dir.join(format!("after.{after_ext}.test"));
        if let (Ok(existing_before), Ok(existing_after)) =
            (fs::read(&before_path), fs::read(&after_path))
            && existing_before == before
            && existing_after == after
        {
            return Ok(Resolution::AlreadyPresent(dir));
        }
    }
    bail!("too many name collisions for base name {base_name}");
}

/// Writes `README.md` into an already-promoted row's `diffs/<dataset>/<promoted_to>/` directory
/// when it has none (see `Args::include_triaged`). Touches nothing else.
fn backfill_promoted_readme(row: &Row, repo_roots: &[PathBuf]) -> Result<Resolution> {
    if row.promoted_to.is_empty() {
        bail!("row has status PROMOTED but no promoted_to name recorded");
    }
    let dir = diffs_root().join(&row.dataset).join(&row.promoted_to);
    if !dir.is_dir() {
        bail!("promoted case directory {:?} does not exist", dir);
    }

    let readme_path = dir.join("README.md");
    if readme_path.exists() {
        return Ok(Resolution::AlreadyPresent(dir));
    }

    // Best-effort: a commit that has aged out of a shallow clone still gets a README.md, with
    // `unverifiable_reason` saying why, rather than one claiming "no license". `repo_url` is
    // resolved outside the fallible closure because the repository usually outlives the commit.
    let repo = find_repo_path(repo_roots, &row.repository).and_then(|p| Repository::open(p).ok());
    let repo_url = repo.as_ref().and_then(license::origin_remote_url);

    let lookup: Result<Vec<license::LicenseFile>> = (|| {
        let repo = repo.as_ref().ok_or_else(|| {
            anyhow::anyhow!("repository {} not found in any root", row.repository)
        })?;
        let commit = repo.find_commit(Oid::from_str(&row.commit)?)?;
        let tree = commit.tree()?;
        Ok(license::find_license_files(repo, &tree))
    })();

    let (license_files, unverifiable_reason) = match lookup {
        Ok(license_files) => (license_files, None),
        Err(err) => {
            eprintln!(
                "  note: {:?} could not be verified against the local checkout ({:#}); writing a \
                 placeholder README.md instead",
                dir, err
            );
            (Vec::new(), Some(format!("{err:#}")))
        }
    };

    let readme = license::render_readme(
        repo_url.as_deref(),
        &row.repository,
        &row.commit,
        row.moved_from(),
        &row.path,
        &row.dataset,
        &license_files,
        unverifiable_reason.as_deref(),
    );
    fs::write(&readme_path, readme)?;

    Ok(Resolution::Create(dir))
}

fn materialize_row(row: &Row, repo_roots: &[PathBuf], output_dir: &Path) -> Result<Resolution> {
    let repo_path = find_repo_path(repo_roots, &row.repository)
        .ok_or_else(|| anyhow::anyhow!("repository {} not found in any root", row.repository))?;
    let repo = Repository::open(&repo_path)?;

    let commit = repo.find_commit(Oid::from_str(&row.commit)?)?;
    if commit.parents().len() != 1 {
        bail!("commit {} does not have exactly one parent", row.commit);
    }
    let parent_tree = commit.parent(0)?.tree()?;
    let tree = commit.tree()?;

    let path = Path::new(&row.path);
    let before_path = Path::new(row.moved_from().unwrap_or(&row.path));
    // Content (a picture, a font, ...) is copied byte for byte, and so is text in UTF-16 or UTF-32
    // (`sample_test_diffs --encodings`); other code must be UTF-8, since every tool reads it so.
    let content = omnidiff::diff::content::Family::from_name(&row.dataset).is_some();
    let (before, after) = if content || row.dataset == "encodings" {
        (
            omnidiff::stats::git::blob_bytes(&repo, &parent_tree, before_path)?,
            omnidiff::stats::git::blob_bytes(&repo, &tree, path)?,
        )
    } else {
        (
            blob_text(&repo, &parent_tree, before_path)?.into_bytes(),
            blob_text(&repo, &tree, path)?.into_bytes(),
        )
    };

    // Content can be extensionless (an X cursor named `wait`); its kind is in its bytes.
    let ext_of = |path: &Path| match path.extension() {
        Some(ext) => Ok(ext.to_string_lossy().into_owned()),
        None if content => Ok("bin".to_string()),
        None => Err(anyhow::anyhow!("path {} has no extension", path.display())),
    };
    let (before_ext, after_ext) = (ext_of(before_path)?, ext_of(path)?);

    let base = base_name(row);
    let resolution = resolve_target(
        output_dir,
        &base,
        (&before_ext, &after_ext),
        &before,
        &after,
    )?;

    let dir = match &resolution {
        Resolution::Create(dir) => {
            fs::create_dir_all(dir)?;
            fs::write(dir.join(format!("before.{before_ext}.test")), &before)?;
            fs::write(dir.join(format!("after.{after_ext}.test")), &after)?;
            fs::write(dir.join("source.json"), serde_json::to_string_pretty(row)?)?;
            dir
        }
        Resolution::AlreadyPresent(dir) => dir,
    };

    // Never overwritten, so a hand-corrected README.md survives a re-run.
    let readme_path = dir.join("README.md");
    if !readme_path.exists() {
        let license_files = license::find_license_files(&repo, &tree);
        let repo_url = license::origin_remote_url(&repo);
        let readme = license::render_readme(
            repo_url.as_deref(),
            &row.repository,
            &row.commit,
            row.moved_from(),
            &row.path,
            &row.dataset,
            &license_files,
            None,
        );
        fs::write(&readme_path, readme)?;
    }

    Ok(resolution)
}

/// A changeset fixture's `source.json`, as the design of 2026-11-12 has it (AGENT_LOG.md,
/// "changeset ground truth"): where the changeset came from, every file it holds, and what the
/// VCS says happened - hints, never truth.
///
/// To be replaced at merge by the shared type in `src/test/helper/human_changeset.rs` (branch
/// `changesets`), which did not exist when this was written; this is the only place that writes
/// the file.
#[derive(Serialize)]
struct ChangesetSource {
    repository: String,
    commit: String,
    parent: String,
    stratum: String,
    dataset: String,
    status: String,
    files: Vec<ChangesetSourceFile>,
    vcs_hints: Vec<ChangesetSourceHint>,
}

/// A file of a changeset by its repository path on each side it exists on: the fixture holds
/// `before/<path>.test` and `after/<path>.test`.
#[derive(Serialize)]
struct ChangesetSourceFile {
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<String>,
    /// True for a file the commit does not change, held because a signal points at it.
    untouched: bool,
}

/// One line of `git diff --name-status -M` about a file the fixture holds.
#[derive(Serialize)]
struct ChangesetSourceHint {
    /// `A`, `D`, `M`, `R`, `C` or `T`.
    status: String,
    path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    old_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    similarity: Option<u8>,
}

/// The regular files under `dir`, by path relative to it; an error if it holds anything that is
/// neither a directory nor a regular file, which a fixture never does.
fn files_under(dir: &Path) -> Result<Vec<(PathBuf, Vec<u8>)>> {
    let mut found = Vec::new();
    if !dir.exists() {
        return Ok(found);
    }
    for entry in walkdir::WalkDir::new(dir).follow_links(false) {
        let entry = entry?;
        let kind = entry.file_type();
        if kind.is_dir() {
            continue;
        }
        if !kind.is_file() {
            bail!("{:?} is not a regular file", entry.path());
        }
        found.push((
            entry.path().strip_prefix(dir)?.to_path_buf(),
            fs::read(entry.path())?,
        ));
    }
    found.sort();
    Ok(found)
}

/// Writes the fixture of one `changesets.csv` row (see the module documentation).
fn materialize_changeset(
    row: &changeset::Row,
    repo_roots: &[PathBuf],
    output_dir: &Path,
) -> Result<Resolution> {
    if row.name.contains('/') || !changeset::is_safe_path(&row.name) {
        bail!("{:?} is not a fixture name", row.name);
    }
    let repo_path = find_repo_path(repo_roots, &row.repository)
        .ok_or_else(|| anyhow::anyhow!("repository {} not found in any root", row.repository))?;
    let repo = Repository::open(&repo_path)?;
    let commit = repo.find_commit(Oid::from_str(&row.commit)?)?;
    let parent = repo.find_commit(Oid::from_str(&row.parent)?)?;
    let (before_tree, tree) = (parent.tree()?, commit.tree()?);

    let files = changeset::load_changes(&repo, &before_tree, &tree)?
        .map_err(|skip| anyhow::anyhow!("no longer a changeset: {}", skip.name()))?;
    let mut source_files = Vec::new();
    let mut readme_files = Vec::new();
    // Relative to the fixture's directory.
    let mut wanted: Vec<(PathBuf, Vec<u8>)> = Vec::new();
    let test_path = |side: &str, path: &str| Path::new(side).join(format!("{path}.test"));
    for file in files {
        readme_files.push((
            file.path.clone(),
            match (&file.before, &file.after) {
                (Some(_), Some(_)) => "modified",
                (None, _) => "added",
                (Some(_), None) => "removed",
            },
        ));
        source_files.push(ChangesetSourceFile {
            before: file.before.as_ref().map(|_| file.path.clone()),
            after: file.after.as_ref().map(|_| file.path.clone()),
            untouched: false,
        });
        if let Some(blob) = file.before {
            wanted.push((test_path("before", &file.path), blob.bytes));
        }
        if let Some(blob) = file.after {
            wanted.push((test_path("after", &file.path), blob.bytes));
        }
    }
    for path in row.untouched() {
        let bytes = changeset::regular_blob(&repo, &before_tree, path)?;
        if changeset::regular_blob(&repo, &tree, path)? != bytes {
            bail!("{path:?} is recorded as untouched, and the commit changes it");
        }
        readme_files.push((path.to_string(), "untouched"));
        source_files.push(ChangesetSourceFile {
            before: Some(path.to_string()),
            after: Some(path.to_string()),
            untouched: true,
        });
        wanted.push((test_path("before", path), bytes.clone()));
        wanted.push((test_path("after", path), bytes));
    }
    wanted.sort();
    // `a` beside `a.test/b` would need `a.test` to be a file and a directory.
    for (path, _) in &wanted {
        if wanted
            .iter()
            .any(|(other, _)| other != path && other.starts_with(path))
        {
            bail!("{path:?} is both a file and a directory of the fixture");
        }
    }

    let dir = output_dir.join(&row.name);
    let resolution = if dir.exists() {
        let mut present = files_under(&dir.join("before"))?
            .into_iter()
            .map(|(path, bytes)| (Path::new("before").join(path), bytes))
            .collect::<Vec<_>>();
        present.extend(
            files_under(&dir.join("after"))?
                .into_iter()
                .map(|(path, bytes)| (Path::new("after").join(path), bytes)),
        );
        present.sort();
        if present != wanted {
            bail!(
                "{dir:?} exists and does not hold this changeset's files; remove it to \
                 materialize the row again"
            );
        }
        Resolution::AlreadyPresent(dir.clone())
    } else {
        for (path, bytes) in &wanted {
            let target = dir.join(path);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(target, bytes)?;
        }
        Resolution::Create(dir.clone())
    };

    // Neither is overwritten: a hand-corrected README.md survives a re-run, and `source.json` is
    // `human_solver`'s to update once the changeset is triaged.
    let missing = |path: &Path| fs::symlink_metadata(path).is_err();
    let source_path = dir.join("source.json");
    if missing(&source_path) {
        let held = |path: &str| readme_files.iter().any(|(file, _)| file == path);
        let vcs_hints = changeset::vcs_hints(&repo, parent.id(), commit.id())?
            .into_iter()
            .filter(|hint| held(&hint.path) || hint.old_path.as_deref().is_some_and(held))
            .map(|hint| ChangesetSourceHint {
                status: hint.status.to_string(),
                path: hint.path,
                old_path: hint.old_path,
                similarity: hint.similarity,
            })
            .collect();
        let source = ChangesetSource {
            repository: row.repository.clone(),
            commit: row.commit.clone(),
            parent: row.parent.clone(),
            stratum: row.stratum.clone(),
            dataset: row.dataset.clone(),
            status: row.status.clone(),
            files: source_files,
            vcs_hints,
        };
        fs::write(&source_path, serde_json::to_string_pretty(&source)?)?;
    }
    let readme_path = dir.join("README.md");
    if missing(&readme_path) {
        let license_files = license::find_license_files(&repo, &tree);
        let repo_url = license::origin_remote_url(&repo);
        let readme = license::render_changeset_readme(
            repo_url.as_deref(),
            &license::ChangesetProvenance {
                repository: &row.repository,
                commit: &row.commit,
                parent: &row.parent,
                stratum: &row.stratum,
                dataset: &row.dataset,
                files: &readme_files,
            },
            &license_files,
        );
        fs::write(&readme_path, readme)?;
    }

    Ok(resolution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Sort;
    use omnidiff::test::helper;
    use tempfile::tempdir;

    /// Finds the first single-parent commit in `repo_path` and the path of a file it modified.
    fn find_a_modified_file(repo_path: &Path) -> (String, String) {
        let repo = Repository::open(repo_path).unwrap();
        let mut walk = repo.revwalk().unwrap();
        walk.set_sorting(Sort::TIME).unwrap();
        walk.push_head().unwrap();

        for id in walk {
            let id = id.unwrap();
            let commit = repo.find_commit(id).unwrap();
            if commit.parents().len() != 1 {
                continue;
            }
            let parent_tree = commit.parent(0).unwrap().tree().unwrap();
            let tree = commit.tree().unwrap();
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&tree), None)
                .unwrap();
            for delta in diff.deltas() {
                if delta.status() == git2::Delta::Modified
                    && let Some(path) = delta.new_file().path()
                {
                    return (id.to_string(), path.to_string_lossy().into_owned());
                }
            }
        }
        panic!("handmade repository has no single-parent modified-file commit");
    }

    fn row_for(repo_path: &Path, commit: String, path: String) -> Row {
        Row {
            language: "Rust".to_string(),
            repository: repo_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            commit,
            path,
            before_path: String::new(),
            dataset: "small".to_string(),
            status: "SAMPLED".to_string(),
            promoted_to: String::new(),
        }
    }

    #[test]
    fn base_name_matches_language_x_repository_commit_filename_scheme() {
        let row = Row {
            language: "Rust".to_string(),
            repository: "GyulyVGC-sniffnet.git".to_string(),
            commit: "1a70be36eb3d50a2b7248a76056fe9b3c2f71c82".to_string(),
            path: "src/gui/pages/overview_page.rs".to_string(),
            before_path: String::new(),
            dataset: "small".to_string(),
            status: "SAMPLED".to_string(),
            promoted_to: String::new(),
        };
        assert_eq!(
            base_name(&row),
            "rust-x-gyulyvgc-sniffnet-1a70be36-overview_page"
        );
    }

    #[test]
    fn materializes_a_real_pair_from_handmade_repository() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;
        let (commit, path) = find_a_modified_file(&repo_path);
        let row = row_for(&repo_path, commit, path);

        let repo_roots = vec![repo_path.parent().unwrap().to_path_buf()];
        let output_dir = tempdir()?;

        let resolution = materialize_row(&row, &repo_roots, output_dir.path())?;
        let dir = match resolution {
            Resolution::Create(dir) => dir,
            Resolution::AlreadyPresent(_) => panic!("expected a fresh directory"),
        };

        assert!(dir.join("before.rs.test").exists());
        assert!(dir.join("after.rs.test").exists());
        let before = fs::read_to_string(dir.join("before.rs.test"))?;
        let after = fs::read_to_string(dir.join("after.rs.test"))?;
        assert_ne!(before, after);

        let source: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("source.json"))?)?;
        assert_eq!(source["language"], "Rust");
        assert_eq!(source["repository"], row.repository);
        assert_eq!(source["commit"], row.commit);
        assert_eq!(source["path"], row.path);
        assert_eq!(source["dataset"], row.dataset);

        Ok(())
    }

    #[test]
    fn a_move_to_another_language_takes_each_side_from_its_own_path() -> Result<()> {
        let root = tempdir()?;
        let repo_path = root.path().join("example-repo");
        let repo = Repository::init(&repo_path)?;
        let signature = git2::Signature::now("test", "test@example.com")?;
        // A one-file tree: a commit that deletes the parent's file and adds `file` in its place.
        let commit_tree = |file: &str, content: &str| -> Result<Oid> {
            let mut builder = repo.treebuilder(None)?;
            builder.insert(file, repo.blob(content.as_bytes())?, 0o100644)?;
            Ok(builder.write()?)
        };
        let java = commit_tree("Greeter.java", "class Greeter {}\n")?;
        let parent = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Java",
            &repo.find_tree(java)?,
            &[],
        )?;
        let kotlin = commit_tree("Greeter.kt", "class Greeter\n")?;
        let commit = repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Kotlin",
            &repo.find_tree(kotlin)?,
            &[&repo.find_commit(parent)?],
        )?;

        let mut row = row_for(&repo_path, commit.to_string(), "Greeter.kt".to_string());
        row.language = "Java-Kotlin".to_string();
        row.dataset = "crosslang".to_string();
        row.before_path = "Greeter.java".to_string();
        let output_dir = tempdir()?;
        let dir = match materialize_row(&row, &[root.path().to_path_buf()], output_dir.path())? {
            Resolution::Create(dir) => dir,
            Resolution::AlreadyPresent(_) => panic!("expected a fresh directory"),
        };

        assert_eq!(
            fs::read_to_string(dir.join("before.java.test"))?,
            "class Greeter {}\n"
        );
        assert_eq!(
            fs::read_to_string(dir.join("after.kt.test"))?,
            "class Greeter\n"
        );
        let source: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("source.json"))?)?;
        assert_eq!(source["before_path"], "Greeter.java");
        assert_eq!(source["path"], "Greeter.kt");
        assert!(fs::read_to_string(dir.join("README.md"))?.contains("**Before file:**"));
        // A same-path row's source.json has no `before_path` (see `Row::before_path`).
        row.before_path.clear();
        assert!(serde_json::to_value(&row)?.get("before_path").is_none());

        Ok(())
    }

    #[test]
    fn rerun_is_idempotent_and_distinct_content_gets_a_numbered_suffix() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;
        let (commit, path) = find_a_modified_file(&repo_path);
        let row = row_for(&repo_path, commit, path);
        let repo_roots = vec![repo_path.parent().unwrap().to_path_buf()];
        let output_dir = tempdir()?;

        let first = materialize_row(&row, &repo_roots, output_dir.path())?;
        let first_dir = match first {
            Resolution::Create(dir) => dir,
            Resolution::AlreadyPresent(_) => panic!("expected a fresh directory on first run"),
        };

        let second = materialize_row(&row, &repo_roots, output_dir.path())?;
        match second {
            Resolution::AlreadyPresent(dir) => assert_eq!(dir, first_dir),
            Resolution::Create(_) => panic!("expected the second run to detect existing content"),
        }

        // Different content in the slot forces a name collision.
        fs::write(first_dir.join("before.rs.test"), "not the real content")?;
        let third = materialize_row(&row, &repo_roots, output_dir.path())?;
        match third {
            Resolution::Create(dir) => {
                assert_eq!(
                    dir,
                    first_dir.with_file_name(format!(
                        "{}-2",
                        first_dir.file_name().unwrap().to_string_lossy()
                    ))
                );
            }
            Resolution::AlreadyPresent(_) => panic!("content was tampered with, should not match"),
        }

        Ok(())
    }

    #[test]
    fn rerun_keeps_a_hand_edited_readme_and_backfills_a_missing_one() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;
        let (commit, path) = find_a_modified_file(&repo_path);
        let row = row_for(&repo_path, commit, path);
        let repo_roots = vec![repo_path.parent().unwrap().to_path_buf()];
        let output_dir = tempdir()?;

        let dir = match materialize_row(&row, &repo_roots, output_dir.path())? {
            Resolution::Create(dir) => dir,
            Resolution::AlreadyPresent(_) => panic!("expected a fresh directory"),
        };
        let readme = dir.join("README.md");
        assert!(readme.exists());

        fs::write(&readme, "hand-corrected")?;
        materialize_row(&row, &repo_roots, output_dir.path())?;
        assert_eq!(fs::read_to_string(&readme)?, "hand-corrected");

        fs::remove_file(&readme)?;
        materialize_row(&row, &repo_roots, output_dir.path())?;
        assert!(readme.exists());

        Ok(())
    }

    /// Three commits: a tree, then one that moves a block from `src/a.rs` to a new `src/b.rs`,
    /// removes `old.rs`, copies from the untouched `lib/source.rs` and retargets a link; the
    /// row of the second, as the sampler would write it.
    fn changeset_repository(root: &Path) -> Result<changeset::Row> {
        let repo_path = root.join("example-repo");
        let repo = Repository::init(&repo_path)?;
        let signature = git2::Signature::now("test", "test@example.com")?;
        let lines = |tag: &str, count: usize| -> String {
            (0..count)
                .map(|i| format!("let {tag}_{i} = compute_{tag}(input_{i}) + offset_{tag}_{i};\n"))
                .collect()
        };
        let commit = |parent: Option<Oid>, entries: &[(&str, String, u32)]| -> Result<Oid> {
            let mut index = git2::Index::new()?;
            for (path, text, mode) in entries {
                index.add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: *mode,
                    uid: 0,
                    gid: 0,
                    file_size: 0,
                    id: repo.blob(text.as_bytes())?,
                    flags: 0,
                    flags_extended: 0,
                    path: path.as_bytes().to_vec(),
                })?;
            }
            let tree = repo.find_tree(index.write_tree_to(&repo)?)?;
            let parents: Vec<git2::Commit> = parent
                .map(|id| repo.find_commit(id))
                .transpose()?
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            Ok(repo.commit(None, &signature, &signature, "c", &tree, &parents)?)
        };
        let (keep, moved, shared) = (lines("keep", 6), lines("moved", 6), lines("shared", 12));
        let first = commit(
            None,
            &[
                (
                    "LICENSE",
                    "Permission is hereby granted, free of charge".to_string(),
                    0o100644,
                ),
                ("lib/source.rs", shared.clone(), 0o100644),
                ("src/a.rs", format!("{keep}{moved}"), 0o100644),
                ("old.rs", lines("old", 5), 0o100644),
                ("link", "old.rs".to_string(), 0o120000),
            ],
        )?;
        let second = commit(
            Some(first),
            &[
                (
                    "LICENSE",
                    "Permission is hereby granted, free of charge".to_string(),
                    0o100644,
                ),
                ("lib/source.rs", shared.clone(), 0o100644),
                ("src/a.rs", keep.clone(), 0o100644),
                ("src/b.rs", format!("{moved}{shared}"), 0o100644),
                ("link", "src/a.rs".to_string(), 0o120000),
            ],
        )?;
        Ok(changeset::Row {
            name: changeset::fixture_name("copy", "example-repo", &second.to_string()),
            repository: "example-repo".to_string(),
            commit: second.to_string(),
            parent: first.to_string(),
            stratum: "copy".to_string(),
            signals: "copy;block-move;untouched=lib/source.rs".to_string(),
            files: 3,
            changed_lines: 29,
            main_language: "Rust".to_string(),
            bucket: "2-3".to_string(),
            dataset: "pilot".to_string(),
            status: "SAMPLED".to_string(),
            comment: String::new(),
        })
    }

    #[test]
    fn a_changeset_becomes_two_trees_a_source_and_a_readme() -> Result<()> {
        let root = tempdir()?;
        let row = changeset_repository(root.path())?;
        let output_dir = tempdir()?;
        let roots = [root.path().to_path_buf()];
        let dir = match materialize_changeset(&row, &roots, output_dir.path())? {
            Resolution::Create(dir) => dir,
            Resolution::AlreadyPresent(_) => panic!("expected a fresh directory"),
        };
        assert_eq!(dir.file_name().unwrap().to_string_lossy(), row.name);

        let held = |side: &str| -> Result<Vec<String>> {
            Ok(files_under(&dir.join(side))?
                .into_iter()
                .map(|(path, _)| path.to_string_lossy().into_owned())
                .collect())
        };
        // A removed file only before, an added one only after, the untouched source in both;
        // the link and the unchanged licence in neither.
        assert_eq!(
            held("before")?,
            ["lib/source.rs.test", "old.rs.test", "src/a.rs.test"]
        );
        assert_eq!(
            held("after")?,
            ["lib/source.rs.test", "src/a.rs.test", "src/b.rs.test"]
        );
        assert!(fs::read_to_string(dir.join("after/src/b.rs.test"))?.contains("compute_moved"));

        let source: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(dir.join("source.json"))?)?;
        let mut keys: Vec<&String> = source.as_object().unwrap().keys().collect();
        keys.sort();
        assert_eq!(
            keys,
            [
                "commit",
                "dataset",
                "files",
                "parent",
                "repository",
                "status",
                "stratum",
                "vcs_hints"
            ]
        );
        assert_eq!(source["parent"], row.parent);
        assert_eq!(source["stratum"], "copy");
        assert_eq!(
            source["files"],
            serde_json::json!([
                {"before": "old.rs", "untouched": false},
                {"before": "src/a.rs", "after": "src/a.rs", "untouched": false},
                {"after": "src/b.rs", "untouched": false},
                {"before": "lib/source.rs", "after": "lib/source.rs", "untouched": true},
            ])
        );
        // Git's own account of the files held, the link's change left out.
        assert_eq!(
            source["vcs_hints"],
            serde_json::json!([
                {"status": "D", "path": "old.rs"},
                {"status": "M", "path": "src/a.rs"},
                {"status": "A", "path": "src/b.rs"},
            ])
        );

        let readme = fs::read_to_string(dir.join("README.md"))?;
        assert!(readme.contains(&format!("- **Parent:** `{}`", row.parent)));
        assert!(readme.contains("  - `lib/source.rs` (untouched)"));
        assert!(readme.contains("- `LICENSE` - MIT License"));
        Ok(())
    }

    #[test]
    fn a_changeset_rerun_changes_nothing_and_never_overwrites() -> Result<()> {
        let root = tempdir()?;
        let row = changeset_repository(root.path())?;
        let output_dir = tempdir()?;
        let roots = [root.path().to_path_buf()];
        let Resolution::Create(dir) = materialize_changeset(&row, &roots, output_dir.path())?
        else {
            panic!("expected a fresh directory");
        };
        fs::write(dir.join("README.md"), "hand-corrected")?;
        fs::remove_file(dir.join("source.json"))?;
        match materialize_changeset(&row, &roots, output_dir.path())? {
            Resolution::AlreadyPresent(again) => assert_eq!(again, dir),
            Resolution::Create(_) => panic!("expected the second run to find the fixture"),
        }
        assert_eq!(fs::read_to_string(dir.join("README.md"))?, "hand-corrected");
        assert!(dir.join("source.json").exists(), "backfilled");

        // Anything else in the fixture's trees and the row is refused, not written over.
        fs::write(dir.join("after/src/b.rs.test"), "edited by hand")?;
        assert!(materialize_changeset(&row, &roots, output_dir.path()).is_err());
        assert_eq!(
            fs::read_to_string(dir.join("after/src/b.rs.test"))?,
            "edited by hand"
        );
        Ok(())
    }

    #[test]
    fn a_changeset_row_cannot_write_outside_its_fixture() -> Result<()> {
        let root = tempdir()?;
        let row = changeset_repository(root.path())?;
        let output_dir = tempdir()?;
        let roots = [root.path().to_path_buf()];
        for name in ["../escaped", "a/b", "/tmp/escaped", ".."] {
            let mut bad = row.clone();
            bad.name = name.to_string();
            assert!(
                materialize_changeset(&bad, &roots, output_dir.path()).is_err(),
                "{name}"
            );
        }
        for untouched in ["../../etc/passwd", "/etc/passwd", "link", "src", "src/a.rs"] {
            let mut bad = row.clone();
            bad.signals = format!("copy;untouched={untouched}");
            assert!(
                materialize_changeset(&bad, &roots, output_dir.path()).is_err(),
                "{untouched}"
            );
        }
        assert_eq!(
            fs::read_dir(output_dir.path())?.count(),
            0,
            "nothing was written"
        );
        Ok(())
    }
}
