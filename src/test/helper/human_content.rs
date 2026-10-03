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

//! Human-authored ground truth for a container pair - an archive, a font, a PDF, a catalog, a
//! cursor theme (see `diff::content::container`):
//! `src/test/data/<family>/<name>/human_content.json`, written by `human_solver` beside the pair's
//! `before.<ext>.test` / `after.<ext>.test`.
//!
//! **A verdict per changed member**, the same verdicts a picture gets (`diff::content::Verdict`):
//! what happened to each glyph, page or file whose bytes changed. A member only one side has is a
//! fact, not a judgement, and gets none. When a pair has too many changed members to judge one by
//! one, a **pair verdict** stands in for them, and a fixture may carry both. A fixture is complete
//! when it has a pair verdict, or a verdict for every changed member.
//!
//! The fixtures live outside `src/test/data/diffs/`, the code corpus, for the reason
//! `human_picture` gives: everything that sweeps that corpus parses its pairs as text.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::diff::content::{self, ContentDiff, container::ContainerDiff};
pub use crate::diff::content::{Family, Verdict};

/// The fixture file's name in each fixture directory.
pub const FILE_NAME: &str = "human_content.json";

/// Where `family`'s fixtures live, one directory per fixture.
pub fn root(family: Family) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("test")
        .join("data")
        .join(family.name())
}

/// The ground truth file of `family`'s fixture `name`.
pub fn content_path(family: Family, name: &str) -> PathBuf {
    root(family).join(name).join(FILE_NAME)
}

/// What a human recorded about a container pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanContent {
    /// The verdict on the whole pair, when one was given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Verdict>,
    /// A verdict per changed member, by key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub members: BTreeMap<String, Verdict>,
    /// How many changed members had no verdict when this was saved, so a picker can tell a
    /// finished fixture without decoding its pair.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unjudged: usize,
}

fn is_zero(count: &usize) -> bool {
    *count == 0
}

impl HumanContent {
    /// A pair verdict, or a verdict for every changed member.
    pub fn is_complete(&self) -> bool {
        self.verdict.is_some() || (!self.members.is_empty() && self.unjudged == 0)
    }
}

pub fn load(family: Family, name: &str) -> Result<HumanContent> {
    let path = content_path(family, name);
    let contents = std::fs::read_to_string(&path).with_context(|| format!("reading {path:?}"))?;
    serde_json::from_str(&contents).with_context(|| format!("parsing {path:?}"))
}

pub fn save(family: Family, name: &str, content: &HumanContent) -> Result<()> {
    let path = content_path(family, name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(content)?;
    json.push('\n');
    std::fs::write(&path, json).with_context(|| format!("writing {path:?}"))
}

/// The engine's diff of `family`'s fixture `name`.
pub fn engine_diff(family: Family, name: &str) -> Result<ContainerDiff> {
    let dir = root(family).join(name);
    let (before, after) = super::human_picture::pair_bytes_in(&dir)?;
    match content::diff(&before, &after)? {
        Some(ContentDiff::Container(diff)) => Ok(diff),
        Some(ContentDiff::Picture(_)) => bail!("{dir:?} is a picture pair, not a container pair"),
        None => bail!("{dir:?} is not a pair omnidiff diffs by content"),
    }
}

/// Where the engine and the human disagree: the pair verdict (if the human gave one) and every
/// judged member, each with what the engine says. A judged member the engine does not see as
/// changed counts as the engine saying nothing (`None`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Mismatches {
    pub pair: Option<Verdict>,
    pub members: Vec<(String, Option<Verdict>)>,
}

impl Mismatches {
    pub fn is_empty(&self) -> bool {
        self.pair.is_none() && self.members.is_empty()
    }
}

