//! Standalone NBT binary format reader/writer and level.dat editing
//! (ported from `NbtIo.cs`, `LevelDatEditorService.cs`, `LevelDatSummary.cs`).
//!
//! # Binary format
//!
//! This crate implements Minecraft **Bedrock Edition**'s NBT variant, exactly as
//! read/written by the original C# `NbtIo` class (which used `System.IO.BinaryReader`
//! / `BinaryWriter`, both little-endian on all supported platforms):
//!
//! - All multi-byte numeric fields (i16/i32/i64/f32/f64, array/string lengths,
//!   list counts) are **little-endian**. This differs from Java Edition NBT, which
//!   is big-endian.
//! - Strings are length-prefixed with a `u16` (little-endian) byte length, followed
//!   by that many bytes of **plain UTF-8** (the C# code used `Encoding.UTF8`, not
//!   Java's "modified UTF-8").
//! - A named tag is: `u8` type id, then (if type != End) a string name, then the
//!   payload.
//! - `Compound` payload: a sequence of named tags terminated by a tag with type id 0
//!   (`End`).
//! - `List` payload: `u8` element type id, `i32` (LE) element count, then that many
//!   *unnamed* payloads of the element type back-to-back. If the element type is
//!   `End`, no payloads follow (matches the C# `if (elemType != End) WritePayload`
//!   guard).
//! - `ByteArray` / `IntArray` / `LongArray` payload: `i32` (LE) element count,
//!   followed by that many elements (bytes, or i32/i64 LE respectively).
//!
//! ## level.dat wrapping
//!
//! Bedrock's `level.dat` is **not gzip-compressed** (unlike Java Edition's
//! `level.dat`, which is). Per `LevelDatEditorService.cs`, it has an 8-byte header
//! prepended to a single root NBT tag:
//!
//! ```text
//! i32 (LE) save/storage version
//! i32 (LE) length in bytes of the NBT payload that follows
//! <NBT root tag, length bytes>
//! ```
//!
//! `gzip_decompress`/`gzip_compress` are provided as general-purpose helpers (using
//! the newly added `flate2` dependency) for other NBT-bearing files that *are*
//! gzip-wrapped (e.g. Java Edition saves), but they are **not** used by the
//! `level.dat`-specific functions in this crate, since Bedrock's `level.dat` uses the
//! length-prefixed header above instead.

use std::collections::HashMap;
use std::io::{Cursor, Read, Write};

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;

/// Errors that can occur while reading or writing NBT data.
#[derive(Debug, thiserror::Error)]
pub enum NbtError {
    #[error("unexpected end of NBT data while reading {context}")]
    UnexpectedEof { context: &'static str },
    #[error("unknown NBT tag type id {0}")]
    UnknownTagType(u8),
    #[error("invalid UTF-8 string in NBT data: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("negative or otherwise invalid length ({0}) in NBT data")]
    InvalidLength(i64),
    #[error("level.dat header doesn't match the file size — it may be corrupt")]
    InvalidLevelDatHeader,
    #[error("level.dat is too short to contain a header")]
    LevelDatTooShort,
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, NbtError>;

/// The NBT tag type id, matching `NbtType` in `NbtIo.cs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum TagType {
    End = 0,
    Byte = 1,
    Short = 2,
    Int = 3,
    Long = 4,
    Float = 5,
    Double = 6,
    ByteArray = 7,
    String = 8,
    List = 9,
    Compound = 10,
    IntArray = 11,
    LongArray = 12,
}

impl TagType {
    fn from_u8(v: u8) -> Result<Self> {
        Ok(match v {
            0 => TagType::End,
            1 => TagType::Byte,
            2 => TagType::Short,
            3 => TagType::Int,
            4 => TagType::Long,
            5 => TagType::Float,
            6 => TagType::Double,
            7 => TagType::ByteArray,
            8 => TagType::String,
            9 => TagType::List,
            10 => TagType::Compound,
            11 => TagType::IntArray,
            12 => TagType::LongArray,
            other => return Err(NbtError::UnknownTagType(other)),
        })
    }
}

/// A named NBT compound entry: `(name, tag)`.
pub type NamedTag = (String, Tag);

/// An NBT tag value. Mirrors the shape of `NbtTag` in `NbtIo.cs`, but names are
/// carried alongside values inside `Compound` rather than on the tag itself, which
/// is more idiomatic in Rust and avoids an "unnamed root/list-item" special case.
#[derive(Debug, Clone, PartialEq)]
pub enum Tag {
    Byte(u8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    ByteArray(Vec<u8>),
    String(String),
    /// Homogeneous list. `element_type` is preserved even for empty lists so a
    /// round-trip write reproduces the exact original bytes (the C# code writes
    /// `ListElementType` regardless of item count, and an empty list is commonly
    /// written with element type `End`).
    List {
        element_type: TagType,
        items: Vec<Tag>,
    },
    Compound(Vec<NamedTag>),
    IntArray(Vec<i32>),
    LongArray(Vec<i64>),
}

impl Tag {
    /// The `TagType` id for this tag's variant.
    pub fn tag_type(&self) -> TagType {
        match self {
            Tag::Byte(_) => TagType::Byte,
            Tag::Short(_) => TagType::Short,
            Tag::Int(_) => TagType::Int,
            Tag::Long(_) => TagType::Long,
            Tag::Float(_) => TagType::Float,
            Tag::Double(_) => TagType::Double,
            Tag::ByteArray(_) => TagType::ByteArray,
            Tag::String(_) => TagType::String,
            Tag::List { .. } => TagType::List,
            Tag::Compound(_) => TagType::Compound,
            Tag::IntArray(_) => TagType::IntArray,
            Tag::LongArray(_) => TagType::LongArray,
        }
    }

