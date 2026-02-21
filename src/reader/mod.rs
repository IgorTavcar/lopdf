mod encrypted;
mod load;
mod metadata;
mod object_loader;

#[cfg(test)]
mod tests;

use log::{error, warn};
use std::cmp;
use std::collections::{BTreeMap, HashSet};
use std::sync::Mutex;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

use crate::encryption::EncryptionState;
use crate::error::{ParseError, XrefError};
use crate::object_stream::ObjectStream;
use crate::parser::{self, ParserInput};
use crate::xref::XrefEntry;
use crate::{Document, Error, Object, Result};

pub use metadata::PdfMetadata;

pub(crate) type FilterFunc = fn((u32, u16), &mut Object) -> Option<((u32, u16), Object)>;

pub struct Reader<'a> {
    pub buffer: &'a [u8],
    pub document: Document,
    pub encryption_state: Option<EncryptionState>,
    pub password: Option<String>, // Password for encrypted PDFs
}

/// Maximum allowed embedding of literal strings.
pub const MAX_BRACKET: usize = 100;

impl Reader<'_> {
    /// Read whole document.
    pub fn read(mut self, filter_func: Option<FilterFunc>) -> Result<Document> {
        let offset = self.buffer.windows(5).position(|w| w == b"%PDF-").unwrap_or(0);
        self.buffer = &self.buffer[offset..];

        // The document structure can be expressed in PEG as:
        //   document <- header indirect_object* xref trailer xref_start
        let version =
            parser::header(ParserInput::new_extra(self.buffer, "header")).ok_or(ParseError::InvalidFileHeader)?;

        //The binary_mark is in line 2 after the pdf version. If at other line number, then will be declared as invalid pdf.
        if let Some(pos) = self.buffer.iter().position(|&byte| byte == b'\n') {
            if let Some(binary_mark) =
                parser::binary_mark(ParserInput::new_extra(&self.buffer[pos + 1..], "binary_mark"))
            {
                if binary_mark.iter().all(|&byte| byte >= 128) {
                    self.document.binary_mark = binary_mark;
                }
            }
        }

        let xref_start = Self::get_xref_start(self.buffer)?;
        if xref_start > self.buffer.len() {
            return Err(Error::Xref(XrefError::Start));
        }
        self.document.xref_start = xref_start;

        let (mut xref, mut trailer) =
            parser::xref_and_trailer(ParserInput::new_extra(&self.buffer[xref_start..], "xref"), &self)?;

        // Read previous Xrefs of linearized or incremental updated document.
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

            // Read xref stream in hybrid-reference file
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

        self.document.version = version;
        self.document.max_id = xref.size - 1;
        self.document.trailer = trailer;
        self.document.reference_table = xref;

        // Check if encrypted
        let is_encrypted = self.document.trailer.get(b"Encrypt").is_ok();

        if is_encrypted {
            // For encrypted PDFs, use a special loading strategy
            self.load_encrypted_document(filter_func)?;
        } else {
            // For non-encrypted PDFs, use the normal loading
            self.load_objects_raw(filter_func)?;
        }

        Ok(self.document)
    }

    fn load_objects_raw(&mut self, filter_func: Option<FilterFunc>) -> Result<()> {
        let is_encrypted = self.document.trailer.get(b"Encrypt").is_ok();
        let zero_length_streams = Mutex::new(vec![]);
        let object_streams = Mutex::new(vec![]);

        let entries_filter_map = |(_, entry): (&_, &_)| {
            if let XrefEntry::Normal { offset, .. } = *entry {
                // read_object now handles decryption internally
                let result = self.read_object(offset as usize, None, &mut HashSet::new());
                let (object_id, mut object) = match result {
                    Ok(obj) => obj,
                    Err(e) => {
                        // Log error but continue
                        if is_encrypted {
                            // Expected for some encrypted objects - but log which ones
                            warn!("Skipping encrypted object at offset {}: {:?}", offset, e);
                        } else {
                            error!("Object load error at offset {}: {e:?}", offset);
                        }
                        return None;
                    }
                };
                if let Some(filter_func) = filter_func {
                    filter_func(object_id, &mut object)?;
                }

                if let Ok(ref mut stream) = object.as_stream_mut() {
                    if stream.dict.has_type(b"ObjStm") && !is_encrypted {
                        let obj_stream = ObjectStream::new(stream).ok()?;
                        let mut object_streams = object_streams.lock().expect("object_streams mutex poisoned");
                        if let Some(filter_func) = filter_func {
                            let objects: BTreeMap<(u32, u16), Object> = obj_stream
                                .objects
                                .into_iter()
                                .filter_map(|(object_id, mut object)| filter_func(object_id, &mut object))
                                .collect();
                            object_streams.extend(objects);
                        } else {
                            object_streams.extend(obj_stream.objects);
                        }
                    } else if stream.content.is_empty() {
                        let mut zero_length_streams =
                            zero_length_streams.lock().expect("zero_length_streams mutex poisoned");
                        zero_length_streams.push(object_id);
                    }
                }

                Some((object_id, object))
            } else {
                None
            }
        };

        #[cfg(feature = "rayon")]
        {
            self.document.objects = self
                .document
                .reference_table
                .entries
                .par_iter()
                .filter_map(entries_filter_map)
                .collect();
        }
        #[cfg(not(feature = "rayon"))]
        {
            self.document.objects = self
                .document
                .reference_table
                .entries
                .iter()
                .filter_map(entries_filter_map)
                .collect();
        }

        // Per PDF spec, first definition wins for duplicate object IDs.
        // See https://github.com/J-F-Liu/lopdf/issues/160
        for (id, entry) in object_streams.into_inner().expect("object_streams mutex poisoned") {
            self.document.objects.entry(id).or_insert(entry);
        }

        for object_id in zero_length_streams.into_inner().expect("zero_length_streams mutex poisoned") {
            let _ = self.read_stream_content(object_id);
        }

        Ok(())
    }

    fn get_xref_start(buffer: &[u8]) -> Result<usize> {
        let seek_pos = buffer.len() - cmp::min(buffer.len(), 512);
        Self::search_substring(buffer, b"%%EOF", seek_pos)
            .and_then(|eof_pos| if eof_pos > 25 { Some(eof_pos) } else { None })
            .and_then(|eof_pos| Self::search_substring(buffer, b"startxref", eof_pos - 25))
            .ok_or(Error::Xref(XrefError::Start))
            .and_then(|xref_pos| {
                if xref_pos <= buffer.len() {
                    match parser::xref_start(ParserInput::new_extra(&buffer[xref_pos..], "xref")) {
                        Some(startxref) => Ok(startxref as usize),
                        None => Err(Error::Xref(XrefError::Start)),
                    }
                } else {
                    Err(Error::Xref(XrefError::Start))
                }
            })
    }

    pub(crate) fn search_substring(buffer: &[u8], pattern: &[u8], start_pos: usize) -> Option<usize> {
        buffer
            .get(start_pos..)?
            .windows(pattern.len())
            .rposition(|window| window == pattern)
            .map(|pos| start_pos + pos)
    }
}