/// [`Mismatches`] between `human` and `engine`.
pub fn mismatches(human: &HumanContent, engine: &ContainerDiff) -> Mismatches {
    let pair = human
        .verdict
        .filter(|verdict| *verdict != engine.verdict())
        .map(|_| engine.verdict());
    let members = human
        .members
        .iter()
        .filter_map(|(key, verdict)| {
            let found = engine.member(key).and_then(|member| member.verdict());
            (found != Some(*verdict)).then(|| (key.clone(), found))
        })
        .collect();
    Mismatches { pair, members }
}

/// Fails unless the engine agrees with every verdict the human gave on `family`'s fixture `name`.
pub fn assert_matches_human_verdicts(family: Family, name: &str) -> Result<()> {
    assert_known_verdict_mismatches(family, name, None, &[])
}

/// For a fixture the engine is known to get partly wrong: fails unless it disagrees with the human
/// exactly where recorded - the pair verdict as `pair` (`None` where it agrees) and the members in
/// `members`, each as the engine's verdict then - so a fix (or a different mistake) is noticed
/// and the stub is updated.
pub fn assert_known_verdict_mismatches(
    family: Family,
    name: &str,
    pair: Option<Verdict>,
    members: &[(&str, Option<Verdict>)],
) -> Result<()> {
    let human = load(family, name)?;
    let engine = engine_diff(family, name)?;
    let found = mismatches(&human, &engine);
    let expected = Mismatches {
        pair,
        members: members
            .iter()
            .map(|(key, verdict)| (key.to_string(), *verdict))
            .collect(),
    };
    if found != expected {
        let describe = |mismatches: &Mismatches| {
            let mut parts: Vec<String> = mismatches
                .pair
                .map(|verdict| format!("the pair: {}", verdict.label()))
                .into_iter()
                .collect();
            parts.extend(mismatches.members.iter().map(|(key, verdict)| {
                format!("{key}: {}", verdict.map_or("unchanged", Verdict::label))
            }));
            if parts.is_empty() {
                "nothing".to_string()
            } else {
                parts.join("; ")
            }
        };
        bail!(
            "{} '{name}': omnidiff now disagrees with the human on {}, not on {} as recorded; \
             update the stub",
            family.name(),
            describe(&found),
            describe(&expected)
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::content::container::{MemberDetail, MemberDiff, MemberStatus};

    fn engine() -> ContainerDiff {
        let text = |removed, added| MemberDetail::Text {
            before_lines: 10,
            after_lines: 10,
            removed,
            added,
        };
        ContainerDiff {
            family: Family::Archives,
            before: None,
            after: None,
            unchanged: 0,
            members: vec![
                MemberDiff {
                    key: "a".to_string(),
                    status: MemberStatus::Changed,
                    detail: Some(text(1, 1)),
                },
                MemberDiff {
                    key: "b".to_string(),
                    status: MemberStatus::Changed,
                    detail: Some(text(0, 0)),
                },
            ],
        }
    }

    #[test]
    fn mismatches_name_what_the_engine_says_instead() {
        let human = HumanContent {
            verdict: None,
            members: BTreeMap::from([
                ("a".to_string(), Verdict::ContentChange),
                ("b".to_string(), Verdict::ContentChange),
                ("gone".to_string(), Verdict::ContentChange),
            ]),
            unjudged: 0,
        };
        assert_eq!(
            mismatches(&human, &engine()),
            Mismatches {
                pair: None,
                members: vec![
                    ("b".to_string(), Some(Verdict::NoVisibleChange)),
                    ("gone".to_string(), None),
                ],
            }
        );
    }

    #[test]
    fn complete_means_a_pair_verdict_or_every_member() {
        let mut human = HumanContent::default();
        assert!(!human.is_complete());
        human.members.insert("a".to_string(), Verdict::Replaced);
        human.unjudged = 1;
        assert!(!human.is_complete());
        human.unjudged = 0;
        assert!(human.is_complete());
        let pair_only = HumanContent {
            verdict: Some(Verdict::ContentChange),
            ..HumanContent::default()
        };
        assert!(pair_only.is_complete());
        let json = serde_json::to_string(&pair_only).unwrap();
        assert_eq!(json, r#"{"verdict":"content_change"}"#);
    }
}
