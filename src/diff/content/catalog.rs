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

//! Compiled message catalogs as containers of messages: GNU gettext's `.mo` and Qt's `.qm`, the
//! binary forms of `.po` and `.ts` files that projects commit beside their sources.
//!
//! A member is one message, **keyed by what it translates** - its context, source text and
//! disambiguating comment (`menu | Open | verb`) - and holding its translation as text, plural
//! forms one per line. A catalog recompiled from unchanged sources has no changed member; an
//! edited translation is one changed member, shown as a line diff.
//!
//! **`.mo`** (gettext): a header, then two tables of (length, offset) pairs, originals and
//! translations, N each. An original is `context\x04msgid`, with a plural's `msgid_plural` after a
//! NUL; a translation's plural forms are separated by NULs. The message with an empty msgid is the
//! catalog's header (`Content-Type`, `Plural-Forms`, ...), keyed `(header)`. Text is read as UTF-8
//! (the charset nearly every catalog declares), lossily otherwise.
//!
//! **`.qm`** (Qt): a 16-byte magic, then tagged blocks (tag byte, big-endian length); the messages
//! block is a run of tagged fields per message - translation (UTF-16BE), source text, context,
//! comment - each message ended by an end tag. A catalog compiled without source texts keys its
//! messages by their hash instead.

use anyhow::{Context, Result, bail};

use super::container::{Container, ContainerInfo, Member, MemberContent, hash_bytes};

/// Qt's `.qm` magic.
const QM_MAGIC: [u8; 16] = [
    0x3C, 0xB8, 0x64, 0x18, 0xCA, 0xEF, 0x9C, 0x95, 0xCD, 0x21, 0x1C, 0xBF, 0x60, 0xA1, 0xBD, 0xDD,
];

/// True if `bytes` are a gettext `.mo`, in either byte order.
pub fn is_mo(bytes: &[u8]) -> bool {
    matches!(
        bytes.get(0..4),
        Some([0xDE, 0x12, 0x04, 0x95] | [0x95, 0x04, 0x12, 0xDE])
    )
}

/// True if `bytes` are a Qt `.qm`.
pub fn is_qm(bytes: &[u8]) -> bool {
    bytes.starts_with(&QM_MAGIC)
}

/// A catalog's messages: (key, translation).
pub struct Catalog {
    info: ContainerInfo,
    members: Vec<Member>,
    texts: Vec<String>,
}

impl Catalog {
    fn new(format: &str, bytes: usize, mut messages: Vec<(String, String)>) -> Self {
        // A key twice (a malformed catalog) would match the wrong message: the first one wins.
        let mut seen = std::collections::HashSet::new();
        messages.retain(|(key, _)| seen.insert(key.clone()));
        let members = messages
            .iter()
            .map(|(key, text)| Member {
                key: key.clone(),
                hash: hash_bytes(text.as_bytes()),
            })
            .collect::<Vec<_>>();
        Self {
            info: ContainerInfo {
                format: format.to_string(),
                bytes,
                members: members.len(),
            },
            members,
            texts: messages.into_iter().map(|(_, text)| text).collect(),
        }
    }

    pub fn mo(bytes: &[u8]) -> Result<Self> {
        Ok(Self::new("MO", bytes.len(), mo_messages(bytes)?))
    }

    pub fn qm(bytes: &[u8]) -> Result<Self> {
        Ok(Self::new("QM", bytes.len(), qm_messages(bytes)?))
    }
}

impl Container for Catalog {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        Ok(MemberContent::Text(self.texts[index].clone()))
    }
}

