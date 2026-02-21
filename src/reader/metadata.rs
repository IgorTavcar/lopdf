use log::warn;
use std::collections::HashSet;

use super::Reader;
use crate::error::{ParseError, XrefError};
use crate::parser::{self, ParserInput};
use crate::{Dictionary, Error, Object, ObjectId, Result};

/// PDF metadata extracted without loading the entire document.
/// This is useful for quickly getting basic information about large PDFs.
#[derive(Debug, Clone)]
pub struct PdfMetadata {
    /// Document title from Info dictionary
    pub title: Option<String>,
    /// Document author from Info dictionary
    pub author: Option<String>,
    /// Document subject from Info dictionary
    pub subject: Option<String>,
    /// Document keywords from Info dictionary
    pub keywords: Option<String>,
    /// Application that created the document
    pub creator: Option<String>,
    /// Application that produced the document
    pub producer: Option<String>,
    /// Document creation date (PDF date format: D:YYYYMMDDHHmmSSOHH'mm')
    pub creation_date: Option<String>,
    /// Document modification date (PDF date format: D:YYYYMMDDHHmmSSOHH'mm')
    pub modification_date: Option<String>,
    /// Number of pages in the document
    pub page_count: u32,
    /// PDF version
    pub version: String,
}

pub struct InfoMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
}

