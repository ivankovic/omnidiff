/*  This file is part of the OmniDiff code diffing tool.
 *
 *  Copyright (C) 2026 Marko Ivankovic
 *
 *  This program is free software: you can redistribute it and/or modify
 *  it under the terms of the GNU Affero General Public License as published
 *  by the Free Software Foundation, either version 3 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program. If not, see <https://www.gnu.org/licenses/>.
 */

//! What OmniDiff makes of each changed file of one repository: text, content of a family it
//! diffs (`diff::content::Family`), or binary it can only call "differs". For
//! `research/analysis/content_census.py`, which lists the changes (one `commit<TAB>path` per line
//! on stdin) and counts the answers (`path<TAB>outcome` per line on stdout) over the corpus.
//!
//! Every outcome is OmniDiff's own verdict, from the bytes, not git's: a Latin-1 source file git
//! counts as text is binary here (`code::decode_text` cannot read it), and a UTF-16 one git counts
//! as binary is text.
//!
//! * `text`: every side present reads as text.
//! * `<family>` (`pictures`, `fonts`, ...): a pair of that family's content that diffs. With
//!   `--diff` the diff is run, so this is a measurement, not a sniff; `failed:<family>`,
//!   `panicked:<family>` and `timeout:<family>` are pairs that sniff as the family but do not diff
//!   within [`DIFF_SECONDS`].
//! * `binary`: anything else, including a pair whose sides are content of different kinds.
//!
//! A side is the blob at `path` in the commit and in its first parent; a file added, deleted or
//! renamed has one side, and is judged by it. `missing` marks a line whose commit or path is not
//! in the repository.

use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use git2::{Oid, Repository};

use omnidiff::diff::content::{self, Engine};

/// How long one content diff may take before it counts as `timeout:<family>`.
const DIFF_SECONDS: u64 = 60;

#[derive(Parser)]
struct Args {
    /// The repository the changes are in.
    #[arg(long)]
    repo: PathBuf,

    /// Run the content diff of every content pair, not only recognise it.
    #[arg(long, default_value_t = false)]
    diff: bool,
}

/// The blob at `path` in `commit`'s tree, if there is one.
fn blob(repo: &Repository, commit: &git2::Commit, path: &Path) -> Option<Vec<u8>> {
    let entry = commit.tree().ok()?.get_path(path).ok()?;
    Some(repo.find_blob(entry.id()).ok()?.content().to_vec())
}

/// The outcome of one change (see the module doc).
fn outcome(repo: &Repository, commit: &str, path: &str, diff: bool) -> String {
    let Some(commit) = Oid::from_str(commit)
        .ok()
        .and_then(|oid| repo.find_commit(oid).ok())
    else {
        return "missing".to_string();
    };
    let path = Path::new(path);
    let after = blob(repo, &commit, path);
    let before = commit
        .parent(0)
        .ok()
        .and_then(|parent| blob(repo, &parent, path));
    if before.is_none() && after.is_none() {
        return "missing".to_string();
    }
    let (before, after) = (before.unwrap_or_default(), after.unwrap_or_default());
    if [&before, &after]
        .iter()
        .all(|side| omnidiff::code::decode_text(side).is_some())
    {
        return "text".to_string();
    }
    let Some(engine) = content::pair_kind(&before, &after) else {
        return "binary".to_string();
    };
    let family = match engine {
        Engine::Picture => {
            let side = if before.is_empty() { &after } else { &before };
            content::sniff(side).map_or(content::Family::Pictures, |format| format.family())
        }
        Engine::Container(family) => family,
    }
    .name();
    if !diff {
        return family.to_string();
    }
    // On a thread of its own, so a pathological file is a timeout rather than a hung census; a
    // thread that never returns is left behind, and dies with the process.
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = std::panic::catch_unwind(|| content::diff(&before, &after));
        let _ = sender.send(result);
    });
    match receiver.recv_timeout(Duration::from_secs(DIFF_SECONDS)) {
        Ok(Ok(Ok(Some(_)))) => family.to_string(),
        Ok(Ok(_)) => format!("failed:{family}"),
        Ok(Err(_)) => format!("panicked:{family}"),
        Err(_) => format!("timeout:{family}"),
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    let repo = Repository::open(&args.repo)?;
    // A panicking diff is an outcome, not news on stderr for every one of them.
    std::panic::set_hook(Box::new(|_| {}));
    let mut out = BufWriter::new(std::io::stdout().lock());
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let Some((commit, path)) = line.split_once('\t') else {
            continue;
        };
        writeln!(out, "{path}\t{}", outcome(&repo, commit, path, args.diff))?;
        out.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A repository with one commit after a root: `a.txt` edited, `b.png` added, `c.bin` edited.
    fn repository() -> (tempfile::TempDir, Repository, String) {
        let dir = tempfile::tempdir().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let png = {
            let mut bytes = Vec::new();
            image::RgbaImage::from_pixel(2, 2, image::Rgba([1, 2, 3, 255]))
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        let commit = |files: &[(&str, &[u8])], parent: Option<Oid>| {
            let mut builder = repo.treebuilder(None).unwrap();
            for (name, bytes) in files {
                let blob = repo.blob(bytes).unwrap();
                builder.insert(name, blob, 0o100644).unwrap();
            }
            let tree = repo.find_tree(builder.write().unwrap()).unwrap();
            let signature = git2::Signature::now("t", "t@example.com").unwrap();
            let parents: Vec<git2::Commit> = parent
                .map(|oid| repo.find_commit(oid).unwrap())
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            repo.commit(None, &signature, &signature, "c", &tree, &parents)
                .unwrap()
        };
        let root = commit(&[("a.txt", b"one\n"), ("c.bin", &[0xFF, 0x00, 0x01])], None);
        let head = commit(
            &[
                ("a.txt", b"two\n"),
                ("b.png", &png),
                ("c.bin", &[0xFF, 0x00, 0x02]),
            ],
            Some(root),
        );
        (dir, repo, head.to_string())
    }

    #[test]
    fn each_change_gets_omnidiffs_own_verdict() {
        let (_dir, repo, head) = repository();
        assert_eq!(outcome(&repo, &head, "a.txt", true), "text");
        assert_eq!(outcome(&repo, &head, "b.png", true), "pictures");
        assert_eq!(outcome(&repo, &head, "b.png", false), "pictures");
        assert_eq!(outcome(&repo, &head, "c.bin", true), "binary");
        assert_eq!(outcome(&repo, &head, "nowhere", true), "missing");
        assert_eq!(outcome(&repo, "0123", "a.txt", true), "missing");
    }
}
