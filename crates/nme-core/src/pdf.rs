//! PDF metadata backend, built on `lopdf` (MIT licensed, pure Rust).
//!
//! Only the document's `/Info` dictionary is touched — the legacy
//! Title/Author/Subject/Keywords/Creator/Producer/CreationDate/ModDate
//! entries that every PDF viewer still reads. Content streams, pages, and
//! everything else in the document are left completely untouched, which is
//! what lets a low-level library like `lopdf` be sufficient here: metadata
//! editing doesn't need a page-rendering-capable "high-level" PDF engine.
//!
//! Note: many modern tools additionally mirror this metadata into an XMP
//! stream (`/Metadata`). We intentionally don't touch XMP in the MVP —
//! writing only the Info dict keeps the change small and is still read by
//! effectively every PDF viewer — but nothing in the `MetadataFile` shape
//! assumes Info-dict-only, so XMP read/write can be added later as an
//! internal detail of this module.

use std::path::{Path, PathBuf};

use lopdf::{Dictionary, Document, Object, StringFormat};

use crate::{FieldKey, MetadataFile, NmeError, Result};

/// All fields the PDF backend supports, in GUI display order.
const PDF_FIELDS: &[FieldKey] = &[
    FieldKey::Title,
    FieldKey::Author,
    FieldKey::Subject,
    FieldKey::Keywords,
    FieldKey::Creator,
    FieldKey::Producer,
    FieldKey::CreationDate,
    FieldKey::ModDate,
];

/// Maps a format-agnostic [`FieldKey`] to the PDF `/Info` dictionary key
/// that carries it. Returns `None` for keys this format doesn't support.
fn info_dict_key(key: FieldKey) -> Option<&'static [u8]> {
    match key {
        FieldKey::Title => Some(b"Title"),
        FieldKey::Author => Some(b"Author"),
        FieldKey::Subject => Some(b"Subject"),
        FieldKey::Keywords => Some(b"Keywords"),
        FieldKey::Creator => Some(b"Creator"),
        FieldKey::Producer => Some(b"Producer"),
        FieldKey::CreationDate => Some(b"CreationDate"),
        FieldKey::ModDate => Some(b"ModDate"),
    }
}

pub struct PdfFile {
    path: PathBuf,
    doc: Document,
    dirty: bool,
}

impl PdfFile {
    pub fn open(path: &Path) -> Result<Self> {
        let doc = Document::load(path).map_err(|source| NmeError::PdfParse {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            doc,
            dirty: false,
        })
    }

    /// Read-only access to the `/Info` dictionary, if the trailer points to
    /// one and it resolves to an actual dictionary object.
    fn info_dict(&self) -> Option<&Dictionary> {
        let info_ref = self.doc.trailer.get(b"Info").ok()?;
        let info_id = info_ref.as_reference().ok()?;
        self.doc.get_object(info_id).ok()?.as_dict().ok()
    }

    /// Mutable access to the `/Info` dictionary, creating one (and wiring
    /// it into the trailer) on first write if the document doesn't have one
    /// yet.
    fn info_dict_mut(&mut self) -> &mut Dictionary {
        let info_id = match self
            .doc
            .trailer
            .get(b"Info")
            .ok()
            .and_then(|o| o.as_reference().ok())
        {
            Some(id) => id,
            None => {
                let id = self.doc.add_object(Dictionary::new());
                self.doc.trailer.set("Info", Object::Reference(id));
                id
            }
        };

        let obj = self
            .doc
            .objects
            .entry(info_id)
            .or_insert_with(|| Object::Dictionary(Dictionary::new()));
        if !matches!(obj, Object::Dictionary(_)) {
            *obj = Object::Dictionary(Dictionary::new());
        }
        match obj {
            Object::Dictionary(dict) => dict,
            _ => unreachable!("just normalized to Dictionary above"),
        }
    }
}

/// Decodes a PDF string object's bytes as text.
///
/// PDF text strings are technically PDFDocEncoding, or UTF-16BE with a
/// `\xFE\xFF` BOM — not UTF-8. For the MVP we decode UTF-16BE when the BOM
/// is present and fall back to lossy UTF-8 otherwise, which is correct for
/// the overwhelming majority of real-world PDFs (ASCII/Latin-1-range
/// metadata) and never panics on the rest. Proper PDFDocEncoding decoding
/// can be added later without changing the public `MetadataFile` API.
fn decode_pdf_string(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xFE, 0xFF]) {
        let utf16: Vec<u16> = rest
            .chunks_exact(2)
            .map(|c| u16::from_be_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&utf16)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

impl MetadataFile for PdfFile {
    fn path(&self) -> &Path {
        &self.path
    }

    fn fields(&self) -> &[FieldKey] {
        PDF_FIELDS
    }

    fn get(&self, key: FieldKey) -> Option<String> {
        let dict_key = info_dict_key(key)?;
        let obj = self.info_dict()?.get(dict_key).ok()?;
        let bytes = obj.as_str().ok()?;
        Some(decode_pdf_string(bytes))
    }

    /// ```
    /// use std::path::Path;
    /// use nme_core::FieldKey;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut doc = nme_core::format::open(Path::new("test.pdf"))?;
    /// doc.set(FieldKey::Title, Some("Hello".into()));
    /// doc.save()?;
    /// # Ok(())
    /// # }
    /// ```
    fn set(&mut self, key: FieldKey, value: Option<String>) {
        let Some(dict_key) = info_dict_key(key) else {
            return; // unsupported field for this format: ignore, per trait contract
        };
        let info = self.info_dict_mut();
        match value {
            Some(text) => {
                info.set(dict_key, Object::String(text.into_bytes(), StringFormat::Literal));
            }
            None => {
                info.remove(dict_key);
            }
        }
        self.dirty = true;
    }

    fn is_dirty(&self) -> bool {
        self.dirty
    }

    fn save(&mut self) -> Result<()> {
        self.doc
            .save(&self.path)
            .map_err(|source| NmeError::PdfSave {
                path: self.path.clone(),
                source: lopdf::Error::IO(source),
            })?;
        self.dirty = false;
        Ok(())
    }
}
