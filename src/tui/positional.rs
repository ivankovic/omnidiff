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
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU Affero General Public License for more details.
 *
 *  You should have received a copy of the GNU Affero General Public License
 *  along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */
//! The positional-argument conventions `omnidiff` accepts, and the notice it prints for a binary
//! pair. The tests live in `main.rs`, next to the CLI behaviour they also exercise.

use std::path::{Path, PathBuf};

use anyhow::Result;

/// Resolves the positional arguments into a `(before, after)` pair, or `None` for an empty
/// viewer. The shape is chosen by count alone:
///
/// * **2**: `BEFORE AFTER` (plain CLI and `git difftool`).
/// * **7**: git's `GIT_EXTERNAL_DIFF` form, `path old-file old-hex old-mode new-file new-hex
///   new-mode`; only `old-file` and `new-file` are used, since they keep the real extension. An
///   add/delete arrives as `/dev/null` and is passed through for `compute_diff` to handle.
/// * **9**: the same form for a rename or copy, with two extra trailing arguments that are ignored.
///
/// Any other count is an error rather than a guess.
pub fn resolve_before_after(paths: &[PathBuf]) -> Result<Option<(PathBuf, PathBuf)>> {
    match paths.len() {
        0 => Ok(None),
        2 => Ok(Some((paths[0].clone(), paths[1].clone()))),
        7 | 9 => Ok(Some((paths[1].clone(), paths[4].clone()))),
        n => anyhow::bail!(
            "expected 0 positional arguments (empty viewer), 2 (BEFORE AFTER), or 7 \
            (GIT_EXTERNAL_DIFF's `path old-file old-hex old-mode new-file new-hex new-mode`, \
            or 9 with git's two extra rename/copy arguments), got {n}"
        ),
    }
}

/// Whether the arguments have the `GIT_EXTERNAL_DIFF` shape that `resolve_before_after`
/// recognizes (7 or 9). One predicate so every caller moves together if git adds an argument.
pub fn invoked_as_git_external_diff(paths: &[PathBuf]) -> bool {
    matches!(paths.len(), 7 | 9)
}

/// The headless report of a pair diffed by content (see [`crate::diff::content`]).
pub fn content_notice(
    paths: &[PathBuf],
    before: &Path,
    after: &Path,
    diff: &crate::diff::content::ContentDiff,
) -> String {
    match diff {
        crate::diff::content::ContentDiff::Picture(picture) => {
            picture_notice(paths, before, after, picture)
        }
        crate::diff::content::ContentDiff::Container(container) => {
            container_notice(paths, before, after, container)
        }
    }
}

/// The headless report of a container pair (see [`crate::diff::content::container`]): what each
/// side is, how many members changed, and the first [`LISTED_MEMBERS`] of them with what changed.
pub fn container_notice(
    paths: &[PathBuf],
    before: &Path,
    after: &Path,
    diff: &crate::diff::content::container::ContainerDiff,
) -> String {
    use crate::diff::content::container::{ContainerInfo, MemberStatus};

    let name = if invoked_as_git_external_diff(paths) {
        format!("{} {}", diff.family.noun(false), paths[0].display())
    } else {
        format!(
            "{} {} and {}",
            diff.family.noun(true),
            before.display(),
            after.display()
        )
    };
    let side = |info: &Option<ContainerInfo>| match info {
        Some(info) => format!(
            "{}, {} member{}, {} bytes",
            info.format,
            info.members,
            if info.members == 1 { "" } else { "s" },
            info.bytes
        ),
        None => "nothing".to_string(),
    };
    let mut out = format!("{name}: {} -> {}\n", side(&diff.before), side(&diff.after));
    let count = |status| {
        diff.members
            .iter()
            .filter(|member| member.status == status)
            .count()
    };
    out.push_str(&format!(
        "  {} changed, {} added, {} removed, {} unchanged\n",
        count(MemberStatus::Changed),
        count(MemberStatus::Added),
        count(MemberStatus::Removed),
        diff.unchanged
    ));
    for member in diff.members.iter().take(LISTED_MEMBERS) {
        let line = match (member.status, &member.detail) {
            (MemberStatus::Changed, Some(detail)) => {
                format!("changed  {}: {}", member.key, member_summary(detail))
            }
            (MemberStatus::Changed, None) => format!("changed  {}: does not open", member.key),
            (MemberStatus::Added, _) => format!("added    {}", member.key),
            (MemberStatus::Removed, _) => format!("removed  {}", member.key),
            (MemberStatus::Same, _) => continue,
        };
        out.push_str(&format!("    {line}\n"));
    }
    if diff.members.len() > LISTED_MEMBERS {
        out.push_str(&format!(
            "    ... {} more\n",
            diff.members.len() - LISTED_MEMBERS
        ));
    }
    out
}

