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
//! Samples (repository, commit, path) pointers to single-file edits as candidate fixtures for
//! `src/test/data/diffs/`. Unlike `sample_code_pairs`, it tops up the existing
//! `src/test/data/sample.csv` to `--count` per language instead of starting over, so it can be
//! re-run against different checkout roots.
//!
//! `--stratified` samples per (language, [`omnidiff::stats::sampling::LOC_BUCKETS`] bucket), and
//! `--count` then means per bucket - unlike `sample_code_pairs --count`, a per-language total.
//!
//! `--content <family>[,<family>...]` samples pairs of content families instead (see `sample_content` and
//! `diff::content::Family`), for the fixtures under `src/test/data/<family>/`: tagged with the
//! family as their dataset, promoted by `human_solver`'s content session with verdicts.
//! `--pictures` is `--content pictures`. The first picture draw, 2026-10-02:
//! `sample_test_diffs --pictures --repos-dir /var/tmp/research/full/repositories
//! --max-commits-per-repo 50 --total 250 --seed 20261002`, then `materialize_test_diffs`.
//!
//! `--changesets` samples whole commits instead, for the multi-file fixtures (see
//! `sample_changesets` and [`omnidiff::stats::changeset`], which says what a changeset holds and
//! how its stratum is decided): it tops `src/test/data/changesets.csv` up to `--count` per
//! (stratum, file-count bucket). `--range-length L` samples L consecutive commits squashed into
//! one changeset, the `range` stratum; `--dry-run` writes nothing and only prints the census,
//! how many commits fall in each cell, and `--explain` adds what each signal saw. The pilot draw,
//! 2026-11-12: `sample_test_diffs --changesets --repos-dir
//! /var/tmp/research/changeset-pilot/repositories --max-commits-per-repo 400 --count 2 --seed
//! 20261112`, then the same with `--count 1 --range-length 5`, then `materialize_test_diffs
//! --changesets`.
use anyhow::{Result, bail};
use clap::Parser;
use git2::Delta;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use omnidiff::anomalous_paths;
use omnidiff::code::language::{language_for_path, language_for_path_and_content, to_treesitter};
use omnidiff::code::{Encoding, Language, decode_text};
use omnidiff::diff::content::{self, Engine, Family, Format, Probe};
use omnidiff::stats::changeset::{self, BlockIndex, FILE_BUCKETS, PRECEDENCE, Skip, Stratum};
use omnidiff::stats::filesystem::{find_git_repositories, for_each_repository};
use omnidiff::stats::git::{
    text_loc_if_in_range, walk_single_parent_commit_diffs, walk_single_parent_commits,
};
use omnidiff::stats::sampling::{Reservoir, loc_bucket};

// The upper bound is the size `stats::expand_from_code` refuses to parse.
const MIN_BYTES: usize = 1;
const MAX_BYTES: usize = 1024 * 1024;

#[derive(Parser)]
struct Args {
    /// Root directory containing checked-out git repositories (or a single repository).
    #[arg(long, default_value = "/var/tmp/research/small/repositories")]
    repos_dir: PathBuf,

    /// Target number of samples per language. Existing rows in the output file already count
    /// towards this; only the shortfall is sampled.
    #[arg(long, default_value_t = 20)]
    count: usize,

    /// Where the (language, repository, commit, path) rows are read from and written to.
    #[arg(long)]
    output: Option<PathBuf>,

    /// RNG seed. Omitted by default so every run draws a fresh sample.
    #[arg(long)]
    seed: Option<u64>,

    /// Restrict sampling to this language (e.g. "Rust"), matching `Language`'s Debug name.
    /// Default tops up every tree-sitter-supported language found short of `--count`.
    #[arg(long)]
    language: Option<String>,

    /// Stop after walking this many commits per repository (most-recent-first). Repeated
    /// shallow fetches can deepen a clone far past its original depth.
    #[arg(long, default_value_t = 1000)]
    max_commits_per_repo: usize,

    /// Which research dataset (tiny/small/full/stratified) new rows are tagged with; decides
    /// where `human_solver` promotes them. Defaults to `--repos-dir`'s parent directory name
    /// (`.../research/small/repositories` -> "small"), or to "stratified" under `--stratified`,
    /// since there it records the sampling method rather than the checkout. A different value
    /// together with `--stratified` is rejected.
    #[arg(long)]
    dataset: Option<String>,

    /// Stratify sampling by [`omnidiff::stats::sampling::LOC_BUCKETS`] (of the larger side's line
    /// count) as well as language; `--count` becomes a target per (language, bucket).
    #[arg(long, default_value_t = false)]
    stratified: bool,

    /// Sample pairs of one content family instead of code (see `sample_content`): `pictures`,
    /// ... (`diff::content::Family`). Rows are tagged with the family as their dataset, the
    /// format in the `language` column and `<size>-<shape>` as the bucket. `--total` replaces
    /// `--count`, and `--max-commits-per-repo` should be the census window, 50.
    /// Several families at once (`--content fonts,archives`) share one walk of the corpus.
    #[arg(long, value_parser = parse_family, value_delimiter = ',')]
    content: Vec<Family>,

    /// Sample text in UTF-16 or UTF-32 (with a byte order mark, see `code::Encoding`) instead of
    /// UTF-8: rows tagged dataset "encodings", `--count` per language, grammar or not (a file
    /// without one is a painting-only fixture).
    #[arg(long, default_value_t = false)]
    encodings: bool,

    /// `--content pictures`, as the first picture draw was run.
    #[arg(long, default_value_t = false)]
    pictures: bool,

    /// Under `--content`: how many pairs the family's dataset should hold, spread evenly over the
    /// strata that occur. Existing rows of the family count towards it.
    #[arg(long, default_value_t = 100)]
    total: usize,

    /// Sample commits instead of files (see `sample_changesets`): `--count` is per (stratum,
    /// file-count bucket), and `--output` defaults to `src/test/data/changesets.csv`.
    #[arg(long, default_value_t = false)]
    changesets: bool,

    /// Under `--changesets`: sample this many consecutive single-parent commits squashed into one
    /// changeset (the first one's parent against the last one), as the `range` stratum.
    #[arg(long)]
    range_length: Option<usize>,

    /// Under `--changesets`: print the census and write nothing.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Under `--changesets`: print every changeset that is not a control, with what each signal
    /// saw, for checking a signal by hand.
    #[arg(long, default_value_t = false)]
    explain: bool,
}

/// A pointer to a (before, after) code pair in a repository checkout.
///
/// Reconstruction contract: before = blob at `path` in `commit`'s (single) parent tree,
/// after = blob at `path` in `commit`'s tree. Renames are deliberately excluded (see
/// `sample_repository`), so `path` names both sides of every row this binary samples; only a
/// `crosslang` row, added by hand, has a `before_path`.
#[derive(Clone, Eq, PartialEq)]
struct Row {
    language: String,
    repository: String,
    commit: String,
    path: String,
    /// The `src/test/data/diffs/` case this row was promoted to, set by `human_solver`.
    promoted_to: String,
    /// Which research dataset this row was sampled from (see `Args::dataset`).
    dataset: String,
    /// One of `SAMPLED`/`PROMOTED`/`REJECTED`; only `human_solver` moves a row off `SAMPLED`.
    status: String,
    /// Free-form note, set via `human_solver` (a rejection's reason lands here).
    comment: String,
    /// `stats::sampling::loc_bucket` of `max(before_loc, after_loc)`, only for a row sampled
    /// under `--stratified`; an unbucketed row never counts towards a stratified target.
    size_bucket: Option<String>,
    /// The before side's path when it is not `path` (a file moved to another language, the
    /// `crosslang` dataset); empty otherwise. Carried so a rewrite keeps it.
    before_path: String,
}

type SampleKey = (String, String, String);
/// What a target count is tracked per (see `capacity_key`).
type CapacityKey = (String, Option<String>);

/// The dataset of a row without one: every such row was sampled from the small checkout.
const LEGACY_DATASET: &str = "small";

/// The `status` of a row without one, from its `promoted_to`; never `REJECTED`, which postdates
/// the column.
fn default_status(promoted_to: &str) -> &'static str {
    if promoted_to.is_empty() {
        "SAMPLED"
    } else {
        "PROMOTED"
    }
}

fn default_output_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("sample.csv")
}

