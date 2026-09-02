//! FLAC metadata backend, built on `metaflac` (MIT licensed, pure Rust).
//!
//! Reads/writes the file's Vorbis comment block. A valid FLAC with no
//! comment block at all is treated as "an empty, unpopulated tag" rather
//! than an error — that matches how `Mp3File` treats a missing ID3 tag
//! and how `PdfFile` treats a missing `/Info` dict, and means `nme info`
//! on an untagged FLAC just shows every field as unset instead of failing.
//!
//! `Title`/`Author` are shared with the PDF and MP3 backends (mapped to
//! Vorbis `TITLE` and `ARTIST`). Album/Album Artist/Track/Year/Genre reuse
//! the `Mp3*`-prefixed `FieldKey`s because they are the same concepts as
//! ID3's equivalents; the GUI can therefore apply one value across mixed
//! MP3+FLAC selections.

use std::path::{Path, PathBuf};

use metaflac::Tag;

use crate::{FieldKey, MetadataFile, NmeError, Result};

/// All fields the FLAC backend supports, in GUI display order.
const FLAC_FIELDS: &[FieldKey] = &[
    FieldKey::Title,
    FieldKey::Author,
    FieldKey::Mp3Album,
    FieldKey::Mp3AlbumArtist,
    FieldKey::Mp3Track,
    FieldKey::Mp3Year,
    FieldKey::Mp3Genre,
];

pub struct FlacFile {
    path: PathBuf,
    tag: Tag,
    dirty: bool,
}

impl FlacFile {
    pub fn open(path: &Path) -> Result<Self> {
        let tag = Tag::read_from_path(path).map_err(|source| NmeError::FlacParse {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            tag,
            dirty: false,
        })
    }
}

/// Vorbis comments are multi-valued; we expose the first value, matching
/// how most taggers present TITLE/ARTIST/etc. in a single-line field.
fn first_comment(values: Option<&Vec<String>>) -> Option<String> {
    values.and_then(|v| v.first()).cloned()
}

