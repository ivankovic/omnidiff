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

//! Human-authored ground truth for a content pair - a picture, an archive, a font, a PDF, a
//! catalog, a cursor (see `diff::content`): `src/test/data/<family>/<name>/human_content.json`,
//! written by `human_solver` beside the pair's `before.<ext>.test` / `after.<ext>.test`.
//!
//! **A verdict, not regions** (`diff::content::Verdict`: a level and its tags). Which regions
//! changed is postponed: drawing them by hand in a terminal is the expensive part, and the verdict
//! already tests what the engine decides first.
//!
//! **A picture gets one verdict, for the pair. A container gets one per changed member**: what
//! happened to each glyph, page or file whose bytes changed. A member only one side has is a
//! fact, not a judgement, and gets none. When a container has too many changed members to judge
//! one by one, a **pair verdict** stands in for them, and a fixture may carry both. A fixture is
//! complete when it has a pair verdict, or a verdict for every changed member.
//!
//! **Or none: "can't judge"**, with a note, where OmniDiff draws a side wrong or not at all. That
//! records a defect rather than a verdict on a picture nobody can trust, and is not compared with
//! the engine.
//!
//! The file spells each verdict as `level+tag` (`"artifacts+resized"`), and can't judge as
//! `"cant_judge"` or, with a note, `{"cant_judge": "the glyphs are blank"}`:
//!
//! ```json
//! { "verdict": "edited", "members": { "U+0041 A": "redrawn", "U+00E9 é": "cant_judge" } }
//! ```
//!
//! The fixtures live outside `src/test/data/diffs/`, the code corpus, because everything that
//! sweeps that corpus parses its pairs as text.

use std::collections::BTreeMap;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

use crate::diff::content::{self, ContentDiff};
pub use crate::diff::content::{Family, Level, Tag, Verdict};

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

/// What a human said about a pair or a member: a verdict, or that it cannot be judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Judgement {
    Verdict(Verdict),
    /// OmniDiff draws a side wrong or not at all; the note says how, and may be empty.
    CantJudge(String),
}

/// How the file spells can't judge.
const CANT_JUDGE: &str = "cant_judge";

impl Judgement {
    pub fn verdict(&self) -> Option<Verdict> {
        match self {
            Judgement::Verdict(verdict) => Some(*verdict),
            Judgement::CantJudge(_) => None,
        }
    }

    /// `artifacts+resized`, or `can't judge: <note>`.
    pub fn label(&self) -> String {
        self.label_for(false)
    }

    /// [`Judgement::label`] with the level named for text if `text` (`formatting`).
    pub fn label_for(&self, text: bool) -> String {
        match self {
            Judgement::Verdict(verdict) => verdict.label_for(text),
            Judgement::CantJudge(note) if note.is_empty() => "can't judge".to_string(),
            Judgement::CantJudge(note) => format!("can't judge: {note}"),
        }
    }
}

impl Serialize for Judgement {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Judgement::Verdict(verdict) => serializer.serialize_str(&verdict.to_string()),
            Judgement::CantJudge(note) if note.is_empty() => serializer.serialize_str(CANT_JUDGE),
            Judgement::CantJudge(note) => {
                use serde::ser::SerializeMap;
                let mut map = serializer.serialize_map(Some(1))?;
                map.serialize_entry(CANT_JUDGE, note)?;
                map.end()
            }
        }
    }
}

impl<'de> Deserialize<'de> for Judgement {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Spelled {
            Short(String),
            Noted { cant_judge: String },
        }
        match Spelled::deserialize(deserializer)? {
            Spelled::Short(text) if text == CANT_JUDGE => Ok(Judgement::CantJudge(String::new())),
            Spelled::Short(text) => text
                .parse()
                .map(Judgement::Verdict)
                .map_err(serde::de::Error::custom),
            Spelled::Noted { cant_judge } => Ok(Judgement::CantJudge(cant_judge)),
        }
    }
}

/// What a human recorded about a content pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HumanContent {
    /// The verdict on the whole pair, when one was given: a picture's only one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verdict: Option<Judgement>,
    /// A verdict per changed member, by key.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub members: BTreeMap<String, Judgement>,
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

/// The two files of the pair in fixture directory `dir`: `before.<ext>.test` and
/// `after.<ext>.test`.
pub fn pair_bytes_in(dir: &std::path::Path) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut before = None;
    let mut after = None;
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {dir:?}"))? {
        let path = entry?.path();
        let file = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        if !file.ends_with(".test") {
            continue;
        }
        if file.starts_with("before.") {
            before = Some(std::fs::read(&path)?);
        } else if file.starts_with("after.") {
            after = Some(std::fs::read(&path)?);
        }
    }
    match (before, after) {
        (Some(before), Some(after)) => Ok((before, after)),
        _ => bail!("{dir:?} has no before/after pair"),
    }
}

/// The engine's diff of `family`'s fixture `name`.
pub fn engine_diff(family: Family, name: &str) -> Result<ContentDiff> {
    let dir = root(family).join(name);
    let (before, after) = pair_bytes_in(&dir)?;
    content::diff(&before, &after)?
        .with_context(|| format!("{dir:?} is not a pair omnidiff diffs by content"))
}