impl Reader<'_> {
    /// Read metadata (title and page count) without loading the entire document.
    /// This is much faster for large PDFs when you only need basic information.
    ///
    /// For encrypted PDFs, use `Document::load_metadata_with_password()` instead.
    pub fn read_metadata(mut self) -> Result<PdfMetadata> {
        let offset = self.buffer.windows(5).position(|w| w == b"%PDF-").unwrap_or(0);
        self.buffer = &self.buffer[offset..];

        let version =
            parser::header(ParserInput::new_extra(self.buffer, "header")).ok_or(ParseError::InvalidFileHeader)?;

        let xref_start = Self::get_xref_start(self.buffer)?;
        if xref_start > self.buffer.len() {
            return Err(Error::Xref(XrefError::Start));
        }

        let (mut xref, mut trailer) =
            parser::xref_and_trailer(ParserInput::new_extra(&self.buffer[xref_start..], "xref"), &self)?;

        let mut already_seen = HashSet::new();
        let mut prev_xref_start = trailer.remove(b"Prev");
        while let Some(prev) = prev_xref_start.and_then(|offset| offset.as_i64().ok()) {
            if already_seen.contains(&prev) {
                break;
            }
            already_seen.insert(prev);
            if prev < 0 || prev as usize > self.buffer.len() {
                return Err(Error::Xref(XrefError::PrevStart));
            }

            let (prev_xref, prev_trailer) =
                parser::xref_and_trailer(ParserInput::new_extra(&self.buffer[prev as usize..], ""), &self)?;
            xref.merge(prev_xref);

            let prev_xref_stream_start = trailer.remove(b"XRefStm");
            if let Some(prev) = prev_xref_stream_start.and_then(|offset| offset.as_i64().ok()) {
                if prev < 0 || prev as usize > self.buffer.len() {
                    return Err(Error::Xref(XrefError::StreamStart));
                }

                let (prev_xref, _) =
                    parser::xref_and_trailer(ParserInput::new_extra(&self.buffer[prev as usize..], ""), &self)?;
                xref.merge(prev_xref);
            }

            prev_xref_start = prev_trailer.get(b"Prev").cloned().ok();
        }
        let xref_entry_count = xref.max_id().checked_add(1).ok_or(ParseError::InvalidXref)?;
        if xref.size != xref_entry_count {
            warn!(
                "Size entry of trailer dictionary is {}, correct value is {}.",
                xref.size, xref_entry_count
            );
            xref.size = xref_entry_count;
        }

        self.document.reference_table = xref;
        self.document.trailer = trailer.clone();

        if self.document.trailer.get(b"Encrypt").is_ok() {
            self.setup_encryption_for_metadata()?;
        }

        let info_metadata = self.extract_info_metadata()?;
        let page_count = self.extract_page_count()?;

        Ok(PdfMetadata {
            title: info_metadata.title,
            author: info_metadata.author,
            subject: info_metadata.subject,
            keywords: info_metadata.keywords,
            creator: info_metadata.creator,
            producer: info_metadata.producer,
            creation_date: info_metadata.creation_date,
            modification_date: info_metadata.modification_date,
            page_count,
            version,
        })
    }

    pub(super) fn extract_info_metadata(&self) -> Result<InfoMetadata> {
        let info_ref = match self.document.trailer.get(b"Info") {
            Ok(obj) => obj.as_reference().ok(),
            Err(_) => {
                return Ok(InfoMetadata {
                    title: None,
                    author: None,
                    subject: None,
                    keywords: None,
                    creator: None,
                    producer: None,
                    creation_date: None,
                    modification_date: None,
                });
            }
        };

        let info_id = match info_ref {
            Some(id) => id,
            None => {
                return Ok(InfoMetadata {
                    title: None,
                    author: None,
                    subject: None,
                    keywords: None,
                    creator: None,
                    producer: None,
                    creation_date: None,
                    modification_date: None,
                });
            }
        };

        let mut already_seen = HashSet::new();
        let info_obj = match self.get_object(info_id, &mut already_seen) {
            Ok(obj) => obj,
            Err(_) => {
                return Ok(InfoMetadata {
                    title: None,
                    author: None,
                    subject: None,
                    keywords: None,
                    creator: None,
                    producer: None,
                    creation_date: None,
                    modification_date: None,
                });
            }
        };

        let info_dict = match info_obj.as_dict() {
            Ok(dict) => dict,
            Err(_) => {
                return Ok(InfoMetadata {
                    title: None,
                    author: None,
                    subject: None,
                    keywords: None,
                    creator: None,
                    producer: None,
                    creation_date: None,
                    modification_date: None,
                });
            }
        };

        Ok(InfoMetadata {
            title: Self::extract_string_field(info_dict, b"Title"),
            author: Self::extract_string_field(info_dict, b"Author"),
            subject: Self::extract_string_field(info_dict, b"Subject"),
            keywords: Self::extract_string_field(info_dict, b"Keywords"),
            creator: Self::extract_string_field(info_dict, b"Creator"),
            producer: Self::extract_string_field(info_dict, b"Producer"),
            creation_date: Self::extract_string_field(info_dict, b"CreationDate"),
            modification_date: Self::extract_string_field(info_dict, b"ModDate"),
        })
    }

    fn extract_string_field(dict: &Dictionary, key: &[u8]) -> Option<String> {
        match dict.get(key) {
            Ok(obj) => match obj {
                Object::String(bytes, _) => {
                    let s = if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
                        let utf16_bytes: Vec<u16> = bytes[2..]
                            .chunks_exact(2)
                            .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
                            .collect();
                        String::from_utf16_lossy(&utf16_bytes)
                    } else {
                        String::from_utf8_lossy(bytes).to_string()
                    };
                    Some(s)
                }
                _ => None,
            },
            Err(_) => None,
        }
    }

    pub(super) fn extract_page_count(&self) -> Result<u32> {
        let root_ref = match self.document.trailer.get(b"Root").and_then(Object::as_reference) {
            Ok(id) => id,
            Err(_) => return Ok(0),
        };

        let mut already_seen = HashSet::new();
        let catalog_obj = match self.get_object(root_ref, &mut already_seen) {
            Ok(obj) => obj,
            Err(_) => return Ok(0),
        };

        let catalog_dict = match catalog_obj.as_dict() {
            Ok(dict) => dict,
            Err(_) => return Ok(0),
        };

        let pages_ref = match catalog_dict.get(b"Pages").and_then(Object::as_reference) {
            Ok(id) => id,
            Err(_) => return Ok(0),
        };

        self.get_pages_tree_count(pages_ref, &mut HashSet::new()).or(Ok(0))
    }

    fn get_pages_tree_count(&self, pages_id: ObjectId, seen: &mut HashSet<ObjectId>) -> Result<u32> {
        if seen.contains(&pages_id) {
            return Err(Error::ReferenceCycle(pages_id));
        }
        seen.insert(pages_id);

        let mut already_seen = HashSet::new();
        let pages_obj = match self.get_object(pages_id, &mut already_seen) {
            Ok(obj) => obj,
            Err(_) => return Ok(0),
        };

        let pages_dict = match pages_obj.as_dict() {
            Ok(dict) => dict,
            Err(_) => return Ok(0),
        };

        match pages_dict.get_type() {
            Ok(type_name) if type_name == b"Page" => Ok(1),
            Ok(type_name) if type_name == b"Pages" => {
                if let Ok(count_obj) = pages_dict.get(b"Count") {
                    if let Ok(count) = count_obj.as_i64() {
                        if count >= 0 {
                            return Ok(count as u32);
                        }
                    }
                }

                let kids = match pages_dict.get(b"Kids").and_then(Object::as_array) {
                    Ok(arr) => arr,
                    Err(_) => return Ok(0),
                };

                let mut total = 0u32;
                for kid in kids.iter() {
                    if let Ok(kid_ref) = kid.as_reference() {
                        if let Ok(count) = self.get_pages_tree_count(kid_ref, seen) {
                            total += count;
                        }
                    }
                }
                Ok(total)
            }
            _ => Ok(1),
        }
    }
}