impl MetadataFile for FlacFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn fields(&self) -> &[FieldKey] {
        FLAC_FIELDS
    }

    fn get(&self, key: FieldKey) -> Option<String> {
        let comments = self.tag.vorbis_comments()?;
        match key {
            FieldKey::Title => first_comment(comments.title()),
            FieldKey::Author => first_comment(comments.artist()),
            FieldKey::Mp3Album => first_comment(comments.album()),
            FieldKey::Mp3AlbumArtist => first_comment(comments.album_artist()),
            FieldKey::Mp3Track => comments.track().map(|n| n.to_string()),
            // Xiph recommends DATE; some taggers write YEAR instead. Prefer
            // DATE when both are present.
            FieldKey::Mp3Year => first_comment(comments.get("DATE"))
                .or_else(|| first_comment(comments.get("YEAR"))),
            FieldKey::Mp3Genre => first_comment(comments.genre()),
            _ => None, // unsupported field for this format
        }
    }

    fn set(&mut self, key: FieldKey, value: Option<String>) {
        let comments = self.tag.vorbis_comments_mut();
        match key {
            FieldKey::Title => match value {
                Some(v) => comments.set_title(vec![v]),
                None => comments.remove_title(),
            },
            FieldKey::Author => match value {
                Some(v) => comments.set_artist(vec![v]),
                None => comments.remove_artist(),
            },
            FieldKey::Mp3Album => match value {
                Some(v) => comments.set_album(vec![v]),
                None => comments.remove_album(),
            },
            FieldKey::Mp3AlbumArtist => match value {
                Some(v) => comments.set_album_artist(vec![v]),
                None => comments.remove_album_artist(),
            },
            FieldKey::Mp3Track => match value {
                // Track is numeric in Vorbis TRACKNUMBER; a value that
                // doesn't parse is silently dropped rather than stored as
                // garbage — same contract as the MP3 backend.
                Some(v) => {
                    if let Ok(n) = v.parse::<u32>() {
                        comments.set_track(n);
                    }
                }
                None => comments.remove_track(),
            },
            FieldKey::Mp3Year => match value {
                Some(v) => {
                    if v.parse::<i32>().is_ok() {
                        comments.set("DATE", vec![v]);
                        comments.remove("YEAR");
                    }
                }
                None => {
                    comments.remove("DATE");
                    comments.remove("YEAR");
                }
            },
            FieldKey::Mp3Genre => match value {
                Some(v) => comments.set_genre(vec![v]),
                None => comments.remove_genre(),
            },
            _ => return, // unsupported field for this format: ignore, per trait contract
        }
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn save(&mut self) -> Result<()> {
        self.tag.save().map_err(|source| NmeError::FlacSave {
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
    use metaflac::block::StreamInfo;

    /// Builds a minimal-but-valid FLAC (STREAMINFO, no audio frames) at a
    /// temp path, mirroring `pdf::tests::minimal_pdf` / `mp3::tests::minimal_mp3`:
    /// no fixture files committed to the repo.
    fn minimal_flac(dir: &tempfile::TempDir, name: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut tag = Tag::new();
        // StreamInfo::new() zeros channel/bit-depth, and metaflac subtracts
        // 1 from both when serializing — so a dummy STREAMINFO needs the
        // FLAC-legal minima (1 channel, 4 bits) to write without overflow.
        tag.set_streaminfo(StreamInfo {
            min_block_size: 16,
            max_block_size: 16,
            min_frame_size: 0,
            max_frame_size: 0,
            sample_rate: 44100,
            num_channels: 1,
            bits_per_sample: 16,
            total_samples: 0,
            md5: vec![0; 16],
        });
        tag.write_to_path(&path).expect("write empty FLAC tag");
        path
    }

    #[test]
    fn set_then_save_then_reopen_round_trips_title_and_artist() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_flac(&dir, "roundtrip.flac");

        let mut flac = FlacFile::open(&path).expect("open");
        assert_eq!(flac.get(FieldKey::Title), None);

        flac.set(FieldKey::Title, Some("Hello".to_string()));
        flac.set(FieldKey::Author, Some("Takashi".to_string()));
        assert!(flac.is_dirty());
        flac.save().expect("save");
        assert!(!flac.is_dirty());

        let reopened = FlacFile::open(&path).expect("reopen");
        assert_eq!(reopened.get(FieldKey::Title), Some("Hello".to_string()));
        assert_eq!(reopened.get(FieldKey::Author), Some("Takashi".to_string()));
    }

    #[test]
    fn numeric_fields_round_trip_as_strings() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_flac(&dir, "numeric.flac");

        let mut flac = FlacFile::open(&path).expect("open");
        flac.set(FieldKey::Mp3Track, Some("7".to_string()));
        flac.set(FieldKey::Mp3Year, Some("1998".to_string()));
        flac.save().expect("save");

        let reopened = FlacFile::open(&path).expect("reopen");
        assert_eq!(reopened.get(FieldKey::Mp3Track), Some("7".to_string()));
        assert_eq!(reopened.get(FieldKey::Mp3Year), Some("1998".to_string()));
    }

    #[test]
    fn invalid_numeric_value_is_silently_ignored() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_flac(&dir, "invalid_numeric.flac");

        let mut flac = FlacFile::open(&path).expect("open");
        flac.set(FieldKey::Mp3Track, Some("not-a-number".to_string()));
        assert_eq!(flac.get(FieldKey::Mp3Track), None);
    }

    #[test]
    fn untagged_file_reads_as_all_unset_rather_than_erroring() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_flac(&dir, "untagged.flac");

        let flac = FlacFile::open(&path).expect("open with no Vorbis comments");
        for &key in flac.fields() {
            assert_eq!(flac.get(key), None);
        }
    }

    #[test]
    fn fields_lists_all_flac_keys_in_display_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = minimal_flac(&dir, "fields.flac");
        let flac = FlacFile::open(&path).expect("open");

        assert_eq!(flac.fields(), FLAC_FIELDS);
    }
}
