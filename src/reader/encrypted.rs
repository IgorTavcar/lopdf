use log::warn;
use std::collections::HashSet;

use super::{FilterFunc, Reader};
use crate::encryption::{self, EncryptionState};
use crate::error::ParseError;
use crate::object_stream::ObjectStream;
use crate::parser::{self, ParserInput};
use crate::xref::XrefEntry;
use crate::{Error, Object, ObjectId, Result};

impl Reader<'_> {
    pub(super) fn load_encrypted_document(&mut self, _filter_func: Option<FilterFunc>) -> Result<()> {
        // First, extract all raw object bytes without parsing
        let entries: Vec<_> = self
            .document
            .reference_table
            .entries
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect();

        let mut object_streams = Vec::new();

        for (obj_num, entry) in entries {
            match entry {
                XrefEntry::Normal { offset, .. } => {
                    if let Ok((obj_id, raw_bytes)) = self.extract_raw_object(offset as usize) {
                        self.raw_objects.insert(obj_id, raw_bytes);
                    }
                }
                XrefEntry::Compressed { container, index } => {
                    // Store compressed object info for later processing
                    object_streams.push((obj_num, container, index));
                }
                XrefEntry::Free { .. } | XrefEntry::UnusableFree => {
                    // Skip free entries
                }
            }
        }

        self.parse_encryption_dictionary()?;

        if self.authenticate_and_setup_encryption(false)?.is_none() {
            return Ok(());
        }

        if let Some(ref state) = self.encryption_state {
            let encrypt_ref = self
                .document
                .trailer
                .get(b"Encrypt")
                .ok()
                .and_then(|o| o.as_reference().ok());

            for (obj_id, raw_bytes) in &self.raw_objects {
                if let Some(enc_ref) = encrypt_ref {
                    if *obj_id == enc_ref {
                        continue;
                    }
                }

                if let Ok((id, mut obj)) = self.parse_raw_object(raw_bytes) {
                    let _ = encryption::decrypt_object(state, *obj_id, &mut obj);
                    self.document.objects.insert(id, obj);
                }
            }

            let mut streams_to_process: std::collections::HashMap<u32, Vec<(u32, u16)>> =
                std::collections::HashMap::new();
            for (obj_num, container_id, index) in object_streams {
                streams_to_process
                    .entry(container_id)
                    .or_default()
                    .push((obj_num, index));
            }

            for (container_id, objects_in_stream) in streams_to_process {
                if let Some(container_obj) = self.document.objects.get_mut(&(container_id, 0)) {
                    if let Ok(stream) = container_obj.as_stream_mut() {
                        match ObjectStream::new(stream) {
                            Ok(object_stream) => {
                                for (obj_num, _index) in objects_in_stream {
                                    let obj_id = (obj_num, 0);
                                    if let Some(obj) = object_stream.objects.get(&obj_id) {
                                        self.document.objects.insert(obj_id, obj.clone());
                                    }
                                }
                            }
                            Err(_e) => {}
                        }
                    }
                }
            }

            self.document.encryption_state = Some(state.clone());

            if let Some(enc_ref) = encrypt_ref {
                self.document.objects.remove(&enc_ref);
            }
            self.document.trailer.remove(b"Encrypt");
        }

        Ok(())
    }

    pub(super) fn parse_raw_object(&self, raw_bytes: &[u8]) -> Result<(ObjectId, Object)> {
        // Parse the raw bytes as an indirect object
        parser::indirect_object(
            ParserInput::new_extra(raw_bytes, "indirect object"),
            0,
            None,
            self,
            &mut HashSet::new(),
        )
    }

    pub(super) fn parse_encryption_dictionary(&mut self) -> Result<()> {
        if let Ok(encrypt_ref) = self.document.trailer.get(b"Encrypt").and_then(|o| o.as_reference()) {
            if self.raw_objects.is_empty() {
                let offset = self.get_offset(encrypt_ref)?;
                let (_, encrypt_obj) = self.read_object(offset as usize, Some(encrypt_ref), &mut HashSet::new())?;
                self.document.objects.insert(encrypt_ref, encrypt_obj);
            } else if let Some(raw_bytes) = self.raw_objects.get(&encrypt_ref) {
                if let Ok((_, obj)) = self.parse_raw_object(raw_bytes) {
                    self.document.objects.insert(encrypt_ref, obj);
                }
            }
        }
        Ok(())
    }

    pub(super) fn authenticate_and_setup_encryption(&mut self, require_password: bool) -> Result<Option<String>> {
        let password_to_use: Option<String> = if self.document.authenticate_password("").is_ok() {
            Some(String::new())
        } else if let Some(ref pwd) = self.password {
            if self.document.authenticate_password(pwd).is_ok() {
                Some(pwd.clone())
            } else if require_password {
                return Err(Error::InvalidPassword);
            } else {
                warn!("Invalid password provided for encrypted PDF");
                return Err(Error::InvalidPassword);
            }
        } else if require_password {
            return Err(Error::Unimplemented(
                "PDF is encrypted and requires a password. Use Document::load_metadata_with_password() instead.",
            ));
        } else {
            warn!("PDF is encrypted and requires a password");
            return Ok(None);
        };

        if let Some(ref password) = password_to_use {
            let state = EncryptionState::decode(&self.document, password)?;
            self.encryption_state = Some(state);
        }

        Ok(password_to_use)
    }

    pub(super) fn setup_encryption_for_metadata(&mut self) -> Result<()> {
        self.parse_encryption_dictionary()?;
        self.authenticate_and_setup_encryption(true)?;
        Ok(())
    }

    pub(super) fn extract_raw_object(&mut self, offset: usize) -> Result<(ObjectId, Vec<u8>)> {
        if offset > self.buffer.len() {
            return Err(Error::InvalidOffset(offset));
        }

        // Find object header (e.g., "19 0 obj")
        let slice = &self.buffer[offset..];

        // Parse object ID
        let mut pos = 0;
        while pos < slice.len() && slice[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Get object number
        let num_start = pos;
        while pos < slice.len() && slice[pos].is_ascii_digit() {
            pos += 1;
        }
        let obj_num: u32 = std::str::from_utf8(&slice[num_start..pos])
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Parse(ParseError::InvalidXref))?;

        // Skip whitespace
        while pos < slice.len() && slice[pos].is_ascii_whitespace() {
            pos += 1;
        }

        // Get generation number
        let gen_start = pos;
        while pos < slice.len() && slice[pos].is_ascii_digit() {
            pos += 1;
        }
        let obj_gen: u16 = std::str::from_utf8(&slice[gen_start..pos])
            .ok()
            .and_then(|s| s.parse().ok())
            .ok_or(Error::Parse(ParseError::InvalidXref))?;

        // Skip to "obj"
        while pos < slice.len() && slice[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos + 3 > slice.len() || &slice[pos..pos + 3] != b"obj" {
            return Err(Error::Parse(ParseError::InvalidXref));
        }
        pos += 3;

        // Find "endobj"
        let endobj_pattern = b"endobj";
        let mut end_pos = pos;
        while end_pos + endobj_pattern.len() <= slice.len() {
            if &slice[end_pos..end_pos + endobj_pattern.len()] == endobj_pattern {
                end_pos += endobj_pattern.len();
                break;
            }
            end_pos += 1;
        }

        if end_pos > slice.len() {
            return Err(Error::Parse(ParseError::InvalidXref));
        }

        // Extract raw object bytes (including header and trailer)
        let raw_bytes = slice[0..end_pos].to_vec();

        Ok(((obj_num, obj_gen), raw_bytes))
    }
}