/// `--dataset`'s default: `repos_dir`'s parent directory name, per the
/// `.../research/<dataset>/repositories` convention.
fn infer_dataset(repos_dir: &Path) -> Option<String> {
    repos_dir
        .parent()?
        .file_name()?
        .to_str()
        .map(|s| s.to_string())
}

fn read_existing_rows(path: &Path) -> Result<Vec<Row>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let mut reader = csv::Reader::from_path(path)?;
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record?;
        let promoted_to = record.get(4).unwrap_or("").to_string();
        let status = match record.get(6) {
            Some(status) if !status.is_empty() => status.to_string(),
            _ => default_status(&promoted_to).to_string(),
        };
        rows.push(Row {
            language: record[0].to_string(),
            repository: record[1].to_string(),
            commit: record[2].to_string(),
            path: record[3].to_string(),
            promoted_to,
            dataset: record.get(5).unwrap_or(LEGACY_DATASET).to_string(),
            status,
            comment: record.get(7).unwrap_or("").to_string(),
            size_bucket: record.get(8).filter(|s| !s.is_empty()).map(str::to_string),
            before_path: record.get(9).unwrap_or("").to_string(),
        });
    }
    Ok(rows)
}

/// Resolves `--dataset` against `--stratified` (see `Args::dataset`). A conflict is an error, not
/// an override: either way round, stratified rows would land in a non-stratified corpus silently.
fn resolve_dataset(args: &Args) -> Result<String> {
    if args.encodings {
        return match (args.dataset.as_deref(), args.stratified) {
            (_, true) => bail!("--encodings and --stratified are separate samples"),
            (Some(dataset), _) if dataset != ENCODINGS_DATASET => bail!(
                "--encodings samples are tagged \"{ENCODINGS_DATASET}\" - omit --dataset, not \
                 --dataset {dataset}"
            ),
            _ => Ok(ENCODINGS_DATASET.to_string()),
        };
    }
    match (args.dataset.as_deref(), args.stratified) {
        (Some(dataset), true) if dataset != "stratified" => bail!(
            "--stratified samples are provenance-tagged \"stratified\" (the sampling method, not \
             a checkout) - pass --dataset stratified or omit --dataset, not --dataset {dataset}"
        ),
        (Some(dataset), _) => Ok(dataset.to_string()),
        (None, true) => Ok("stratified".to_string()),
        (None, false) => infer_dataset(&args.repos_dir).ok_or_else(|| {
            anyhow::anyhow!(
                "could not infer a dataset name from --repos-dir {:?} (expected \
                 .../<dataset>/repositories); pass --dataset explicitly",
                args.repos_dir
            )
        }),
    }
}

/// What a row counts towards for top-up: `(language, None)` normally, `(language, size_bucket)`
/// under `--stratified`.
fn capacity_key(language: &str, bucket: Option<&str>, stratified: bool) -> CapacityKey {
    (
        language.to_string(),
        if stratified {
            bucket.map(str::to_string)
        } else {
            None
        },
    )
}

fn main() -> Result<()> {
    let args = Args::parse();
    let output = args.output.clone().unwrap_or_else(default_output_path);
    let mut families = args.content.clone();
    if args.pictures && !families.contains(&Family::Pictures) {
        families.push(Family::Pictures);
    }
    if args.changesets {
        let output = args.output.clone().unwrap_or_else(default_changesets_path);
        return sample_changesets(&args, &output);
    }
    if args.range_length.is_some() || args.dry_run || args.explain {
        bail!("--range-length, --dry-run and --explain go with --changesets");
    }
    if !families.is_empty() {
        return sample_content(&args, &output, &families);
    }
    let dataset = resolve_dataset(&args)?;

    let existing_rows = read_existing_rows(&output)?;
    let mut existing_counts: HashMap<CapacityKey, usize> = HashMap::new();
    let mut existing_keys: HashSet<SampleKey> = HashSet::new();
    for row in &existing_rows {
        *existing_counts
            .entry(capacity_key(
                &row.language,
                row.size_bucket.as_deref(),
                args.stratified,
            ))
            .or_default() += 1;
        existing_keys.insert((row.repository.clone(), row.commit.clone(), row.path.clone()));
    }

    let repo_paths = find_git_repositories(&args.repos_dir)?;
    if repo_paths.is_empty() {
        eprintln!("No git repositories found in {:?}", args.repos_dir);
        return Ok(());
    }
    println!("Found {} repositories", repo_paths.len());

    let mut rng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_rng(&mut rand::rng()),
    };

    let mut reservoirs: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
    let mut capacities: HashMap<CapacityKey, usize> = HashMap::new();

    for_each_repository(&repo_paths, |repo_path, repository_name| {
        sample_repository(
            repo_path,
            repository_name,
            args.language.as_deref(),
            args.max_commits_per_repo,
            args.count,
            &dataset,
            args.stratified,
            args.encodings,
            &existing_counts,
            &existing_keys,
            &mut reservoirs,
            &mut capacities,
            &mut rng,
        )
    });

    let added: usize = reservoirs.values().map(|r| r.items.len()).sum();
    write_csv(&output, existing_rows, reservoirs)?;
    println!("Added {} new samples to {:?}", added, output);

    Ok(())
}

/// The dataset `--encodings` rows are tagged with, and promoted into.
const ENCODINGS_DATASET: &str = "encodings";

/// The line count of a blob in the size limits that is text in UTF-16 or UTF-32, by its byte
/// order mark; `None` for UTF-8 and for anything that does not decode.
fn encoded_loc(repo: &git2::Repository, oid: git2::Oid) -> Option<usize> {
    let blob = repo.find_blob(oid).ok()?;
    let bytes = blob.content();
    if bytes.len() < MIN_BYTES || bytes.len() > MAX_BYTES || Encoding::of(bytes) == Encoding::Utf8 {
        return None;
    }
    Some(decode_text(bytes)?.lines().count())
}