/// How many members [`container_notice`] lists.
pub const LISTED_MEMBERS: usize = 20;

/// One changed member's change, in a few words.
fn member_summary(detail: &crate::diff::content::container::MemberDetail) -> String {
    use crate::diff::content::ContentDiff;
    use crate::diff::content::container::{MemberDetail, MemberStatus};
    match detail {
        MemberDetail::Picture(picture) => picture_summary(picture),
        MemberDetail::Content { content } => match content.as_ref() {
            ContentDiff::Picture(picture) => picture_summary(picture),
            ContentDiff::Container(container) => {
                let count = |status| {
                    container
                        .members
                        .iter()
                        .filter(|member| member.status == status)
                        .count()
                };
                format!(
                    "{} with {} changed, {} added, {} removed",
                    container.family.noun(false).to_lowercase(),
                    count(MemberStatus::Changed),
                    count(MemberStatus::Added),
                    count(MemberStatus::Removed)
                )
            }
        },
        MemberDetail::Text {
            removed: 0,
            added: 0,
            ..
        } => "only line endings or the encoding changed".to_string(),
        MemberDetail::Text { removed, added, .. } => {
            let noun = if *removed == 1 { "line" } else { "lines" };
            format!("{removed} {noun} removed, {added} added")
        }
        MemberDetail::Binary {
            before_bytes,
            after_bytes,
        } => format!("{before_bytes} -> {after_bytes} bytes"),
    }
}

/// A picture diff in one phrase, for a list of members.
fn picture_summary(diff: &crate::diff::picture::PictureDiff) -> String {
    use crate::diff::picture::{Comparison, FrameCounts};
    match &diff.comparison {
        Comparison::OneSided => "added or removed".to_string(),
        Comparison::Resized => match (&diff.before, &diff.after) {
            (Some(before), Some(after)) => format!(
                "resized, {}x{} -> {}x{}",
                before.width, before.height, after.width, after.height
            ),
            _ => "resized".to_string(),
        },
        Comparison::Frames { steps, .. } => FrameCounts::of(steps).describe(),
        Comparison::Pixels { regions, .. } if regions.is_empty() => "no pixel changed".to_string(),
        Comparison::Pixels {
            changed_pixels,
            total_pixels,
            regions,
        } => format!(
            "{:.2}% of pixels changed, in {} region{}",
            100.0 * *changed_pixels as f64 / (*total_pixels).max(1) as f64,
            regions.len(),
            if regions.len() == 1 { "" } else { "s" }
        ),
    }
}

