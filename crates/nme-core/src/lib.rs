//! Format-agnostic metadata model shared by nme's CLI and GUI frontends.
//!
//! [`MetadataFile`] is the one interface both frontends talk to. Adding a
//! new file format means: implement `MetadataFile` for it in a new module,
//! add new `FieldKey` variants if it introduces metadata concepts no
//! existing format has, and add one match arm in [`format::open`]. Nothing
//! else in the CLI or GUI needs to change.

mod error;
pub mod format;
pub mod pdf;

pub use error::{NmeError, Result};

use std::path::Path;

/// A single piece of metadata a file format can carry.
///
/// Fields that several formats share by convention (Title, Author, ...) get
/// one unprefixed variant, so "apply this value to every selected file"
/// works in the GUI even across formats. A field specific to one format
/// (e.g. a future MP3 track number) should be prefixed, e.g. `Mp3Track`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FieldKey {
    Title,
    Author,
    Subject,
    Keywords,
    Creator,
    Producer,
    CreationDate,
    ModDate,
}

impl FieldKey {
    /// Human-readable label for the GUI's left-hand column.
    pub fn label(self) -> &'static str {
        match self {
            FieldKey::Title => "Title",
            FieldKey::Author => "Author",
            FieldKey::Subject => "Subject",
            FieldKey::Keywords => "Keywords",
            FieldKey::Creator => "Creator",
            FieldKey::Producer => "Producer",
            FieldKey::CreationDate => "Creation Date",
            FieldKey::ModDate => "Modification Date",
        }
    }

    /// Stable lowercase identifier for the CLI (`nme get title file.pdf`),
    /// kept separate from `label` so relabeling the GUI never breaks
    /// scripts, and vice versa.
    pub fn cli_name(self) -> &'static str {
        match self {
            FieldKey::Title => "title",
            FieldKey::Author => "author",
            FieldKey::Subject => "subject",
            FieldKey::Keywords => "keywords",
            FieldKey::Creator => "creator",
            FieldKey::Producer => "producer",
            FieldKey::CreationDate => "creation-date",
            FieldKey::ModDate => "mod-date",
        }
    }

    /// Parses a CLI field name back into a `FieldKey`. Kept next to
    /// `cli_name` so the two can't drift out of sync.
    pub fn from_cli_name(name: &str) -> Option<Self> {
        Some(match name {
            "title" => FieldKey::Title,
            "author" => FieldKey::Author,
            "subject" => FieldKey::Subject,
            "keywords" => FieldKey::Keywords,
            "creator" => FieldKey::Creator,
            "producer" => FieldKey::Producer,
            "creation-date" => FieldKey::CreationDate,
            "mod-date" => FieldKey::ModDate,
            _ => return None,
        })
    }

    /// All keys, in canonical display order. Used by the CLI's `list`
    /// command and anywhere else that wants "every field that exists"
    /// rather than "every field one particular file supports"
    /// ([`MetadataFile::fields`] is the latter).
    pub fn all() -> &'static [FieldKey] {
        &[
            FieldKey::Title,
            FieldKey::Author,
            FieldKey::Subject,
            FieldKey::Keywords,
            FieldKey::Creator,
            FieldKey::Producer,
            FieldKey::CreationDate,
            FieldKey::ModDate,
        ]
    }
}

/// Format-agnostic handle to one open file's metadata.
///
/// Implementations own the parsed in-memory representation of the file
/// (e.g. an `lopdf::Document`) and translate between it and the
/// [`FieldKey`]/`String` vocabulary the CLI and GUI both speak.
pub trait MetadataFile {
    /// Path this file was opened from (and saves back to).
    fn path(&self) -> &Path;

    /// Fields this format supports, in the order the GUI should list them.
    fn fields(&self) -> &[FieldKey];

    /// Current value of `key`, or `None` if unset. Callers should only pass
    /// keys returned by [`fields`](Self::fields); others simply return
    /// `None`.
    fn get(&self, key: FieldKey) -> Option<String>;

    /// Sets `key` to `value` in memory (`None` clears the field). Call
    /// [`save`](Self::save) to persist. Setting a key this format doesn't
    /// support is a silent no-op, matching `get`'s contract.
    fn set(&mut self, key: FieldKey, value: Option<String>);

    /// Whether there are unsaved in-memory edits.
    fn is_dirty(&self) -> bool;

    /// Writes in-memory changes back to [`path`](Self::path).
    fn save(&mut self) -> Result<()>;
}
