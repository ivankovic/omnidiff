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
use anyhow::Result;

use crate::test::helper::human_content::{self, Family};

#[test]
fn verdicts() -> Result<()> {
    // Recorded as found, not examined.
    human_content::assert_known_verdict_mismatches(
        Family::Catalogs,
        "mo-x-adammajer-lpe-0d8fb399-fr",
        Some("replaced+members"),
        &[
            ("Current buffer modified", Some("replaced")),
            ("File saved successfully", Some("redrawn")),
            ("Not found", Some("replaced")),
            ("Unknown debug request", Some("redrawn")),
            ("Using hard tabs", Some("replaced")),
            ("Using soft tabs", Some("replaced")),
            ("debug: ", Some("replaced")),
            ("file: ", Some("replaced")),
            ("line: ", Some("replaced")),
            ("search term: ", Some("replaced")),
            (
                "usage:\\t'lpe [--mode mode] [--] file'\\topens file for editing\\n      \\t'lpe --version'              \\tprints version number\\n      \\t'lpe --help'                 \\tprints this help message\\n",
                Some("replaced"),
            ),
        ],
    )
}
