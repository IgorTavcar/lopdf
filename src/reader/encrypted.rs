use log::warn;
use std::collections::HashSet;

use super::{FilterFunc, Reader};
use crate::encryption::{self, EncryptionState};
use crate::object_stream::ObjectStream;
use crate::{Error, Object, ObjectId, Result};

impl Reader<'_> {
    pub(super) fn load_encrypted_document(&mut self, filter_func: Option<FilterFunc>) -> Result<()> {
        // Step 1: Parse the Encrypt dictionary using the proper parser.
        // Since raw_objects is empty, parse_encryption_dictionary uses read_object().
        self.parse_encryption_dictionary()?;

        // Step 2: Authenticate and set up encryption
        if self.authenticate_and_setup_encryption(false)?.is_none() {
            return Ok(());
        }

        // Step 3: Load all objects using the proper parser.
        // load_objects_raw already skips ObjStm processing for encrypted documents.
        self.load_objects_raw(filter_func)?;

        // Step 4: Decrypt all loaded objects and process object streams
        if let Some(ref state) = self.encryption_state {
            let encrypt_ref = self
                .document
                .trailer
                .get(b"Encrypt")
                .ok()
                .and_then(|o| o.as_reference().ok());

            // Decrypt all objects (skip the encryption dictionary itself)
            for (&obj_id, obj) in self.document.objects.iter_mut() {
                if Some(obj_id) == encrypt_ref {
                    continue;
                }
                let _ = encryption::decrypt_object(state, obj_id, obj);
            }

            // Step 5: Process object streams now that they're decrypted
            let obj_stream_ids: Vec<ObjectId> = self
                .document
                .objects
                .iter()
                .filter_map(|(&id, obj)| {
                    if let Object::Stream(stream) = obj {
                        if stream.dict.has_type(b"ObjStm") {
                            return Some(id);
                        }
                    }
                    None
                })
                .collect();

            for container_id in obj_stream_ids {
                if let Some(container_obj) = self.document.objects.get_mut(&container_id) {
                    if let Ok(stream) = container_obj.as_stream_mut() {
                        if let Ok(object_stream) = ObjectStream::new(stream) {
                            for (obj_id, obj) in object_stream.objects {
                                self.document.objects.entry(obj_id).or_insert(obj);
                            }
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

    pub(super) fn parse_encryption_dictionary(&mut self) -> Result<()> {
        if let Ok(encrypt_ref) = self.document.trailer.get(b"Encrypt").and_then(|o| o.as_reference()) {
            let offset = self.get_offset(encrypt_ref)?;
            let (_, encrypt_obj) = self.read_object(offset as usize, Some(encrypt_ref), &mut HashSet::new())?;
            self.document.objects.insert(encrypt_ref, encrypt_obj);
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

}
