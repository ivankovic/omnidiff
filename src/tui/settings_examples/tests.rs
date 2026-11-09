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
use super::*;
use crate::code::Code;

#[test]
fn every_example_parses_without_an_error() {
    for example in EXAMPLES {
        if example.language == Language::Unknown {
            continue;
        }
        for (side, text) in [("before", example.before), ("after", example.after)] {
            let code = Code::from_string(text, &example.language);
            let root = code
                .ast
                .as_ref()
                .unwrap_or_else(|| panic!("{} has no grammar", example.file_name))
                .root_node();
            assert!(
                !root.has_error(),
                "{} {side} does not parse cleanly",
                example.file_name
            );
        }
    }
}

/// The page's promise: selecting an option shows what it changes. Each option, flipped alone
/// from the `Minimal` preset or from the `Full` one, must repaint some byte of the example -
/// unless the example says why it cannot. Not from both: the options compose, so one can have
/// nothing to show while another is off (a re-indented block shows as moved only once displaced
/// code does), and the page names that other option instead.
#[test]
fn every_option_changes_every_example() {
    let mut missing = Vec::new();
    for example in EXAMPLES {
        for (index, (name, _)) in RenderOptions::FULL.options().into_iter().enumerate() {
            let declared = example.no_effect.iter().any(|&(i, _)| i == index);
            let shows = [RenderOptions::MINIMAL, RenderOptions::FULL]
                .into_iter()
                .any(|preset| {
                    let mut flipped = preset;
                    flipped.toggle(index);
                    painted(example, preset).map(|(painting, _)| painting)
                        != painted(example, flipped).map(|(painting, _)| painting)
                });
            if shows == declared {
                missing.push(format!(
                    "{}: {name} {}",
                    example.file_name,
                    if declared {
                        "is declared without effect but changes the example"
                    } else {
                        "changes nothing"
                    }
                ));
            }
        }
    }
    assert!(missing.is_empty(), "\n  {}", missing.join("\n  "));
}

#[test]
fn an_unknown_or_missing_language_falls_back() {
    assert_eq!(example_for(None).language, Language::Rust);
    assert_eq!(example_for(Some(Language::Rust)).language, Language::Rust);
    assert_eq!(
        example_for(Some(Language::MarkDown)).language,
        Language::Unknown
    );
}