/// Where the engine and the human disagree: the pair verdict (if the human gave one) and every
/// judged member, each with what the engine says. A judged member the engine does not see as
/// changed counts as the engine saying nothing (`None`). Can't judge is never a disagreement.
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
pub fn mismatches(human: &HumanContent, engine: &ContentDiff) -> Mismatches {
    let found = engine.verdict();
    let pair = human
        .verdict
        .as_ref()
        .and_then(Judgement::verdict)
        .filter(|verdict| *verdict != found)
        .map(|_| found);
    let container = match engine {
        ContentDiff::Container(diff) => Some(diff),
        ContentDiff::Picture(_) => None,
    };
    let members = human
        .members
        .iter()
        .filter_map(|(key, judgement)| {
            let verdict = judgement.verdict()?;
            let found = container
                .and_then(|diff| diff.member(key))
                .and_then(|member| member.verdict());
            (found != Some(verdict)).then(|| (key.clone(), found))
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
/// `members`, each as the engine's verdict then, spelled `level+tag` - so a fix (or a different
/// mistake) is noticed and the stub is updated.
pub fn assert_known_verdict_mismatches(
    family: Family,
    name: &str,
    pair: Option<&str>,
    members: &[(&str, Option<&str>)],
) -> Result<()> {
    let human = load(family, name)?;
    let engine = engine_diff(family, name)?;
    let found = mismatches(&human, &engine);
    let parse = |verdict: Option<&str>| verdict.map(str::parse::<Verdict>).transpose();
    let expected = Mismatches {
        pair: parse(pair)?,
        members: members
            .iter()
            .map(|(key, verdict)| Ok((key.to_string(), parse(*verdict)?)))
            .collect::<Result<_>>()?,
    };
    if found != expected {
        let describe = |mismatches: &Mismatches| {
            let mut parts: Vec<String> = mismatches
                .pair
                .map(|verdict| format!("the pair: {}", verdict.label()))
                .into_iter()
                .collect();
            parts.extend(mismatches.members.iter().map(|(key, verdict)| {
                format!(
                    "{key}: {}",
                    verdict.map_or("unchanged".to_string(), Verdict::label)
                )
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
    use crate::diff::content::container::{ContainerDiff, MemberDetail, MemberDiff, MemberStatus};

    fn engine() -> ContentDiff {
        let text = |removed, added| MemberDetail::Text {
            before_lines: 10,
            after_lines: 10,
            removed,
            added,
            whitespace: None,
            words_kept: true,
        };
        ContentDiff::Container(ContainerDiff {
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
                MemberDiff {
                    key: "c".to_string(),
                    status: MemberStatus::Changed,
                    detail: Some(text(9, 9)),
                },
            ],
        })
    }

    fn judged(text: &str) -> Judgement {
        Judgement::Verdict(text.parse().unwrap())
    }

    #[test]
    fn mismatches_name_what_the_engine_says_instead() {
        let human = HumanContent {
            verdict: None,
            members: BTreeMap::from([
                ("a".to_string(), judged("edited")),
                ("b".to_string(), judged("edited")),
                (
                    "c".to_string(),
                    Judgement::CantJudge("drawn blank".to_string()),
                ),
                ("gone".to_string(), judged("edited")),
            ]),
            unjudged: 0,
        };
        assert_eq!(
            mismatches(&human, &engine()),
            Mismatches {
                pair: None,
                members: vec![
                    ("b".to_string(), Some(Verdict::new(Level::Invisible))),
                    ("gone".to_string(), None),
                ],
            },
            "can't judge is no disagreement"
        );
    }

    #[test]
    fn complete_means_a_pair_verdict_or_every_member() {
        let mut human = HumanContent::default();
        assert!(!human.is_complete());
        human.members.insert("a".to_string(), judged("replaced"));
        human.unjudged = 1;
        assert!(!human.is_complete());
        human.unjudged = 0;
        assert!(human.is_complete());
        let pair_only = HumanContent {
            verdict: Some(judged("edited")),
            ..HumanContent::default()
        };
        assert!(pair_only.is_complete());
    }

    #[test]
    fn verdicts_are_spelled_level_and_tags_and_cant_judge_may_take_a_note() {
        let human = HumanContent {
            verdict: Some(judged("artifacts+resized+timing")),
            members: BTreeMap::from([
                ("a".to_string(), Judgement::CantJudge(String::new())),
                (
                    "b".to_string(),
                    Judgement::CantJudge("blank glyph".to_string()),
                ),
            ]),
            unjudged: 0,
        };
        let json = serde_json::to_string(&human).unwrap();
        assert_eq!(
            json,
            r#"{"verdict":"artifacts+resized+timing","members":{"a":"cant_judge","b":{"cant_judge":"blank glyph"}}}"#
        );
        assert_eq!(serde_json::from_str::<HumanContent>(&json).unwrap(), human);
        assert!(serde_json::from_str::<HumanContent>(r#"{"verdict":"content_change"}"#).is_err());
        assert!(serde_json::from_str::<HumanContent>(r#"{"verdict":"edited+wobbly"}"#).is_err());
    }
}