/// Walks every non-merge commit in the repository and offers each purely-modified file's
/// (commit, path) to the reservoir for its `capacity_key` (language, or (language, size bucket)
/// under `stratified`), topping up towards `target_count` per key.
#[allow(clippy::too_many_arguments)]
fn sample_repository(
    repo_path: &Path,
    repository_name: &str,
    language_filter: Option<&str>,
    max_commits: usize,
    target_count: usize,
    dataset: &str,
    stratified: bool,
    encodings: bool,
    existing_counts: &HashMap<CapacityKey, usize>,
    existing_keys: &HashSet<SampleKey>,
    reservoirs: &mut HashMap<CapacityKey, Reservoir<Row>>,
    capacities: &mut HashMap<CapacityKey, usize>,
    rng: &mut StdRng,
) -> Result<()> {
    // `--encodings` rows go through a reservoir of this repository's own first, at most
    // `PAIRS_PER_REPOSITORY_PER_STRATUM` per language: UTF-16 text is rare, and a handful of
    // repositories (Apple `.strings`) would otherwise be the whole sample.
    let mut local: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
    walk_single_parent_commit_diffs(repo_path, max_commits, false, |repo, id, delta| {
        // The schema locates both blobs by one `path`, so only in-place edits qualify (rename
        // detection is off).
        if delta.status() != Delta::Modified {
            return Ok(());
        }
        // A mode-only change.
        if delta.old_file().id() == delta.new_file().id() {
            return Ok(());
        }

        let Some(path) = delta.new_file().path() else {
            return Ok(());
        };
        if anomalous_paths::is_anomalous(path) {
            return Ok(());
        }

        let language = language_for_path(path);
        // Text in another encoding is sampled grammar or not: without one, it is painted.
        if encodings {
            let (Some(before_loc), Some(after_loc)) = (
                encoded_loc(repo, delta.old_file().id()),
                encoded_loc(repo, delta.new_file().id()),
            ) else {
                return Ok(());
            };
            let language = language.map_or("Text".to_string(), |language| language.to_string());
            let path = path.to_string_lossy().into_owned();
            if existing_keys.contains(&(repository_name.to_string(), id.to_string(), path.clone()))
            {
                return Ok(());
            }
            let cap_key = capacity_key(&language, None, false);
            capacities.entry(cap_key.clone()).or_insert_with(|| {
                target_count.saturating_sub(existing_counts.get(&cap_key).copied().unwrap_or(0))
            });
            let row = Row {
                language,
                repository: repository_name.to_string(),
                commit: id.to_string(),
                path,
                promoted_to: String::new(),
                dataset: dataset.to_string(),
                status: "SAMPLED".to_string(),
                comment: String::new(),
                size_bucket: Some(loc_bucket(before_loc.max(after_loc)).to_string()),
                before_path: String::new(),
            };
            local
                .entry(cap_key)
                .or_default()
                .offer(row, PAIRS_PER_REPOSITORY_PER_STRATUM, rng);
            return Ok(());
        }
        let Some(mut language) = language else {
            return Ok(());
        };
        // Only `.ts` needs content to disambiguate (Qt Linguist vs. TypeScript); gating on it
        // avoids a blob read for every other file.
        if language == Language::TypeScript
            && let Ok(blob) = repo.find_blob(delta.new_file().id())
            && let Ok(text) = std::str::from_utf8(blob.content())
            && let Some(refined) = language_for_path_and_content(path, text)
        {
            language = refined;
        }
        if to_treesitter(&language).is_none() {
            return Ok(());
        }
        let language = language.to_string();
        if let Some(filter) = language_filter
            && language != filter
        {
            return Ok(());
        }

        let path = path.to_string_lossy().into_owned();
        let key = (repository_name.to_string(), id.to_string(), path.clone());
        if existing_keys.contains(&key) {
            return Ok(());
        }

        // The larger side decides the bucket, as in `sample_code_pairs`.
        let Some(before_loc) =
            text_loc_if_in_range(repo, delta.old_file().id(), MIN_BYTES, MAX_BYTES)
        else {
            return Ok(());
        };
        let Some(after_loc) =
            text_loc_if_in_range(repo, delta.new_file().id(), MIN_BYTES, MAX_BYTES)
        else {
            return Ok(());
        };
        let bucket = stratified.then(|| loc_bucket(before_loc.max(after_loc)));

        let cap_key = capacity_key(&language, bucket, stratified);
        let capacity = *capacities.entry(cap_key.clone()).or_insert_with(|| {
            target_count.saturating_sub(existing_counts.get(&cap_key).copied().unwrap_or(0))
        });

        let row = Row {
            language: language.clone(),
            repository: repository_name.to_string(),
            commit: id.to_string(),
            path,
            promoted_to: String::new(),
            dataset: dataset.to_string(),
            status: "SAMPLED".to_string(),
            comment: String::new(),
            size_bucket: bucket.map(str::to_string),
            before_path: String::new(),
        };
        reservoirs
            .entry(cap_key)
            .or_default()
            .offer(row, capacity, rng);

        Ok(())
    })?;
    for (key, reservoir) in local {
        let capacity = capacities.get(&key).copied().unwrap_or(0);
        let global = reservoirs.entry(key).or_default();
        for row in reservoir.items {
            global.offer(row, capacity, rng);
        }
    }
    Ok(())
}

fn parse_family(name: &str) -> Result<Family, String> {
    Family::from_name(name).ok_or_else(|| {
        let names: Vec<&str> = Family::ALL.iter().map(|family| family.name()).collect();
        format!("expected one of {}", names.join(", "))
    })
}

/// How many pairs one repository may contribute to one stratum: a handful of repositories hold
/// most changes of a binary format (the change census), and without a cap they would be most of
/// the sample.
const PAIRS_PER_REPOSITORY_PER_STRATUM: usize = 2;

/// How many pairs one repository may contribute to one family, over all its strata: a repository
/// with many formats and sizes (a font family shipped as TTF, OTF, WOFF and WOFF2 in every
/// weight) otherwise fills a tenth of the sample on its own.
const PAIRS_PER_REPOSITORY_PER_FAMILY: usize = 4;

/// The largest side a family's sample holds. Pictures keep the code sample's cap; a font, an
/// archive or a PDF is often larger than any source file.
fn max_bytes(family: Family) -> usize {
    match family {
        Family::Pictures => MAX_BYTES,
        _ => 16 * MAX_BYTES,
    }
}

/// The size half of a stratum, by the larger side's [`Probe::size`]: pixels for a picture (a
/// still cursor too), members for a container.
fn size_bucket(format: Format, size: u64) -> &'static str {
    match format.engine() {
        Engine::Picture => match size {
            0..=4_096 => "icon",
            4_097..=65_536 => "small",
            65_537..=1_048_576 => "medium",
            _ => "large",
        },
        Engine::Container(_) => match size {
            0..=1 => "single",
            2..=10 => "few",
            11..=100 => "some",
            101..=1_000 => "many",
            _ => "lots",
        },
    }
}

/// The shape half of a stratum: whether the two sides have the same shape ([`Probe::shape`]),
/// in the engine's words.
fn shape_label(format: Format, same: bool) -> &'static str {
    match (format.engine(), same) {
        (Engine::Picture, true) => "same",
        (Engine::Picture, false) => "resized",
        (Engine::Container(_), true) => "same-members",
        (Engine::Container(_), false) => "members-changed",
    }
}

/// True if a changed file at `path` may be of `family`: its extension is one the family's files
/// carry, or, for cursors, it has none (X cursors are named after the cursor). Whether it is, is
/// the content's say ([`content_side`]).
fn may_be(family: Family, path: &Path) -> bool {
    match path.extension() {
        Some(ext) => family
            .extensions()
            .contains(&ext.to_string_lossy().to_ascii_lowercase().as_str()),
        None => family == Family::Cursors,
    }
}

/// A side the sample can hold: content of one of `families` by its bytes (which also drops Git
/// LFS pointers, text files named `.png`), within its family's size limit, probed from its header.
fn content_side(repo: &git2::Repository, oid: git2::Oid, families: &[Family]) -> Option<Probe> {
    let blob = repo.find_blob(oid).ok()?;
    let bytes = blob.content();
    let family = content::sniff(bytes)?.family();
    if !families.contains(&family) || bytes.len() < MIN_BYTES || bytes.len() > max_bytes(family) {
        return None;
    }
    content::probe(bytes)
}

