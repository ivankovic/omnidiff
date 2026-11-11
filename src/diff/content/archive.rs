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

//! Archives as containers: a zip (and everything that is one: a jar, an Office document, an
//! EPUB) or a tar, its files the members, keyed by path; a gzip, xz or bzip2 stream as the tar
//! inside it, or as one member when it compresses a single file.
//!
//! A zip's members are hashed by the CRC and size its directory already records, so listing one
//! decompresses nothing, and the directory is read once: opening a member clones the parsed
//! archive, which shares it. A tar or a compressed stream has no directory: it is expanded once,
//! up to [`MAX_EXPANDED_BYTES`]. Whatever is decompressed is charged to the diff's budget
//! ([`super::charge_expanded`]) as it comes.

use std::io::{Cursor, Read};
use std::sync::Arc;

use anyhow::{Context, Result, bail};

use super::container::{
    Container, ContainerInfo, Loaded, MAX_EXPANDED_BYTES, Member, MemberContent,
};
use super::{Format, charge_expanded};

/// The key of the one member a compressed single file has: the stream does not say what the file
/// was called (gzip may, but then a renamed file would read as removed and added).
pub const SINGLE_FILE: &str = "(content)";

/// True if `bytes` are a tar: the `ustar` magic every POSIX tar header carries.
pub fn is_tar(bytes: &[u8]) -> bool {
    bytes.get(257..262) == Some(b"ustar")
}

/// Decodes an archive `format` names.
pub fn decode(bytes: &[u8], format: Format) -> Result<Box<dyn Container>> {
    Ok(match format {
        Format::Zip => Box::new(Zip::new(bytes)?),
        Format::Tar => Box::new(tar(bytes, "TAR", bytes.len())?),
        Format::Gzip => expanded(
            flate2::read::MultiGzDecoder::new(bytes),
            "GZIP",
            bytes.len(),
        )?,
        Format::Xz => expanded(lzma_rust2::XzReader::new(bytes, true), "XZ", bytes.len())?,
        Format::Bzip2 => expanded(
            bzip2::read::MultiBzDecoder::new(bytes),
            "BZIP2",
            bytes.len(),
        )?,
        other => bail!("{} is not an archive", other.name()),
    })
}

/// A compressed stream, expanded: the tar inside it, or a single file.
fn expanded(reader: impl Read, format: &str, size: usize) -> Result<Box<dyn Container>> {
    let inner = read_capped(reader)?;
    Ok(if is_tar(&inner) {
        Box::new(tar(&inner, &format!("TAR.{format}"), size)?)
    } else {
        Box::new(Loaded::new(
            format,
            size,
            vec![(SINGLE_FILE.to_string(), inner)],
        ))
    })
}

/// How much of a stream is read before it is charged to the diff's budget.
const CHARGE_CHUNK: u64 = 1024 * 1024;

/// All of `reader`, unless it runs past [`MAX_EXPANDED_BYTES`] or the diff's budget.
fn read_capped(mut reader: impl Read) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    loop {
        let read = (&mut reader)
            .take(CHARGE_CHUNK)
            .read_to_end(&mut bytes)
            .context("decompressing")?;
        if read == 0 {
            return Ok(bytes);
        }
        charge_expanded(read as u64)?;
        if bytes.len() as u64 > MAX_EXPANDED_BYTES {
            bail!("expands past {MAX_EXPANDED_BYTES} bytes");
        }
    }
}

/// A tar's regular files, in their order.
fn tar(bytes: &[u8], format: &str, size: usize) -> Result<Loaded> {
    let mut archive = tar::Archive::new(bytes);
    let mut files = Vec::new();
    let mut total = 0u64;
    for entry in archive.entries().context("reading the tar")? {
        let mut entry = entry.context("reading a tar entry")?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let key = entry.path()?.to_string_lossy().into_owned();
        total += entry.header().size()?;
        if total > MAX_EXPANDED_BYTES {
            bail!("expands past {MAX_EXPANDED_BYTES} bytes");
        }
        let mut contents = Vec::new();
        entry.read_to_end(&mut contents)?;
        files.push((key, contents));
    }
    Ok(Loaded::new(format, size, files))
}

/// A zip, read from its directory; members are decompressed one at a time, when opened.
pub struct Zip {
    info: ContainerInfo,
    members: Vec<Member>,
    /// Each member's index in the zip's directory.
    indices: Vec<usize>,
    /// The parsed directory, over the zip's bytes: a clone shares both.
    archive: zip::ZipArchive<Cursor<Arc<[u8]>>>,
}