/// A message's key from its parts, the empty ones left out, and line breaks and tabs written as
/// escapes: a key is shown on one line of a member list.
fn key(parts: &[&str]) -> String {
    let parts: Vec<&str> = parts
        .iter()
        .copied()
        .filter(|part| !part.is_empty())
        .collect();
    parts
        .join(" | ")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

/// A `.mo`'s messages, in its (sorted) order.
fn mo_messages(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let big = bytes.starts_with(&[0x95, 0x04, 0x12, 0xDE]);
    let word = |at: usize| -> Result<usize> {
        let b: [u8; 4] = bytes
            .get(at..at + 4)
            .context("a short .mo")?
            .try_into()
            .expect("four bytes");
        Ok(if big {
            u32::from_be_bytes(b)
        } else {
            u32::from_le_bytes(b)
        } as usize)
    };
    let (count, originals, translations) = (word(8)?, word(12)?, word(16)?);
    if count > bytes.len() / 16 {
        bail!("a .mo of {count} messages in {} bytes", bytes.len());
    }
    let string = |table: usize, index: usize| -> Result<String> {
        let (length, offset) = (word(table + index * 8)?, word(table + index * 8 + 4)?);
        let raw = bytes
            .get(offset..offset + length)
            .context("a .mo string past the end")?;
        Ok(String::from_utf8_lossy(raw).into_owned())
    };
    let mut messages = Vec::with_capacity(count);
    for index in 0..count {
        let original = string(originals, index)?;
        let translation = string(translations, index)?;
        let (context, id) = original.split_once('\u{4}').unwrap_or(("", &original));
        // A plural's msgid_plural follows its msgid; the msgid alone keys it.
        let id = id.split('\0').next().unwrap_or_default();
        let message_key = if context.is_empty() && id.is_empty() {
            "(header)".to_string()
        } else {
            key(&[context, id])
        };
        messages.push((message_key, translation.replace('\0', "\n")));
    }
    Ok(messages)
}

/// The tags of a `.qm`'s message fields (`Tag` in Qt's `qtranslator.cpp`).
mod tag {
    pub const END: u8 = 1;
    pub const SOURCE_TEXT_16: u8 = 2;
    pub const TRANSLATION: u8 = 3;
    pub const CONTEXT_16: u8 = 4;
    pub const OBSOLETE_1: u8 = 5;
    pub const SOURCE_TEXT: u8 = 6;
    pub const CONTEXT: u8 = 7;
    pub const COMMENT: u8 = 8;
}

/// The block of a `.qm` that holds its messages.
const QM_MESSAGES: u8 = 0x69;

/// A `.qm`'s messages, in its order.
fn qm_messages(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let be32 = |data: &[u8], at: usize| -> Result<u32> {
        Ok(u32::from_be_bytes(
            data.get(at..at + 4)
                .context("a short .qm")?
                .try_into()
                .expect("four bytes"),
        ))
    };
    let mut at = QM_MAGIC.len();
    let mut block = None;
    while at + 5 <= bytes.len() {
        let (tag, length) = (bytes[at], be32(bytes, at + 1)? as usize);
        let data = bytes
            .get(at + 5..at + 5 + length)
            .context("a .qm block past the end")?;
        if tag == QM_MESSAGES {
            block = Some(data);
        }
        at += 5 + length;
    }
    let Some(data) = block else {
        return Ok(Vec::new());
    };

    let mut messages = Vec::new();
    let (mut source, mut context, mut comment, mut hash) =
        (String::new(), String::new(), String::new(), None);
    let mut translations: Vec<String> = Vec::new();
    let mut at = 0;
    while at < data.len() {
        let field = data[at];
        at += 1;
        let bytes_field = |at: &mut usize| -> Result<&[u8]> {
            let length = be32(data, *at)?;
            *at += 4;
            // A null translation is written with the length 0xFFFFFFFF and no bytes.
            if length == u32::MAX {
                return Ok(&[]);
            }
            let value = data
                .get(*at..*at + length as usize)
                .context("a .qm field past the end")?;
            *at += length as usize;
            Ok(value)
        };
        match field {
            tag::END => {
                let message_key = if source.is_empty() {
                    let hash = hash.map_or("-".to_string(), |hash| format!("{hash:08x}"));
                    key(&[&context, &format!("(hash {hash})"), &comment])
                } else {
                    key(&[&context, &source, &comment])
                };
                messages.push((message_key, translations.join("\n")));
                (source, context, comment, hash) =
                    (String::new(), String::new(), String::new(), None);
                translations.clear();
            }
            tag::TRANSLATION => {
                let raw = bytes_field(&mut at)?;
                let units: Vec<u16> = raw
                    .chunks_exact(2)
                    .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
                    .collect();
                translations.push(String::from_utf16_lossy(&units));
            }
            tag::SOURCE_TEXT | tag::SOURCE_TEXT_16 => {
                source = String::from_utf8_lossy(bytes_field(&mut at)?).into_owned();
            }
            tag::CONTEXT | tag::CONTEXT_16 => {
                context = String::from_utf8_lossy(bytes_field(&mut at)?).into_owned();
            }
            tag::COMMENT => {
                comment = String::from_utf8_lossy(bytes_field(&mut at)?).into_owned();
            }
            tag::OBSOLETE_1 => {
                hash = Some(be32(data, at)?);
                at += 4;
            }
            other => bail!("a .qm message field of unknown tag {other:#x}"),
        }
    }
    Ok(messages)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A little-endian `.mo` of (original, translation) pairs, in the order given.
    pub(crate) fn mo(messages: &[(&str, &str)]) -> Vec<u8> {
        let count = messages.len();
        let (originals, translations) = (28, 28 + count * 8);
        let mut strings = Vec::new();
        let mut tables = [Vec::new(), Vec::new()];
        let data_start = 28 + count * 16;
        for (original, translation) in messages {
            for (table, text) in [(0, original), (1, translation)] {
                let offset = data_start + strings.len();
                tables[table].extend_from_slice(&(text.len() as u32).to_le_bytes());
                tables[table].extend_from_slice(&(offset as u32).to_le_bytes());
                strings.extend_from_slice(text.as_bytes());
                strings.push(0);
            }
        }
        let mut bytes = Vec::new();
        for word in [
            0x9504_12de_u32,
            0,
            count as u32,
            originals as u32,
            translations as u32,
            0,
            0,
        ] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        bytes.extend(tables.concat());
        bytes.extend(strings);
        bytes
    }

    /// A `.qm` of (context, source, translation) messages.
    pub(crate) fn qm(messages: &[(&str, &str, &str)]) -> Vec<u8> {
        let mut block = Vec::new();
        let field = |block: &mut Vec<u8>, tag: u8, value: &[u8]| {
            block.push(tag);
            block.extend_from_slice(&(value.len() as u32).to_be_bytes());
            block.extend_from_slice(value);
        };
        for (context, source, translation) in messages {
            let utf16: Vec<u8> = translation
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect();
            field(&mut block, tag::TRANSLATION, &utf16);
            field(&mut block, tag::SOURCE_TEXT, source.as_bytes());
            field(&mut block, tag::CONTEXT, context.as_bytes());
            block.push(tag::END);
        }
        let mut bytes = QM_MAGIC.to_vec();
        bytes.push(QM_MESSAGES);
        bytes.extend_from_slice(&(block.len() as u32).to_be_bytes());
        bytes.extend(block);
        bytes
    }

    fn messages(catalog: &Catalog) -> Vec<(String, String)> {
        (0..catalog.members().len())
            .map(|index| {
                let MemberContent::Text(text) = catalog.open(index).unwrap() else {
                    panic!("a message is text");
                };
                (catalog.members()[index].key.clone(), text)
            })
            .collect()
    }

    #[test]
    fn a_mo_is_its_messages_keyed_by_what_they_translate() -> Result<()> {
        let bytes = mo(&[
            ("", "Content-Type: text/plain; charset=UTF-8\n"),
            ("Open", "Öffnen"),
            ("menu\u{4}File", "Datei"),
            ("%d file\0%d files", "%d Datei\0%d Dateien"),
        ]);
        assert!(is_mo(&bytes));
        let catalog = Catalog::mo(&bytes)?;
        assert_eq!(
            messages(&catalog),
            vec![
                (
                    "(header)".to_string(),
                    "Content-Type: text/plain; charset=UTF-8\n".to_string()
                ),
                ("Open".to_string(), "Öffnen".to_string()),
                ("menu | File".to_string(), "Datei".to_string()),
                ("%d file".to_string(), "%d Datei\n%d Dateien".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_key_stays_on_one_line() {
        assert_eq!(
            key(&["ctx", "Error:\n\tdetails", ""]),
            "ctx | Error:\\n\\tdetails"
        );
    }

    #[test]
    fn a_qm_is_its_messages_too() -> Result<()> {
        let bytes = qm(&[("MainWindow", "&Open", "&Öffnen"), ("", "Quit", "Beenden")]);
        assert!(is_qm(&bytes));
        let catalog = Catalog::qm(&bytes)?;
        assert_eq!(
            messages(&catalog),
            vec![
                ("MainWindow | &Open".to_string(), "&Öffnen".to_string()),
                ("Quit".to_string(), "Beenden".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    fn a_changed_translation_is_one_changed_member() -> Result<()> {
        use crate::diff::content::{ContentDiff, container::MemberStatus};
        let before = mo(&[("Open", "Öffnen"), ("Save", "Speichern")]);
        let after = mo(&[
            ("Open", "Öffnen …"),
            ("Save", "Speichern"),
            ("Quit", "Beenden"),
        ]);
        let Some(ContentDiff::Container(diff)) = crate::diff::content::diff(&before, &after)?
        else {
            panic!("a catalog pair");
        };
        assert_eq!(diff.unchanged, 1);
        let status = |key: &str| diff.member(key).map(|member| member.status);
        assert_eq!(status("Open"), Some(MemberStatus::Changed));
        assert_eq!(status("Quit"), Some(MemberStatus::Added));
        Ok(())
    }
}