    /// Find a direct child of a `Compound` tag by name (equivalent to `NbtIo.Find`).
    /// Returns `None` if `self` is not a `Compound` or the name isn't present.
    pub fn find(&self, name: &str) -> Option<&Tag> {
        match self {
            Tag::Compound(items) => items.iter().find(|(n, _)| n == name).map(|(_, t)| t),
            _ => None,
        }
    }

    /// Set an existing child's value (replacing it entirely) or append a new child,
    /// if `self` is a `Compound` (equivalent to `NbtIo.SetOrAdd`). No-op otherwise.
    pub fn set_or_add(&mut self, name: &str, value: Tag) {
        if let Tag::Compound(items) = self {
            if let Some((_, existing)) = items.iter_mut().find(|(n, _)| n == name) {
                *existing = value;
            } else {
                items.push((name.to_string(), value));
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Raw reader/writer
// ---------------------------------------------------------------------------

struct Reader<'a> {
    cursor: Cursor<&'a [u8]>,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            cursor: Cursor::new(data),
        }
    }

    fn read_u8(&mut self) -> Result<u8> {
        let mut buf = [0u8; 1];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "u8" })?;
        Ok(buf[0])
    }

    fn read_i16(&mut self) -> Result<i16> {
        let mut buf = [0u8; 2];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "i16" })?;
        Ok(i16::from_le_bytes(buf))
    }

    fn read_u16(&mut self) -> Result<u16> {
        let mut buf = [0u8; 2];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "u16" })?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let mut buf = [0u8; 4];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "i32" })?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i64(&mut self) -> Result<i64> {
        let mut buf = [0u8; 8];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "i64" })?;
        Ok(i64::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> Result<f32> {
        let mut buf = [0u8; 4];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "f32" })?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_f64(&mut self) -> Result<f64> {
        let mut buf = [0u8; 8];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof { context: "f64" })?;
        Ok(f64::from_le_bytes(buf))
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        self.cursor
            .read_exact(&mut buf)
            .map_err(|_| NbtError::UnexpectedEof {
                context: "byte buffer",
            })?;
        Ok(buf)
    }

    fn read_len(&mut self) -> Result<usize> {
        let len = self.read_i32()?;
        if len < 0 {
            return Err(NbtError::InvalidLength(len as i64));
        }
        Ok(len as usize)
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u16()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8(bytes)?)
    }

    fn read_payload(&mut self, tag_type: TagType) -> Result<Tag> {
        Ok(match tag_type {
            TagType::Byte => Tag::Byte(self.read_u8()?),
            TagType::Short => Tag::Short(self.read_i16()?),
            TagType::Int => Tag::Int(self.read_i32()?),
            TagType::Long => Tag::Long(self.read_i64()?),
            TagType::Float => Tag::Float(self.read_f32()?),
            TagType::Double => Tag::Double(self.read_f64()?),
            TagType::ByteArray => {
                let len = self.read_len()?;
                Tag::ByteArray(self.read_bytes(len)?)
            }
            TagType::String => Tag::String(self.read_string()?),
            TagType::List => {
                let element_type = TagType::from_u8(self.read_u8()?)?;
                let count = self.read_len()?;
                let mut items = Vec::with_capacity(count.min(1 << 16));
                for _ in 0..count {
                    if element_type == TagType::End {
                        // Matches C#: `if (elemType != NbtType.End) ReadPayload(...)`.
                        // An End-typed item carries no payload; represent it as an
                        // empty compound-less placeholder is not possible, so we
                        // simply do not read anything (there is nothing to store).
                        continue;
                    }
                    items.push(self.read_payload(element_type)?);
                }
                Tag::List {
                    element_type,
                    items,
                }
            }
            TagType::Compound => {
                let mut items = Vec::new();
                loop {
                    let child_type = TagType::from_u8(self.read_u8()?)?;
                    if child_type == TagType::End {
                        break;
                    }
                    let name = self.read_string()?;
                    let value = self.read_payload(child_type)?;
                    items.push((name, value));
                }
                Tag::Compound(items)
            }
            TagType::IntArray => {
                let len = self.read_len()?;
                let mut arr = Vec::with_capacity(len.min(1 << 16));
                for _ in 0..len {
                    arr.push(self.read_i32()?);
                }
                Tag::IntArray(arr)
            }
            TagType::LongArray => {
                let len = self.read_len()?;
                let mut arr = Vec::with_capacity(len.min(1 << 16));
                for _ in 0..len {
                    arr.push(self.read_i64()?);
                }
                Tag::LongArray(arr)
            }
            TagType::End => {
                return Err(NbtError::UnknownTagType(0));
            }
        })
    }
}