/// `--content`: tops `output`'s rows of each of `families` up to `--total` pairs, in one walk.
/// Every in-place modification of a family's files in the last `--max-commits-per-repo` commits
/// is a candidate; each repository offers at most [`PAIRS_PER_REPOSITORY_PER_STRATUM`] per stratum
/// (format x size bucket x shape) and [`PAIRS_PER_REPOSITORY_PER_FAMILY`] in all, a pair already
/// seen elsewhere (the same two blobs) is offered once, and each family's shortfall is split
/// evenly over its strata that have candidates, a stratum with fewer giving its share to the rest.
fn sample_content(args: &Args, output: &Path, families: &[Family]) -> Result<()> {
    let existing_rows = read_existing_rows(output)?;
    let shortfalls: HashMap<Family, usize> = families
        .iter()
        .map(|family| {
            let existing = existing_rows
                .iter()
                .filter(|row| row.dataset == family.name())
                .count();
            (*family, args.total.saturating_sub(existing))
        })
        .collect();
    let shortfall = shortfalls.values().copied().max().unwrap_or(0);
    let names: Vec<&str> = families.iter().map(|family| family.name()).collect();
    let existing_keys: HashSet<SampleKey> = existing_rows
        .iter()
        .map(|row| (row.repository.clone(), row.commit.clone(), row.path.clone()))
        .collect();

    let repo_paths = find_git_repositories(&args.repos_dir)?;
    println!(
        "Found {} repositories; up to {shortfall} pairs to sample of each of {}",
        repo_paths.len(),
        names.join(", ")
    );
    let mut rng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_rng(&mut rand::rng()),
    };

    let mut seen_pairs: HashSet<(git2::Oid, git2::Oid)> = HashSet::new();
    let mut strata: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
    let mut stratum_family: HashMap<CapacityKey, Family> = HashMap::new();
    for_each_repository(&repo_paths, |repo_path, repository_name| {
        let mut local: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
        walk_single_parent_commit_diffs(
            repo_path,
            args.max_commits_per_repo,
            false,
            |repo, id, delta| {
                if delta.status() != Delta::Modified
                    || delta.old_file().id() == delta.new_file().id()
                {
                    return Ok(());
                }
                let Some(path) = delta.new_file().path() else {
                    return Ok(());
                };
                if anomalous_paths::is_anomalous(path)
                    || !families.iter().any(|family| may_be(*family, path))
                {
                    return Ok(());
                }
                let path = path.to_string_lossy().into_owned();
                if existing_keys.contains(&(
                    repository_name.to_string(),
                    id.to_string(),
                    path.clone(),
                )) {
                    return Ok(());
                }
                let (Some(before), Some(after)) = (
                    content_side(repo, delta.old_file().id(), families),
                    content_side(repo, delta.new_file().id(), families),
                ) else {
                    return Ok(());
                };
                let family = after.format.family();
                if before.format.family() != family {
                    return Ok(());
                }
                if !seen_pairs.insert((delta.old_file().id(), delta.new_file().id())) {
                    return Ok(());
                }
                let bucket = format!(
                    "{}-{}",
                    size_bucket(after.format, before.size.max(after.size)),
                    shape_label(after.format, before.shape == after.shape)
                );
                let format = after.format.name();
                let row = Row {
                    language: format.clone(),
                    repository: repository_name.to_string(),
                    commit: id.to_string(),
                    path,
                    promoted_to: String::new(),
                    dataset: family.name().to_string(),
                    status: "SAMPLED".to_string(),
                    comment: String::new(),
                    size_bucket: Some(bucket.clone()),
                    before_path: String::new(),
                };
                let key = (format, Some(bucket));
                stratum_family.insert(key.clone(), family);
                local.entry(key).or_default().offer(
                    row,
                    PAIRS_PER_REPOSITORY_PER_STRATUM,
                    &mut rng,
                );
                Ok(())
            },
        )?;
        // At most `PAIRS_PER_REPOSITORY_PER_FAMILY` of this repository's pairs per family, chosen
        // at random among the ones its strata kept.
        let mut by_family: HashMap<String, Vec<(CapacityKey, Row)>> = HashMap::new();
        for (key, reservoir) in local {
            for row in reservoir.items {
                by_family
                    .entry(row.dataset.clone())
                    .or_default()
                    .push((key.clone(), row));
            }
        }
        for (_, mut rows) in by_family {
            rows.shuffle(&mut rng);
            rows.truncate(PAIRS_PER_REPOSITORY_PER_FAMILY);
            for (key, row) in rows {
                strata
                    .entry(key)
                    .or_default()
                    .offer(row, shortfall, &mut rng);
            }
        }
        Ok(())
    });

    // Each family's shortfall over its own strata.
    let mut quotas = HashMap::new();
    for family in families {
        let available: Vec<(CapacityKey, usize)> = strata
            .iter()
            .filter(|(key, _)| stratum_family.get(*key) == Some(family))
            .map(|(key, reservoir)| (key.clone(), reservoir.items.len()))
            .collect();
        quotas.extend(even_quotas(&available, shortfalls[family]));
    }
    let mut picked = HashMap::new();
    for (key, mut reservoir) in strata {
        let quota = quotas.get(&key).copied().unwrap_or(0);
        reservoir.items.shuffle(&mut rng);
        reservoir.items.truncate(quota);
        picked.insert(key, reservoir);
    }
    for ((format, bucket), quota) in {
        let mut sorted: Vec<_> = quotas.iter().collect();
        sorted.sort();
        sorted
    } {
        println!("  {format} {}: {quota}", bucket.as_deref().unwrap_or(""));
    }
    let added: usize = picked.values().map(|r| r.items.len()).sum();
    write_csv(output, existing_rows, picked)?;
    println!("Added {added} pairs of {} to {output:?}", names.join(", "));
    Ok(())
}

/// Splits `total` over strata as evenly as their `available` counts allow: each gets an equal
/// share, a stratum with fewer takes what it has, and the remainder is shared out again.
fn even_quotas(available: &[(CapacityKey, usize)], total: usize) -> HashMap<CapacityKey, usize> {
    let mut quotas: HashMap<CapacityKey, usize> = HashMap::new();
    let mut open: Vec<&(CapacityKey, usize)> = available.iter().filter(|(_, n)| *n > 0).collect();
    open.sort_by_key(|(_, n)| *n);
    let mut left = total;
    while !open.is_empty() && left > 0 {
        let share = left.div_ceil(open.len());
        let (key, have) = open.remove(0);
        let take = (*have).min(share).min(left);
        quotas.insert(key.clone(), take);
        left -= take;
    }
    quotas
}

fn default_changesets_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join("changesets.csv")
}

/// A stratum and a file-count bucket: what `--changesets` samples per.
type Cell = (String, String);

/// How many commits of each repository fall in each cell, and why the rest are not changesets.
#[derive(Default)]
struct Census {
    cells: BTreeMap<String, BTreeMap<Cell, usize>>,
    /// Commits a signal fired in, whether or not it decided the stratum.
    signals: BTreeMap<String, BTreeMap<String, usize>>,
    skipped: BTreeMap<String, BTreeMap<Skip, usize>>,
    seconds: BTreeMap<String, f64>,
}

impl Census {
    /// One table: a row a stratum that occurs, a column a bucket, then the row's total and the
    /// commits its signal fired in at all.
    fn table(cells: &BTreeMap<Cell, usize>, signals: &BTreeMap<String, usize>) -> String {
        let mut strata: Vec<String> = Vec::new();
        for (stratum, _) in cells.keys() {
            if !strata.contains(stratum) {
                strata.push(stratum.clone());
            }
        }
        // Most specific first, as the precedence; a squash's strata after them.
        let rank = |name: &String| {
            let inner = name.strip_prefix("range:").unwrap_or(name);
            (
                name.starts_with("range:"),
                PRECEDENCE
                    .iter()
                    .position(|stratum| stratum.name() == inner),
            )
        };
        strata.sort_by_key(rank);
        let mut out = format!(
            "  {:<24}{:>7}{:>7}{:>7}{:>7}{:>8}\n",
            "stratum", FILE_BUCKETS[0], FILE_BUCKETS[1], FILE_BUCKETS[2], "all", "signal"
        );
        let mut totals = [0usize; 4];
        for stratum in &strata {
            let counts = FILE_BUCKETS.map(|bucket| {
                cells
                    .get(&(stratum.clone(), bucket.to_string()))
                    .copied()
                    .unwrap_or(0)
            });
            let all: usize = counts.iter().sum();
            for (total, count) in totals.iter_mut().zip(counts.iter().chain([&all])) {
                *total += count;
            }
            let inner = stratum.strip_prefix("range:").unwrap_or(stratum);
            out.push_str(&format!(
                "  {:<24}{:>7}{:>7}{:>7}{:>7}{:>8}\n",
                stratum,
                counts[0],
                counts[1],
                counts[2],
                all,
                signals.get(inner).copied().unwrap_or(0)
            ));
        }
        out.push_str(&format!(
            "  {:<24}{:>7}{:>7}{:>7}{:>7}\n",
            "all", totals[0], totals[1], totals[2], totals[3]
        ));
        out
    }

    fn print(&self) {
        let mut all_cells: BTreeMap<Cell, usize> = BTreeMap::new();
        let mut all_signals: BTreeMap<String, usize> = BTreeMap::new();
        let mut all_skipped: BTreeMap<Skip, usize> = BTreeMap::new();
        let empty = BTreeMap::new();
        for (repository, seconds) in &self.seconds {
            let cells = self.cells.get(repository).unwrap_or(&empty);
            let signals = self.signals.get(repository).cloned().unwrap_or_default();
            println!("\n{repository} ({seconds:.1}s)");
            print!("{}", Census::table(cells, &signals));
            for (cell, count) in cells {
                *all_cells.entry(cell.clone()).or_default() += count;
            }
            for (signal, count) in signals {
                *all_signals.entry(signal).or_default() += count;
            }
            for (skip, count) in self.skipped.get(repository).into_iter().flatten() {
                println!("  not a changeset, {}: {count}", skip.name());
                *all_skipped.entry(*skip).or_default() += count;
            }
        }
        println!("\nAll {} repositories", self.seconds.len());
        print!("{}", Census::table(&all_cells, &all_signals));
        for (skip, count) in all_skipped {
            println!("  not a changeset, {}: {count}", skip.name());
        }
    }
}

/// What one `--changesets` run is asked for.
struct ChangesetOptions<'a> {
    max_commits: usize,
    /// Commits squashed into a changeset; `None` for one commit each.
    range_length: Option<usize>,
    count: usize,
    dataset: &'a str,
    explain: bool,
}