/// The headless report of a picture pair (see [`crate::diff::picture`]), named the way
/// [`binary_notice`] names a pair: what each side is, then how much changed and where. Lists the
/// ten largest regions.
pub fn picture_notice(
    paths: &[PathBuf],
    before: &Path,
    after: &Path,
    diff: &crate::diff::picture::PictureDiff,
) -> String {
    use crate::diff::picture::{Comparison, FrameCounts, FrameStep, PictureInfo};
    const LISTED_REGIONS: usize = 10;
    const LISTED_STEPS: usize = 10;

    let name = if invoked_as_git_external_diff(paths) {
        format!("Picture {}", paths[0].display())
    } else {
        format!("Pictures {} and {}", before.display(), after.display())
    };
    let side = |info: &Option<PictureInfo>| match info {
        Some(info) if info.frames > 1 => format!(
            "{} {}x{} {}, {} frames, {:.1}s, {} bytes",
            info.format,
            info.width,
            info.height,
            info.color,
            info.frames,
            info.duration_ms as f64 / 1000.0,
            info.bytes
        ),
        Some(info) => format!(
            "{} {}x{} {}, {} bytes",
            info.format, info.width, info.height, info.color, info.bytes
        ),
        None => "nothing".to_string(),
    };
    let mut out = format!("{name}: {} -> {}\n", side(&diff.before), side(&diff.after));
    match &diff.comparison {
        Comparison::OneSided => {}
        Comparison::Frames { steps, retimed, .. } => {
            out.push_str(&format!("  {}", FrameCounts::of(steps).describe()));
            if *retimed {
                out.push_str("; frames that look the same show for different times");
            }
            out.push('\n');
            let range = |from: usize, frames: usize| match frames {
                1 => format!("frame {from}"),
                _ => format!("frames {from}-{}", from + frames - 1),
            };
            let listed: Vec<&FrameStep> = steps
                .iter()
                .filter(|step| !matches!(step, FrameStep::Same { .. }))
                .collect();
            for step in listed.iter().take(LISTED_STEPS) {
                let line = match step {
                    FrameStep::Changed {
                        before,
                        after,
                        regions,
                        ..
                    } => format!(
                        "frame {before} -> {after}: changed, in {} region{}",
                        regions.len(),
                        if regions.len() == 1 { "" } else { "s" }
                    ),
                    FrameStep::Inserted { after, frames } => {
                        format!("{} added", range(*after, *frames))
                    }
                    FrameStep::Deleted { before, frames } => {
                        format!("{} removed", range(*before, *frames))
                    }
                    FrameStep::Same { .. } => unreachable!("filtered out above"),
                };
                out.push_str(&format!("    {line}\n"));
            }
            if listed.len() > LISTED_STEPS {
                out.push_str(&format!("    ... {} more\n", listed.len() - LISTED_STEPS));
            }
        }
        Comparison::Resized => out.push_str("  resized, so not compared pixel by pixel\n"),
        Comparison::Pixels { regions, .. } if regions.is_empty() => {
            out.push_str("  no pixel changed\n");
        }
        Comparison::Pixels {
            changed_pixels,
            total_pixels,
            regions,
        } => {
            let share = 100.0 * *changed_pixels as f64 / (*total_pixels).max(1) as f64;
            let noun = if regions.len() == 1 {
                "region"
            } else {
                "regions"
            };
            out.push_str(&format!(
                "  {share:.2}% of pixels changed ({changed_pixels} of {total_pixels}), in {} {noun}:\n",
                regions.len()
            ));
            for region in regions.iter().take(LISTED_REGIONS) {
                out.push_str(&format!(
                    "    {}x{} at ({}, {})\n",
                    region.width, region.height, region.x, region.y
                ));
            }
            if regions.len() > LISTED_REGIONS {
                out.push_str(&format!(
                    "    ... {} more\n",
                    regions.len() - LISTED_REGIONS
                ));
            }
        }
    }
    out
}

/// The one-line stand-in for a binary diff, worded like git's. Under `GIT_EXTERNAL_DIFF` both
/// sides are temp blobs, so it names git's repo-relative `path` once instead. `differed` is a
/// byte comparison: a direct invocation may pass identical files.
pub fn binary_notice(paths: &[PathBuf], before: &Path, after: &Path, differed: bool) -> String {
    if invoked_as_git_external_diff(paths) {
        let name = paths[0].display();
        return match differed {
            true => format!("Binary file {name} differs\n"),
            false => format!("Binary file {name} is unchanged\n"),
        };
    }
    let (before, after) = (before.display(), after.display());
    match differed {
        true => format!("Binary files {before} and {after} differ\n"),
        false => format!("Binary files {before} and {after} are identical\n"),
    }
}