impl Zip {
    pub fn new(bytes: &[u8]) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(Cursor::new(Arc::<[u8]>::from(bytes)))
            .context("reading the zip's directory")?;
        let (mut members, mut indices) = (Vec::new(), Vec::new());
        let mut total = 0u64;
        for index in 0..archive.len() {
            let file = archive.by_index_raw(index).context("reading the zip")?;
            if file.is_dir() {
                continue;
            }
            total += file.size();
            if total > MAX_EXPANDED_BYTES {
                bail!("expands past {MAX_EXPANDED_BYTES} bytes");
            }
            members.push(Member {
                key: file.name().context("reading a member's name")?.into_owned(),
                hash: u64::from(file.crc32()) << 32 ^ file.size(),
            });
            indices.push(index);
        }
        Ok(Self {
            info: ContainerInfo {
                format: "ZIP".to_string(),
                bytes: bytes.len(),
                members: members.len(),
            },
            members,
            indices,
            archive,
        })
    }

    /// The bytes of member `index`.
    pub fn read(&self, index: usize) -> Result<Vec<u8>> {
        let mut archive = self.archive.clone();
        let file = archive.by_index(self.indices[index])?;
        read_capped(file)
    }

    /// The index of the member called `key`.
    pub fn find(&self, key: &str) -> Option<usize> {
        self.members.iter().position(|member| member.key == key)
    }
}

impl Container for Zip {
    fn info(&self) -> &ContainerInfo {
        &self.info
    }

    fn members(&self) -> &[Member] {
        &self.members
    }

    fn open(&self, index: usize) -> Result<MemberContent> {
        Ok(MemberContent::Bytes(self.read(index)?))
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// A zip of `files`, deflated.
    pub(crate) fn zip(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut bytes));
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, contents) in files {
                writer.start_file(*name, options).expect("starts");
                writer.write_all(contents).expect("writes");
            }
            writer.finish().expect("finishes");
        }
        bytes
    }

    /// A tar of `files`.
    pub(crate) fn tar_of(files: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, contents) in files {
            let mut header = tar::Header::new_gnu();
            header.set_size(contents.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            builder
                .append_data(&mut header, name, *contents)
                .expect("appends");
        }
        builder.into_inner().expect("finishes")
    }

    fn keys(container: &dyn Container) -> Vec<&str> {
        container
            .members()
            .iter()
            .map(|member| member.key.as_str())
            .collect()
    }

    #[test]
    fn a_zip_lists_its_files_and_opens_one() -> Result<()> {
        let bytes = zip(&[("a.txt", b"one\n"), ("dir/b.txt", b"two\n")]);
        let container = decode(&bytes, Format::Zip)?;
        assert_eq!(keys(container.as_ref()), vec!["a.txt", "dir/b.txt"]);
        let MemberContent::Bytes(opened) = container.open(1)? else {
            panic!("a zip member is bytes");
        };
        assert_eq!(opened, b"two\n");
        Ok(())
    }

    #[test]
    fn a_gzipped_tar_is_the_tar_and_a_gzipped_file_is_one_member() -> Result<()> {
        let tar = tar_of(&[("x/y.txt", b"hello\n")]);
        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(&tar)?;
        let container = decode(&gz.finish()?, Format::Gzip)?;
        assert_eq!(container.info().format, "TAR.GZIP");
        assert_eq!(keys(container.as_ref()), vec!["x/y.txt"]);

        let mut gz = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
        gz.write_all(b"just text\n")?;
        let container = decode(&gz.finish()?, Format::Gzip)?;
        assert_eq!(keys(container.as_ref()), vec![SINGLE_FILE]);
        Ok(())
    }

    #[test]
    fn xz_and_bzip2_expand() -> Result<()> {
        let mut bz = bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::fast());
        bz.write_all(b"bz\n")?;
        let container = decode(&bz.finish()?, Format::Bzip2)?;
        let MemberContent::Bytes(opened) = container.open(0)? else {
            panic!("bytes");
        };
        assert_eq!(opened, b"bz\n");

        let mut xz = lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::default())?;
        xz.write_all(b"xz\n")?;
        let container = decode(&xz.finish()?, Format::Xz)?;
        let MemberContent::Bytes(opened) = container.open(0)? else {
            panic!("bytes");
        };
        assert_eq!(opened, b"xz\n");
        Ok(())
    }
}