/// The state a `--changesets` run carries from repository to repository.
#[derive(Default)]
struct ChangesetSample {
    existing_counts: HashMap<Cell, usize>,
    /// (repository, commit, parent) of the rows already there.
    existing_keys: HashSet<SampleKey>,
    reservoirs: HashMap<Cell, Reservoir<changeset::Row>>,
    capacities: HashMap<Cell, usize>,
    census: Census,
}

/// `--changesets`: tops `output` up to `--count` changesets per (stratum, file-count bucket).
/// Every commit of the last `--max-commits-per-repo` that is a changeset (see
/// [`omnidiff::stats::changeset`]) is a candidate; a repository offers at most
/// [`PAIRS_PER_REPOSITORY_PER_STRATUM`] per cell, chosen at random, so that one repository's habit
/// (a project that moves a directory a week) is not a stratum. Rows already in `output` count
/// towards their cell whatever their status, and their commits are not offered again.
fn sample_changesets(args: &Args, output: &Path) -> Result<()> {
    if args.range_length.is_some_and(|length| length < 2) {
        bail!("--range-length squashes at least 2 commits");
    }
    let dataset = match &args.dataset {
        Some(dataset) => dataset.clone(),
        None => infer_dataset(&args.repos_dir).ok_or_else(|| {
            anyhow::anyhow!("could not infer a dataset name from --repos-dir; pass --dataset")
        })?,
    };
    let existing_rows = changeset::read_rows(output)?;
    let mut sample = ChangesetSample::default();
    for row in &existing_rows {
        *sample
            .existing_counts
            .entry((row.stratum.clone(), row.bucket.clone()))
            .or_default() += 1;
        sample.existing_keys.insert((
            row.repository.clone(),
            row.commit.clone(),
            row.parent.clone(),
        ));
    }
    let repo_paths = find_git_repositories(&args.repos_dir)?;
    println!("Found {} repositories", repo_paths.len());
    let mut rng = match args.seed {
        Some(seed) => StdRng::seed_from_u64(seed),
        None => StdRng::from_rng(&mut rand::rng()),
    };
    let options = ChangesetOptions {
        max_commits: args.max_commits_per_repo,
        range_length: args.range_length,
        count: args.count,
        dataset: &dataset,
        explain: args.explain,
    };
    for_each_repository(&repo_paths, |repo_path, repository_name| {
        let started = std::time::Instant::now();
        let result = sample_changeset_repository(
            repo_path,
            repository_name,
            &options,
            &mut sample,
            &mut rng,
        );
        sample
            .census
            .seconds
            .insert(repository_name.to_string(), started.elapsed().as_secs_f64());
        result
    });
    sample.census.print();
    if args.dry_run {
        println!("\n--dry-run: nothing written");
        return Ok(());
    }
    let mut rows = existing_rows;
    let before = rows.len();
    for (_, reservoir) in sample.reservoirs {
        rows.extend(reservoir.items);
    }
    let added = rows.len() - before;
    changeset::write_rows(output, rows)?;
    println!("\nAdded {added} changesets to {output:?}");
    Ok(())
}

/// The commit a changeset of `commit` starts from: its parent, or under `range_length` the parent
/// of the last of that many commits down the single-parent chain. `None` when the chain is shorter,
/// meets a merge, or runs into a commit an earlier range took (`taken`), so ranges never overlap.
fn changeset_base<'repo>(
    commit: &git2::Commit<'repo>,
    range_length: Option<usize>,
    taken: &mut HashSet<git2::Oid>,
) -> Result<Option<git2::Commit<'repo>>> {
    let Some(length) = range_length else {
        return Ok(Some(commit.parent(0)?));
    };
    let mut chain = vec![commit.clone()];
    while chain.len() < length {
        let Ok(parent) = chain[chain.len() - 1].parent(0) else {
            return Ok(None);
        };
        if parent.parents().len() != 1 {
            return Ok(None);
        }
        chain.push(parent);
    }
    if chain.iter().any(|commit| taken.contains(&commit.id())) {
        return Ok(None);
    }
    // A shallow clone's last commit names a parent it does not have.
    let Ok(base) = chain[chain.len() - 1].parent(0) else {
        return Ok(None);
    };
    taken.extend(chain.iter().map(|commit| commit.id()));
    Ok(Some(base))
}

/// Classifies every changeset among the repository's last `max_commits` commits, counts it in the
/// census, and offers the ones not yet sampled to their cell's reservoir (see `sample_changesets`).
fn sample_changeset_repository(
    repo_path: &Path,
    repository_name: &str,
    options: &ChangesetOptions,
    sample: &mut ChangesetSample,
    rng: &mut StdRng,
) -> Result<()> {
    let mut local: HashMap<Cell, Reservoir<changeset::Row>> = HashMap::new();
    let mut index = BlockIndex::default();
    let mut taken: HashSet<git2::Oid> = HashSet::new();
    walk_single_parent_commits(repo_path, options.max_commits, |repo, commit| {
        let Some(base) = changeset_base(commit, options.range_length, &mut taken)? else {
            return Ok(());
        };
        let (before, after) = (base.tree()?, commit.tree()?);
        let mut skip = |skip: Skip| {
            let skipped = sample.census.skipped.entry(repository_name.to_string());
            *skipped.or_default().entry(skip).or_default() += 1;
        };
        let files = match changeset::load_changes(repo, &before, &after)? {
            Ok(files) => files,
            Err(reason) => {
                skip(reason);
                return Ok(());
            }
        };
        let changed_lines = changeset::changed_lines(&files);
        if changed_lines > changeset::MAX_CHANGED_LINES {
            skip(Skip::TooManyLines);
            return Ok(());
        }
        // Git pairs a removed path with an added one; without both it has nothing to say that
        // the tree diff did not.
        let hints = if files.iter().any(|file| file.after.is_none())
            && files.iter().any(|file| file.before.is_none())
        {
            changeset::vcs_hints(repo, base.id(), commit.id())?
        } else {
            Vec::new()
        };
        index.add_tree(repo, &before);
        let classification =
            changeset::classify(&files, &hints, &index.view(repo, &before, &files));

        let bucket = changeset::file_bucket(files.len());
        let (stratum, census_stratum) = match options.range_length {
            Some(_) => (
                Stratum::Range.name().to_string(),
                format!("range:{}", classification.stratum.name()),
            ),
            None => {
                let name = classification.stratum.name().to_string();
                (name.clone(), name)
            }
        };
        let cells = sample.census.cells.entry(repository_name.to_string());
        *cells
            .or_default()
            .entry((census_stratum, bucket.to_string()))
            .or_default() += 1;
        let signals = sample.census.signals.entry(repository_name.to_string());
        let signals = signals.or_default();
        for signal in &classification.signals {
            *signals.entry(signal.name().to_string()).or_default() += 1;
        }
        if options.explain && classification.stratum != Stratum::Control {
            println!(
                "\n  {} {} {} files, {changed_lines} lines: {}",
                &commit.id().to_string()[..12],
                classification.stratum.name(),
                files.len(),
                commit.summary().ok().flatten().unwrap_or("")
            );
            for line in &classification.evidence {
                println!("    {line}");
            }
        }

        let (commit_id, parent_id) = (commit.id().to_string(), base.id().to_string());
        let key = (
            repository_name.to_string(),
            commit_id.clone(),
            parent_id.clone(),
        );
        if sample.existing_keys.contains(&key) {
            return Ok(());
        }
        let cell = (stratum.clone(), bucket.to_string());
        sample.capacities.entry(cell.clone()).or_insert_with(|| {
            let existing = sample.existing_counts.get(&cell).copied().unwrap_or(0);
            options.count.saturating_sub(existing)
        });
        let row = changeset::Row {
            name: changeset::fixture_name(&stratum, repository_name, &commit_id),
            repository: repository_name.to_string(),
            commit: commit_id,
            parent: parent_id,
            stratum,
            signals: changeset::signals_column(&classification, options.range_length),
            files: files.len(),
            changed_lines,
            main_language: changeset::main_language(&files),
            bucket: bucket.to_string(),
            dataset: options.dataset.to_string(),
            status: "SAMPLED".to_string(),
            comment: String::new(),
        };
        local
            .entry(cell)
            .or_default()
            .offer(row, PAIRS_PER_REPOSITORY_PER_STRATUM, rng);
        Ok(())
    })?;
    // In a fixed order: a `HashMap`'s would make a seeded draw differ from run to run.
    let mut local: Vec<(Cell, Reservoir<changeset::Row>)> = local.into_iter().collect();
    local.sort_by(|a, b| a.0.cmp(&b.0));
    for (cell, reservoir) in local {
        let capacity = sample.capacities.get(&cell).copied().unwrap_or(0);
        let global = sample.reservoirs.entry(cell).or_default();
        for row in reservoir.items {
            global.offer(row, capacity, rng);
        }
    }
    Ok(())
}

