//! MP3 metadata backend, built on `id3` (MIT licensed, pure Rust).
//!
//! Reads/writes the file's ID3v2 tag. If a file has no tag at all,
//! `id3::Tag::read_from_path` reports `ErrorKind::NoTag`, which we treat as
//! "an empty, unpopulated tag" rather than an error — that matches how
//! `PdfFile` treats a missing `/Info` dict, and means `nme info` on an
//! untagged MP3 just shows every field as unset instead of failing.
//!
//! `Title`/`Author` are shared with the PDF backend (mapped to ID3's TIT2
//! "title" and TPE1 "artist" respectively, which are the same concepts by a
//! different name). Album/Album Artist/Track/Year/Genre have no PDF
//! equivalent, so they're `Mp3*`-prefixed fields specific to this backend.

use std::path::{Path, PathBuf};

use id3::{Tag, TagLike, Version};

use crate::{FieldKey, MetadataFile, NmeError, Result};

/// All fields the MP3 backend supports, in GUI display order.
const MP3_FIELDS: &[FieldKey] = &[
    FieldKey::Title,
    FieldKey::Author,
    FieldKey::Mp3Album,
    FieldKey::Mp3AlbumArtist,
    FieldKey::Mp3Track,
    FieldKey::Mp3Year,
    FieldKey::Mp3Genre,
];

pub struct Mp3File {
    path: PathBuf,
    tag: Tag,
    dirty: bool,
}

impl Mp3File {
    pub fn open(path: &Path) -> Result<Self> {
        let tag = match Tag::read_from_path(path) {
            Ok(tag) => tag,
            Err(id3::Error {
                kind: id3::ErrorKind::NoTag,
                ..
            }) => Tag::new(),
            Err(source) => {
                return Err(NmeError::Mp3Parse {
                    path: path.to_path_buf(),
                    source,
                })
            }
        };
        Ok(Self {
            path: path.to_path_buf(),
            tag,
            dirty: false,
        })
    }
}

impl MetadataFile for Mp3File {
    fn path(&self) -> &Path {
        &self.path
    }

    fn fields(&self) -> &[FieldKey] {
        MP3_FIELDS
    }

    fn get(&self, key: FieldKey) -> Option<String> {
        match key {
            FieldKey::Title => self.tag.title().map(str::to_owned),
            FieldKey::Author => self.tag.artist().map(str::to_owned),
            FieldKey::Mp3Album => self.tag.album().map(str::to_owned),
            FieldKey::Mp3AlbumArtist => self.tag.album_artist().map(str::to_owned),
            FieldKey::Mp3Track => self.tag.track().map(|n| n.to_string()),
            FieldKey::Mp3Year => self.tag.year().map(|y| y.to_string()),
            FieldKey::Mp3Genre => self.tag.genre().map(str::to_owned),
            _ => None, // unsupported field for this format
        }
    }

    fn set(&mut self, key: FieldKey, value: Option<String>) {
        match key {
            FieldKey::Title => match value {
                Some(v) => self.tag.set_title(v),
                None => self.tag.remove_title(),
            },
            FieldKey::Author => match value {
                Some(v) => self.tag.set_artist(v),
                None => self.tag.remove_artist(),
            },
            FieldKey::Mp3Album => match value {
                Some(v) => self.tag.set_album(v),
                None => self.tag.remove_album(),
            },
            FieldKey::Mp3AlbumArtist => match value {
                Some(v) => self.tag.set_album_artist(v),
                None => self.tag.remove_album_artist(),
            },
            FieldKey::Mp3Track => match value {
                // Track/Year are numeric in ID3; a value that doesn't parse
                // is silently dropped rather than stored as garbage. This
                // only matters once a `set` CLI command exists — `info` is
                // read-only and never hits this branch.
                Some(v) => {
                    if let Ok(n) = v.parse::<u32>() {
                        self.tag.set_track(n);
                    }
                }
                None => self.tag.remove_track(),
            },
            FieldKey::Mp3Year => match value {
                Some(v) => {
                    if let Ok(y) = v.parse::<i32>() {
                        self.tag.set_year(y);
                    }
                }
                None => self.tag.remove_year(),
            },
            FieldKey::Mp3Genre => match value {
                Some(v) => self.tag.set_genre(v),
                None => self.tag.remove_genre(),
            },
            _ => return, // unsupported field for this format: ignore, per trait contract
        }
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn save(&mut self) -> Result<()> {
        self.tag
            .write_to_path(&self.path, Version::Id3v24)
            .map_err(|source| NmeError::Mp3Save {
                path: self.path.clone(),
                source,
            })?;
        self.dirty = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal file with a valid (empty) ID3v2.4 tag at a temp
    /// path, mirroring `pdf::tests::minimal_pdf`: no fixture files
    /// committed to the repo, nothing that needs to exist beforehand.
    fn minimal_mp3(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        // id3 prepends the tag to existing file bytes, so the file needs
        // to exist first, even if (as here) it holds no actual audio data.
        std::fs::write(&path, []).expect("create empty fixture file");
        Tag::new()
            .write_to_path(&path, Version::Id3v24)
            .expect("write empty ID3v2.4 tag");
        path
    }

    #[test]
    fn set_then_save_then_reopen_round_trips_title_and_artist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_mp3(&dir, "roundtrip.mp3");

        let mut mp3 = Mp3File::open(&path).expect("open");
        assert_eq!(mp3.get(FieldKey::Title), None);

        mp3.set(FieldKey::Title, Some("Hello".to_string()));
        mp3.set(FieldKey::Author, Some("Takashi".to_string()));
        assert!(mp3.is_dirty());
        mp3.save().expect("save");
        assert!(!mp3.is_dirty());

        let reopened = Mp3File::open(&path).expect("reopen");
        assert_eq!(reopened.get(FieldKey::Title), Some("Hello".to_string()));
        assert_eq!(reopened.get(FieldKey::Author), Some("Takashi".to_string()));
    }

    #[test]
    fn numeric_fields_round_trip_as_strings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_mp3(&dir, "numeric.mp3");

        let mut mp3 = Mp3File::open(&path).expect("open");
        mp3.set(FieldKey::Mp3Track, Some("7".to_string()));
        mp3.set(FieldKey::Mp3Year, Some("1998".to_string()));
        mp3.save().expect("save");

        let reopened = Mp3File::open(&path).expect("reopen");
        assert_eq!(reopened.get(FieldKey::Mp3Track), Some("7".to_string()));
        assert_eq!(reopened.get(FieldKey::Mp3Year), Some("1998".to_string()));
    }

    #[test]
    fn invalid_numeric_value_is_silently_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_mp3(&dir, "invalid_numeric.mp3");

        let mut mp3 = Mp3File::open(&path).expect("open");
        mp3.set(FieldKey::Mp3Track, Some("not-a-number".to_string()));
        assert_eq!(mp3.get(FieldKey::Mp3Track), None);
    }

    #[test]
    fn untagged_file_reads_as_all_unset_rather_than_erroring() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("untagged.mp3");
        std::fs::write(&path, []).expect("create empty fixture file");

        let mp3 = Mp3File::open(&path).expect("open with no ID3 tag at all");
        for &key in mp3.fields() {
            assert_eq!(mp3.get(key), None);
        }
    }

    #[test]
    fn fields_lists_all_mp3_keys_in_display_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_mp3(&dir, "fields.mp3");
        let mp3 = Mp3File::open(&path).expect("open");

        assert_eq!(mp3.fields(), MP3_FIELDS);
    }
}