struct Writer {
    buf: Vec<u8>,
}

impl Writer {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }

    fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    fn write_i16(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_u16(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i32(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_i64(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_f32(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_f64(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    fn write_string(&mut self, s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if bytes.len() > u16::MAX as usize {
            return Err(NbtError::InvalidLength(bytes.len() as i64));
        }
        self.write_u16(bytes.len() as u16);
        self.buf.extend_from_slice(bytes);
        Ok(())
    }

    fn write_payload(&mut self, tag: &Tag) -> Result<()> {
        match tag {
            Tag::Byte(v) => self.write_u8(*v),
            Tag::Short(v) => self.write_i16(*v),
            Tag::Int(v) => self.write_i32(*v),
            Tag::Long(v) => self.write_i64(*v),
            Tag::Float(v) => self.write_f32(*v),
            Tag::Double(v) => self.write_f64(*v),
            Tag::ByteArray(v) => {
                self.write_i32(v.len() as i32);
                self.buf.extend_from_slice(v);
            }
            Tag::String(s) => self.write_string(s)?,
            Tag::List {
                element_type,
                items,
            } => {
                self.write_u8(*element_type as u8);
                self.write_i32(items.len() as i32);
                for item in items {
                    if *element_type != TagType::End {
                        self.write_payload(item)?;
                    }
                }
            }
            Tag::Compound(items) => {
                for (name, value) in items {
                    self.write_u8(value.tag_type() as u8);
                    self.write_string(name)?;
                    self.write_payload(value)?;
                }
                self.write_u8(TagType::End as u8);
            }
            Tag::IntArray(v) => {
                self.write_i32(v.len() as i32);
                for x in v {
                    self.write_i32(*x);
                }
            }
            Tag::LongArray(v) => {
                self.write_i32(v.len() as i32);
                for x in v {
                    self.write_i64(*x);
                }
            }
        }
        Ok(())
    }
}

/// Read a root NBT tag (type byte + name + payload) from a raw buffer, returning
/// the root's name and value. Equivalent to `NbtIo.ReadRootTag`.
pub fn read_root_tag(data: &[u8]) -> Result<(String, Tag)> {
    let mut r = Reader::new(data);
    let tag_type = TagType::from_u8(r.read_u8()?)?;
    if tag_type == TagType::End {
        return Ok((String::new(), Tag::Compound(Vec::new())));
    }
    let name = r.read_string()?;
    let value = r.read_payload(tag_type)?;
    Ok((name, value))
}

/// Write a root NBT tag (type byte + name + payload) to a new buffer. Equivalent to
/// `NbtIo.WriteRootTag`.
pub fn write_root_tag(name: &str, tag: &Tag) -> Result<Vec<u8>> {
    let mut w = Writer::new();
    w.write_u8(tag.tag_type() as u8);
    w.write_string(name)?;
    w.write_payload(tag)?;
    Ok(w.buf)
}

// ---------------------------------------------------------------------------
// gzip helpers (for gzip-wrapped NBT such as Java Edition saves; NOT used by
// Bedrock's level.dat, which uses a length-prefixed header instead — see below)
// ---------------------------------------------------------------------------

/// Decompress a gzip-wrapped buffer.
pub fn gzip_decompress(data: &[u8]) -> Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

/// Gzip-compress a buffer at the default compression level.
pub fn gzip_compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data)?;
    Ok(encoder.finish()?)
}

// ---------------------------------------------------------------------------
// level.dat (Bedrock Edition)
// ---------------------------------------------------------------------------

const LEVEL_DAT_HEADER_SIZE: usize = 8;
const LEVEL_DAT_SAVE_VERSION: i32 = 10;

/// The parsed contents of a Bedrock `level.dat`'s 8-byte header plus its NBT root.
pub struct LevelDatFile {
    pub version: i32,
    pub root_name: String,
    pub root: Tag,
}

/// Parse a raw `level.dat` file buffer: an 8-byte header (`i32` version, `i32`
/// payload length, both little-endian) followed by exactly that many bytes of a
/// single root NBT tag. Equivalent to the read side of `LevelDatEditorService.cs`.
pub fn read_level_dat(data: &[u8]) -> Result<LevelDatFile> {
    if data.len() < LEVEL_DAT_HEADER_SIZE {
        return Err(NbtError::LevelDatTooShort);
    }
    let version = i32::from_le_bytes(data[0..4].try_into().expect("checked len"));
    let length = i32::from_le_bytes(data[4..8].try_into().expect("checked len"));
    if length < 0 || LEVEL_DAT_HEADER_SIZE + length as usize > data.len() {
        return Err(NbtError::InvalidLevelDatHeader);
    }
    let payload = &data[LEVEL_DAT_HEADER_SIZE..LEVEL_DAT_HEADER_SIZE + length as usize];
    let (root_name, root) = read_root_tag(payload)?;
    Ok(LevelDatFile {
        version,
        root_name,
        root,
    })
}

/// Serialize a `level.dat` buffer: 8-byte header followed by the NBT root tag.
/// `version` should normally be the version read from the original file (falling
/// back to `LEVEL_DAT_SAVE_VERSION` when it was `<= 0`, matching the C# behavior).
pub fn write_level_dat(version: i32, root_name: &str, root: &Tag) -> Result<Vec<u8>> {
    let payload = write_root_tag(root_name, root)?;
    let effective_version = if version > 0 {
        version
    } else {
        LEVEL_DAT_SAVE_VERSION
    };
    let mut out = Vec::with_capacity(LEVEL_DAT_HEADER_SIZE + payload.len());
    out.extend_from_slice(&effective_version.to_le_bytes());
    out.extend_from_slice(&(payload.len() as i32).to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

// ---------------------------------------------------------------------------
// LevelDatSummary (ported from Models/LevelDatSummary.cs + the field mapping in
// LevelDatEditorService.cs's Load/Save methods)
// ---------------------------------------------------------------------------

/// The summary/edit-facing shape of a Bedrock world's `level.dat`, ported from
/// `LevelDatSummary.cs`. `world_path` from the original C# model is intentionally
/// omitted here — path/IO concerns belong to the caller, not this pure-data crate.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LevelDatSummary {
    pub game_type: i32,
    pub difficulty: i32,
    pub cheats_enabled: bool,
    pub generator: i32,
    pub random_seed: i64,
    pub has_random_seed: bool,
    pub experiments: HashMap<String, bool>,
}

fn find_int(root: &Tag, name: &str, fallback: i32) -> i32 {
    match root.find(name) {
        Some(Tag::Int(i)) => *i,
        _ => fallback,
    }
}

fn find_byte(root: &Tag, name: &str) -> u8 {
    match root.find(name) {
        Some(Tag::Byte(b)) => *b,
        _ => 0,
    }
}

/// Extract a `LevelDatSummary` from a parsed level.dat root tag. Equivalent to
/// `LevelDatEditorService.Load`'s field-mapping logic (the file I/O itself is left
/// to the caller — see `read_level_dat`).
pub fn summary_from_root(root: &Tag) -> LevelDatSummary {
    let mut summary = LevelDatSummary {
        game_type: find_int(root, "GameType", 0),
        difficulty: find_int(root, "Difficulty", 2),
        cheats_enabled: find_byte(root, "commandsEnabled") != 0,
        generator: find_int(root, "Generator", 1),
        random_seed: 0,
        has_random_seed: false,
        experiments: HashMap::new(),
    };

    if let Some(seed_tag) = root.find("RandomSeed") {
        summary.has_random_seed = true;
        summary.random_seed = match seed_tag {
            Tag::Long(v) => *v,
            Tag::Int(v) => *v as i64,
            _ => 0,
        };
    }

    if let Some(Tag::Compound(items)) = root.find("experiments") {
        for (name, value) in items {
            if let Tag::Byte(b) = value {
                summary.experiments.insert(name.clone(), *b != 0);
            }
        }
    }

    summary
}

/// Apply a `LevelDatSummary`'s editable fields onto an existing level.dat root tag,
/// mutating it in place. Equivalent to the field-mapping logic in
/// `LevelDatEditorService.Save` (the file I/O itself is left to the caller — see
/// `write_level_dat`).
pub fn apply_summary_to_root(root: &mut Tag, summary: &LevelDatSummary) {
    root.set_or_add("GameType", Tag::Int(summary.game_type));
    root.set_or_add("Difficulty", Tag::Int(summary.difficulty));
    root.set_or_add(
        "commandsEnabled",
        Tag::Byte(if summary.cheats_enabled { 1 } else { 0 }),
    );
    root.set_or_add("Generator", Tag::Int(summary.generator));

    if !summary.experiments.is_empty() {
        if root.find("experiments").is_none() {
            root.set_or_add("experiments", Tag::Compound(Vec::new()));
        }
        if let Tag::Compound(items) = root {
            if let Some((_, experiments_tag)) = items.iter_mut().find(|(n, _)| n == "experiments") {
                for (name, enabled) in &summary.experiments {
                    experiments_tag.set_or_add(name, Tag::Byte(if *enabled { 1 } else { 0 }));
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::approx_constant)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_mixed_compound() {
        let root = Tag::Compound(vec![
            ("aByte".to_string(), Tag::Byte(200)),
            ("aShort".to_string(), Tag::Short(-1234)),
            ("anInt".to_string(), Tag::Int(-2_000_000_000)),
            ("aLong".to_string(), Tag::Long(9_223_372_036_854_775_807)),
            ("aFloat".to_string(), Tag::Float(3.14159)),
            ("aDouble".to_string(), Tag::Double(2.718281828459045)),
            (
                "aByteArray".to_string(),
                Tag::ByteArray(vec![0, 1, 2, 255, 128, 64]),
            ),
            (
                "aString".to_string(),
                Tag::String("hello, world \u{1F600}".to_string()),
            ),
            ("anEmptyString".to_string(), Tag::String(String::new())),
            (
                "anIntArray".to_string(),
                Tag::IntArray(vec![1, -1, 0, i32::MAX, i32::MIN]),
            ),
            (
                "aLongArray".to_string(),
                Tag::LongArray(vec![1, -1, 0, i64::MAX, i64::MIN]),
            ),
            (
                "aListOfInts".to_string(),
                Tag::List {
                    element_type: TagType::Int,
                    items: vec![Tag::Int(1), Tag::Int(2), Tag::Int(3)],
                },
            ),
            (
                "anEmptyList".to_string(),
                Tag::List {
                    element_type: TagType::End,
                    items: vec![],
                },
            ),
            (
                "aNestedCompound".to_string(),
                Tag::Compound(vec![
                    ("inner1".to_string(), Tag::Byte(7)),
                    (
                        "innerList".to_string(),
                        Tag::List {
                            element_type: TagType::Compound,
                            items: vec![
                                Tag::Compound(vec![("x".to_string(), Tag::Int(1))]),
                                Tag::Compound(vec![("x".to_string(), Tag::Int(2))]),
                            ],
                        },
                    ),
                ]),
            ),
        ]);

        let bytes = write_root_tag("MyRoot", &root).expect("write should succeed");
        let (name, decoded) = read_root_tag(&bytes).expect("read should succeed");

        assert_eq!(name, "MyRoot");
        assert_eq!(decoded, root);
    }

    #[test]
    fn string_uses_le_u16_length_and_plain_utf8() {
        let bytes = write_root_tag("n", &Tag::String("abc".to_string())).unwrap();
        // type byte (String=8), name len (u16 LE = 1), name "n",
        // string payload len (u16 LE = 3), "abc"
        assert_eq!(bytes[0], 8);
        assert_eq!(&bytes[1..3], &1u16.to_le_bytes());
        assert_eq!(bytes[3], b'n');
        assert_eq!(&bytes[4..6], &3u16.to_le_bytes());
        assert_eq!(&bytes[6..9], b"abc");
    }

    #[test]
    fn numbers_are_little_endian() {
        let bytes = write_root_tag("n", &Tag::Int(1)).unwrap();
        // type(1) + name-len(2) + name(1) + payload(4)
        let payload = &bytes[bytes.len() - 4..];
        assert_eq!(payload, &1i32.to_le_bytes());
    }

    #[test]
    fn truncated_data_errors_instead_of_panicking() {
        let bytes = write_root_tag("n", &Tag::Int(42)).unwrap();
        for cut in 0..bytes.len() {
            let result = read_root_tag(&bytes[..cut]);
            assert!(result.is_err(), "expected error when truncated at {cut}");
        }
    }

    #[test]
    fn unknown_tag_type_errors() {
        let data = [99u8, 0, 0]; // bogus type id, empty name
        assert!(matches!(
            read_root_tag(&data),
            Err(NbtError::UnknownTagType(99))
        ));
    }

    #[test]
    fn level_dat_round_trip_with_header() {
        let root = Tag::Compound(vec![
            ("GameType".to_string(), Tag::Int(0)),
            ("Difficulty".to_string(), Tag::Int(2)),
            ("commandsEnabled".to_string(), Tag::Byte(1)),
            ("Generator".to_string(), Tag::Int(1)),
            ("RandomSeed".to_string(), Tag::Long(123456789)),
        ]);
        let bytes = write_level_dat(10, "", &root).expect("write_level_dat");
        let parsed = read_level_dat(&bytes).expect("read_level_dat");
        assert_eq!(parsed.version, 10);
        assert_eq!(parsed.root, root);
    }

    #[test]
    fn level_dat_rejects_bad_header() {
        // length says 1000 bytes follow but buffer is tiny.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&10i32.to_le_bytes());
        bytes.extend_from_slice(&1000i32.to_le_bytes());
        bytes.extend_from_slice(&[0u8; 4]);
        assert!(matches!(
            read_level_dat(&bytes),
            Err(NbtError::InvalidLevelDatHeader)
        ));
    }

    #[test]
    fn level_dat_too_short_errors() {
        assert!(matches!(
            read_level_dat(&[1, 2, 3]),
            Err(NbtError::LevelDatTooShort)
        ));
    }

    #[test]
    fn summary_round_trip_via_level_dat_root() {
        let root = Tag::Compound(vec![
            ("GameType".to_string(), Tag::Int(1)),
            ("Difficulty".to_string(), Tag::Int(3)),
            ("commandsEnabled".to_string(), Tag::Byte(1)),
            ("Generator".to_string(), Tag::Int(2)),
            ("RandomSeed".to_string(), Tag::Long(42)),
            (
                "experiments".to_string(),
                Tag::Compound(vec![
                    ("data_driven_items".to_string(), Tag::Byte(1)),
                    ("upcoming_creator_features".to_string(), Tag::Byte(0)),
                ]),
            ),
        ]);

        let summary = summary_from_root(&root);
        assert_eq!(summary.game_type, 1);
        assert_eq!(summary.difficulty, 3);
        assert!(summary.cheats_enabled);
        assert_eq!(summary.generator, 2);
        assert!(summary.has_random_seed);
        assert_eq!(summary.random_seed, 42);
        assert_eq!(summary.experiments.get("data_driven_items"), Some(&true));
        assert_eq!(
            summary.experiments.get("upcoming_creator_features"),
            Some(&false)
        );

        let mut modified_root = root.clone();
        let mut new_summary = summary.clone();
        new_summary.game_type = 0;
        new_summary.cheats_enabled = false;
        new_summary.experiments.insert("new_flag".to_string(), true);
        apply_summary_to_root(&mut modified_root, &new_summary);

        let reread = summary_from_root(&modified_root);
        assert_eq!(reread.game_type, 0);
        assert!(!reread.cheats_enabled);
        assert_eq!(reread.experiments.get("new_flag"), Some(&true));
    }

    #[test]
    fn gzip_helpers_round_trip() {
        let original = write_root_tag("g", &Tag::Int(7)).unwrap();
        let compressed = gzip_compress(&original).expect("compress");
        let decompressed = gzip_decompress(&compressed).expect("decompress");
        assert_eq!(decompressed, original);
    }
}