fn write_csv(
    path: &Path,
    existing_rows: Vec<Row>,
    reservoirs: HashMap<CapacityKey, Reservoir<Row>>,
) -> Result<()> {
    let mut rows = existing_rows;
    for (_, reservoir) in reservoirs {
        rows.extend(reservoir.items);
    }
    rows.sort_by(|a, b| {
        (&a.language, &a.repository, &a.commit, &a.path).cmp(&(
            &b.language,
            &b.repository,
            &b.commit,
            &b.path,
        ))
    });

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut writer = csv::Writer::from_path(path)?;
    writer.write_record([
        "language",
        "repository",
        "commit",
        "path",
        "promoted_to",
        "dataset",
        "status",
        "comment",
        "size_bucket",
        "before_path",
    ])?;
    for row in &rows {
        writer.write_record([
            &row.language,
            &row.repository,
            &row.commit,
            &row.path,
            &row.promoted_to,
            &row.dataset,
            &row.status,
            &row.comment,
            row.size_bucket.as_deref().unwrap_or(""),
            &row.before_path,
        ])?;
    }
    writer.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One walk samples every family asked for, each into its own dataset, and nothing else.
    #[test]
    fn several_families_share_one_walk() -> Result<()> {
        let root = tempfile::tempdir()?;
        let dir = root.path().join("repo");
        let repo = git2::Repository::init(&dir)?;
        let png = |pixel: u8| {
            let mut bytes = Vec::new();
            image::RgbaImage::from_pixel(4, 4, image::Rgba([pixel, 0, 0, 255]))
                .write_to(
                    &mut std::io::Cursor::new(&mut bytes),
                    image::ImageFormat::Png,
                )
                .unwrap();
            bytes
        };
        let zip = |text: &str| {
            let mut bytes = Vec::new();
            let mut writer = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
            writer
                .start_file("a.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, text.as_bytes()).unwrap();
            writer.finish().unwrap();
            bytes
        };
        let mut parent: Option<git2::Oid> = None;
        for (pixel, text) in [(0u8, "one"), (255, "two")] {
            let mut builder = repo.treebuilder(None)?;
            for (name, bytes) in [
                ("logo.png", png(pixel)),
                ("bundle.zip", zip(text)),
                ("notes.txt", text.as_bytes().to_vec()),
            ] {
                builder.insert(name, repo.blob(&bytes)?, 0o100644)?;
            }
            let tree = repo.find_tree(builder.write()?)?;
            let signature = git2::Signature::now("t", "t@example.com")?;
            let parents: Vec<git2::Commit> = parent
                .map(|oid| repo.find_commit(oid))
                .transpose()?
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            parent =
                Some(repo.commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)?);
        }

        let output = root.path().join("sample.csv");
        let mut args = args(root.path().to_str().unwrap(), None, false);
        args.total = 5;
        args.max_commits_per_repo = 50;
        args.seed = Some(1);
        sample_content(&args, &output, &[Family::Archives, Family::Pictures])?;
        let rows = read_existing_rows(&output)?;
        let datasets: HashSet<(&str, &str)> = rows
            .iter()
            .map(|row| (row.dataset.as_str(), row.path.as_str()))
            .collect();
        assert_eq!(
            datasets,
            HashSet::from([("pictures", "logo.png"), ("archives", "bundle.zip")]),
            "the text file is in neither"
        );
        Ok(())
    }

    #[test]
    fn picture_quotas_are_even_and_a_short_stratum_gives_its_share_away() {
        let key = |name: &str| (name.to_string(), Some("small-same".to_string()));
        let quotas = even_quotas(&[(key("PNG"), 100), (key("GIF"), 3), (key("ICO"), 100)], 21);
        assert_eq!(quotas[&key("GIF")], 3);
        assert_eq!(quotas[&key("PNG")] + quotas[&key("ICO")], 18);
        assert_eq!(quotas[&key("PNG")], 9);
        // Never more than there is, even when the total asks for more.
        let quotas = even_quotas(&[(key("PNG"), 2)], 10);
        assert_eq!(quotas[&key("PNG")], 2);
    }

    #[test]
    fn encodings_take_utf16_and_utf32_text_and_nothing_else() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let utf16: Vec<u8> = "\u{feff}a\nb\n"
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect();
        let utf32: Vec<u8> = "\u{feff}a\n"
            .chars()
            .flat_map(|c| (c as u32).to_be_bytes())
            .collect();
        let blob = |bytes: &[u8]| repo.blob(bytes).unwrap();
        assert_eq!(encoded_loc(&repo, blob(&utf16)), Some(2));
        assert_eq!(encoded_loc(&repo, blob(&utf32)), Some(1));
        assert_eq!(encoded_loc(&repo, blob(b"a\nb\n")), None, "UTF-8");
        assert_eq!(
            encoded_loc(&repo, blob(&[0xFF, 0xFE, 0x00, 0xD8])),
            None,
            "broken"
        );
    }

    /// Six pictures in six formats - six strata - from one repository give four rows.
    #[test]
    fn a_repository_gives_at_most_four_pairs_a_family() -> Result<()> {
        let root = tempfile::tempdir()?;
        let dir = root.path().join("repo");
        let repo = git2::Repository::init(&dir)?;
        use image::ImageFormat::*;
        let formats = [
            (Png, "png"),
            (Bmp, "bmp"),
            (Gif, "gif"),
            (Tiff, "tif"),
            (Ico, "ico"),
            (Jpeg, "jpg"),
        ];
        let mut parent: Option<git2::Oid> = None;
        for pixel in [0u8, 255] {
            let mut builder = repo.treebuilder(None)?;
            for (format, ext) in formats {
                let mut bytes = Vec::new();
                image::RgbImage::from_pixel(8, 8, image::Rgb([pixel, 0, 0]))
                    .write_to(&mut std::io::Cursor::new(&mut bytes), format)
                    .unwrap();
                builder.insert(format!("picture.{ext}"), repo.blob(&bytes)?, 0o100644)?;
            }
            let tree = repo.find_tree(builder.write()?)?;
            let signature = git2::Signature::now("t", "t@example.com")?;
            let parents: Vec<git2::Commit> = parent
                .map(|oid| repo.find_commit(oid))
                .transpose()?
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            parent =
                Some(repo.commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)?);
        }
        let output = root.path().join("sample.csv");
        let mut args = args(root.path().to_str().unwrap(), None, false);
        args.total = 100;
        args.max_commits_per_repo = 50;
        args.seed = Some(1);
        sample_content(&args, &output, &[Family::Pictures])?;
        assert_eq!(
            read_existing_rows(&output)?.len(),
            PAIRS_PER_REPOSITORY_PER_FAMILY
        );
        Ok(())
    }

    /// Five UTF-16 edits in one repository give two rows: a repository's own reservoir first.
    #[test]
    fn encodings_take_at_most_two_a_repository() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let repo = git2::Repository::init(dir.path())?;
        let mut parent: Option<git2::Oid> = None;
        for round in 0..6 {
            let text: Vec<u8> = format!("\u{feff}\"key\" = \"{round}\";\n")
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();
            let mut builder = repo.treebuilder(None)?;
            builder.insert("Localizable.strings", repo.blob(&text)?, 0o100644)?;
            let tree = repo.find_tree(builder.write()?)?;
            let signature = git2::Signature::now("t", "t@example.com")?;
            let parents: Vec<git2::Commit> = parent
                .map(|oid| repo.find_commit(oid))
                .transpose()?
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            parent =
                Some(repo.commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)?);
        }
        let mut reservoirs: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
        let mut capacities: HashMap<CapacityKey, usize> = HashMap::new();
        sample_repository(
            dir.path(),
            "r",
            None,
            50,
            100,
            ENCODINGS_DATASET,
            false,
            true,
            &HashMap::new(),
            &HashSet::new(),
            &mut reservoirs,
            &mut capacities,
            &mut StdRng::seed_from_u64(1),
        )?;
        let rows: usize = reservoirs.values().map(|r| r.items.len()).sum();
        assert_eq!(rows, PAIRS_PER_REPOSITORY_PER_STRATUM);
        Ok(())
    }

    #[test]
    fn picture_sizes_bucket_by_pixel_count() {
        assert_eq!(
            size_bucket(Format::Picture(image::ImageFormat::Png), 16 * 16),
            "icon"
        );
        assert_eq!(
            size_bucket(Format::Picture(image::ImageFormat::Png), 200 * 120),
            "small"
        );
        assert_eq!(
            size_bucket(Format::Picture(image::ImageFormat::Png), 800 * 600),
            "medium"
        );
        assert_eq!(
            size_bucket(Format::Picture(image::ImageFormat::Png), 4000 * 3000),
            "large"
        );
    }
    use omnidiff::stats::sampling::LOC_BUCKETS;
    use omnidiff::test::helper;

    fn sample(
        repo_path: &Path,
        target_count: usize,
        existing: &[Row],
        seed: u64,
        stratified: bool,
    ) -> Result<Vec<Row>> {
        let mut existing_counts: HashMap<CapacityKey, usize> = HashMap::new();
        let mut existing_keys: HashSet<SampleKey> = HashSet::new();
        for row in existing {
            *existing_counts
                .entry(capacity_key(
                    &row.language,
                    row.size_bucket.as_deref(),
                    stratified,
                ))
                .or_default() += 1;
            existing_keys.insert((row.repository.clone(), row.commit.clone(), row.path.clone()));
        }

        let mut reservoirs: HashMap<CapacityKey, Reservoir<Row>> = HashMap::new();
        let mut capacities: HashMap<CapacityKey, usize> = HashMap::new();
        let mut rng = StdRng::seed_from_u64(seed);

        sample_repository(
            repo_path,
            "handmade",
            None,
            1000,
            target_count,
            "small",
            stratified,
            false,
            &existing_counts,
            &existing_keys,
            &mut reservoirs,
            &mut capacities,
            &mut rng,
        )?;

        let mut rows = existing.to_vec();
        for (_, reservoir) in reservoirs {
            rows.extend(reservoir.items);
        }
        Ok(rows)
    }

    #[test]
    fn samples_real_pairs_from_handmade_repository() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;
        let rows = sample(&repo_path, 10, &[], 1, false)?;

        let rust: Vec<&Row> = rows.iter().filter(|r| r.language == "Rust").collect();
        assert!(!rust.is_empty());
        assert!(rust.iter().any(|r| r.path.ends_with("main.rs")));
        for row in &rust {
            assert_eq!(row.repository, "handmade");
            assert!(!row.commit.is_empty());
            assert_eq!(
                row.size_bucket, None,
                "not --stratified: no bucket recorded"
            );
        }

        Ok(())
    }

    #[test]
    fn stratified_sampling_records_a_size_bucket_per_row() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;
        let rows = sample(&repo_path, 10, &[], 1, true)?;

        let rust: Vec<&Row> = rows.iter().filter(|r| r.language == "Rust").collect();
        assert!(!rust.is_empty());
        for row in &rust {
            assert!(
                row.size_bucket.is_some(),
                "--stratified row missing its size bucket: {:?}",
                row.path
            );
        }

        Ok(())
    }

    #[test]
    fn stratified_top_up_ignores_unstratified_rows_and_counts_by_bucket() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;

        // An unbucketed row must not count towards any stratified per-bucket target.
        let unstratified_existing = Row {
            language: "Rust".to_string(),
            repository: "handmade".to_string(),
            commit: "0".repeat(40),
            path: "not-a-real-path.rs".to_string(),
            promoted_to: String::new(),
            dataset: "small".to_string(),
            status: "SAMPLED".to_string(),
            comment: String::new(),
            size_bucket: None,
            before_path: String::new(),
        };

        let rows = sample(&repo_path, 10, &[unstratified_existing], 1, true)?;
        let rust_stratified: Vec<&Row> = rows
            .iter()
            .filter(|r| r.language == "Rust" && r.size_bucket.is_some())
            .collect();

        assert!(!rust_stratified.is_empty());
        use std::collections::HashSet as StdHashSet;
        let buckets: StdHashSet<&str> = rust_stratified
            .iter()
            .map(|r| r.size_bucket.as_deref().unwrap())
            .collect();
        assert!(
            buckets
                .iter()
                .all(|b| LOC_BUCKETS.iter().any(|(_, label)| label == b)),
            "unexpected bucket label(s): {:?}",
            buckets
        );

        Ok(())
    }

    #[test]
    fn tops_up_existing_samples_without_duplicates() -> Result<()> {
        let repo_path = helper::handmade_git_repository()?;

        let first_pass = sample(&repo_path, 1, &[], 1, false)?;
        let rust_count_after_first: usize =
            first_pass.iter().filter(|r| r.language == "Rust").count();
        assert_eq!(rust_count_after_first, 1);

        let second_pass = sample(&repo_path, 5, &first_pass, 2, false)?;
        let rust_rows: Vec<&Row> = second_pass
            .iter()
            .filter(|r| r.language == "Rust")
            .collect();

        assert!(rust_rows.len() > rust_count_after_first);
        assert!(rust_rows.len() <= 5);

        let mut seen: HashSet<SampleKey> = HashSet::new();
        for row in &rust_rows {
            let key = (row.repository.clone(), row.commit.clone(), row.path.clone());
            assert!(seen.insert(key), "duplicate row sampled: {}", row.path);
        }

        Ok(())
    }

    fn args(repos_dir: &str, dataset: Option<&str>, stratified: bool) -> Args {
        Args {
            repos_dir: PathBuf::from(repos_dir),
            count: 1,
            output: None,
            seed: None,
            language: None,
            max_commits_per_repo: 1,
            dataset: dataset.map(str::to_string),
            stratified,
            content: Vec::new(),
            encodings: false,
            pictures: false,
            total: 0,
            changesets: false,
            range_length: None,
            dry_run: false,
            explain: false,
        }
    }

    #[test]
    fn dataset_defaults_to_the_repos_dir_parent_name() {
        let resolved = resolve_dataset(&args("/r/research/full/repositories", None, false));
        assert_eq!(resolved.unwrap(), "full");
        assert!(resolve_dataset(&args("/", None, false)).is_err());
    }

    #[test]
    fn stratified_defaults_to_the_stratified_dataset_and_rejects_any_other() {
        let checkout = "/r/research/small/repositories";
        assert_eq!(
            resolve_dataset(&args(checkout, None, true)).unwrap(),
            "stratified"
        );
        assert_eq!(
            resolve_dataset(&args(checkout, Some("stratified"), true)).unwrap(),
            "stratified"
        );
        assert!(resolve_dataset(&args(checkout, Some("small"), true)).is_err());
    }

    #[test]
    fn rows_without_dataset_or_status_columns_get_their_legacy_defaults() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let csv = dir.path().join("sample.csv");
        std::fs::write(
            &csv,
            "language,repository,commit,path,promoted_to\n\
             Rust,r,c,a.rs,\n\
             Rust,r,c,b.rs,rust-case\n",
        )?;
        let rows = read_existing_rows(&csv)?;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.dataset == "small"));
        assert_eq!(rows[0].status, "SAMPLED");
        assert_eq!(rows[1].status, "PROMOTED");
        Ok(())
    }

    /// Distinct lines of code, four of which make a block (`stats::changeset`).
    fn code(tag: &str, count: usize) -> String {
        (0..count)
            .map(|i| format!("let {tag}_{i} = compute_{tag}(input_{i}) + offset_{tag}_{i};\n"))
            .collect()
    }

    /// A repository under `root` with one commit a tree of `trees`, oldest first.
    fn linear_repository(root: &Path, trees: &[Vec<(&str, String)>]) -> Result<PathBuf> {
        let dir = root.join("repo");
        let repo = git2::Repository::init(&dir)?;
        let mut parent: Option<git2::Oid> = None;
        for (number, files) in trees.iter().enumerate() {
            let mut index = git2::Index::new()?;
            for (path, text) in files {
                index.add(&git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
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
            // A second apart, so the walk's time order is the commits' order.
            let when = git2::Time::new(1_700_000_000 + number as i64, 0);
            let signature = git2::Signature::new("t", "t@example.com", &when)?;
            let parents: Vec<git2::Commit> = parent
                .map(|oid| repo.find_commit(oid))
                .transpose()?
                .into_iter()
                .collect();
            let parents: Vec<&git2::Commit> = parents.iter().collect();
            parent =
                Some(repo.commit(Some("HEAD"), &signature, &signature, "c", &tree, &parents)?);
        }
        Ok(dir)
    }

    /// Six commits: the first tree, an edit of one file (no changeset), an edit of two (a
    /// control), a block moved from `a.rs` to `b.rs`, `c.rs` renamed as it is, and another control.
    fn changeset_history() -> Vec<Vec<(&'static str, String)>> {
        let (a, b, c, moved) = (code("a", 8), code("b", 8), code("c", 8), code("moved", 6));
        let tree = |a: String, b: String, c_path: &'static str| {
            vec![("a.rs", a), ("b.rs", b), (c_path, c.clone())]
        };
        vec![
            tree(format!("{a}{moved}"), b.clone(), "c.rs"),
            tree(format!("{a}{moved}// one\n"), b.clone(), "c.rs"),
            tree(
                format!("{a}{moved}// two\n"),
                format!("{b}// two\n"),
                "c.rs",
            ),
            tree(
                format!("{a}// two\n"),
                format!("{b}// two\n{moved}"),
                "c.rs",
            ),
            tree(
                format!("{a}// three\n"),
                format!("{b}// two\n{moved}"),
                "moved/c.rs",
            ),
            tree(
                format!("{a}// four\n"),
                format!("{b}// four\n{moved}"),
                "moved/c.rs",
            ),
        ]
    }

    fn changeset_args(root: &Path) -> Args {
        let mut args = args(root.to_str().unwrap(), Some("pilot"), false);
        args.changesets = true;
        args.count = 5;
        args.max_commits_per_repo = 100;
        args.seed = Some(7);
        args
    }

    #[test]
    fn changesets_are_sampled_by_stratum_and_topped_up() -> Result<()> {
        let root = tempfile::tempdir()?;
        linear_repository(root.path(), &changeset_history())?;
        let output = root.path().join("changesets.csv");
        let mut args = changeset_args(root.path());

        args.dry_run = true;
        sample_changesets(&args, &output)?;
        assert!(!output.exists(), "--dry-run writes nothing");

        args.dry_run = false;
        sample_changesets(&args, &output)?;
        let rows = changeset::read_rows(&output)?;
        let strata: Vec<(&str, &str, usize)> = rows
            .iter()
            .map(|row| (row.stratum.as_str(), row.bucket.as_str(), row.files))
            .collect();
        // Sorted by stratum; the one-file commit is no changeset, and a repository gives a cell
        // two at most (here it has just two controls).
        assert_eq!(
            strata,
            [
                ("block-move", "2-3", 2),
                ("control", "2-3", 2),
                ("control", "2-3", 2),
                ("rename-unedited", "2-3", 3),
            ]
        );
        let moved = &rows[0];
        assert_eq!(moved.signals, "block-move");
        assert_eq!(
            moved.name,
            format!("block-move-x-repo-{}", &moved.commit[..8])
        );
        assert_eq!(
            (moved.dataset.as_str(), moved.status.as_str()),
            ("pilot", "SAMPLED")
        );
        assert_eq!(moved.main_language, "Rust");
        assert_eq!(moved.changed_lines, 12);
        let repo = git2::Repository::open(root.path().join("repo"))?;
        let commit = repo.find_commit(git2::Oid::from_str(&moved.commit)?)?;
        assert_eq!(commit.parent_id(0)?.to_string(), moved.parent);

        // Again: every commit is already there, and a triaged row still counts and stays.
        let mut triaged = rows.clone();
        triaged[0].status = "REJECTED".to_string();
        triaged[0].comment = "not a move".to_string();
        changeset::write_rows(&output, triaged.clone())?;
        args.seed = Some(8);
        sample_changesets(&args, &output)?;
        assert_eq!(changeset::read_rows(&output)?, triaged);
        Ok(())
    }

    #[test]
    fn a_cell_is_topped_up_to_the_count_and_no_further() -> Result<()> {
        let root = tempfile::tempdir()?;
        let dir = linear_repository(root.path(), &changeset_history())?;
        let options = ChangesetOptions {
            max_commits: 100,
            range_length: None,
            count: 1,
            dataset: "pilot",
            explain: false,
        };
        let mut sample = ChangesetSample::default();
        sample_changeset_repository(
            &dir,
            "repo",
            &options,
            &mut sample,
            &mut StdRng::seed_from_u64(1),
        )?;
        let cell = |stratum: &str| (stratum.to_string(), "2-3".to_string());
        assert_eq!(sample.reservoirs[&cell("control")].items.len(), 1);
        // The census counts every changeset, sampled or not.
        assert_eq!(sample.census.cells["repo"][&cell("control")], 2);
        assert_eq!(sample.census.cells["repo"][&cell("block-move")], 1);
        assert_eq!(sample.census.skipped["repo"][&Skip::TooFewFiles], 1);

        // With the cell already full, nothing is offered room.
        let mut full = ChangesetSample::default();
        full.existing_counts.insert(cell("control"), 1);
        sample_changeset_repository(
            &dir,
            "repo",
            &options,
            &mut full,
            &mut StdRng::seed_from_u64(1),
        )?;
        assert!(full.reservoirs[&cell("control")].items.is_empty());
        assert_eq!(full.reservoirs[&cell("block-move")].items.len(), 1);
        Ok(())
    }

    #[test]
    fn a_range_squashes_consecutive_commits_without_overlap() -> Result<()> {
        let root = tempfile::tempdir()?;
        linear_repository(root.path(), &changeset_history())?;
        let output = root.path().join("changesets.csv");
        let mut args = changeset_args(root.path());
        args.range_length = Some(2);
        sample_changesets(&args, &output)?;
        let rows = changeset::read_rows(&output)?;
        // Five commits have a parent: two ranges of two, newest first, and one left over.
        assert_eq!(rows.len(), 2);
        let repo = git2::Repository::open(root.path().join("repo"))?;
        for row in &rows {
            assert_eq!(row.stratum, "range");
            assert!(row.name.starts_with("range-x-repo-"));
            let commit = repo.find_commit(git2::Oid::from_str(&row.commit)?)?;
            assert_eq!(
                commit.parent(0)?.parent_id(0)?.to_string(),
                row.parent,
                "the before tree is two commits back"
            );
        }
        let mut signals: Vec<&str> = rows.iter().map(|row| row.signals.as_str()).collect();
        signals.sort();
        // The newest two: a rename and an edit. The two before: the block move.
        assert_eq!(signals, ["range=2;block-move", "range=2;rename-unedited"]);
        assert_ne!(rows[0].parent, rows[1].parent);

        args.range_length = Some(1);
        assert!(sample_changesets(&args, &output).is_err());
        Ok(())
    }

    #[test]
    fn a_seeded_changeset_draw_repeats() -> Result<()> {
        let root = tempfile::tempdir()?;
        // Many controls, so that which two a repository offers is the seed's choice.
        let trees: Vec<Vec<(&str, String)>> = (0..12)
            .map(|i| {
                vec![
                    ("a.rs", format!("let a = {i};\n")),
                    ("b.rs", format!("let b = {i};\n")),
                ]
            })
            .collect();
        linear_repository(root.path(), &trees)?;
        let draw = |seed: u64| -> Result<Vec<changeset::Row>> {
            let output = root.path().join(format!("{seed}.csv"));
            let _ = fs::remove_file(&output);
            let mut args = changeset_args(root.path());
            args.seed = Some(seed);
            sample_changesets(&args, &output)?;
            changeset::read_rows(&output)
        };
        let first = draw(7)?;
        assert_eq!(first.len(), PAIRS_PER_REPOSITORY_PER_STRATUM);
        assert_eq!(draw(7)?, first);
        assert!((8..40).any(|seed| draw(seed).unwrap() != first));
        Ok(())
    }
}
