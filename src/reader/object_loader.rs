use log::{error, warn};
use std::collections::HashSet;

use super::Reader;
use crate::encryption;
use crate::object_stream::ObjectStream;
use crate::parser::{self, ParserInput};
use crate::xref::XrefEntry;
use crate::{Error, Object, ObjectId, Result};

impl Reader<'_> {
    pub fn get_object(&self, id: ObjectId, already_seen: &mut HashSet<ObjectId>) -> Result<Object> {
        if already_seen.contains(&id) {
            warn!("reference cycle detected resolving object {} {}", id.0, id.1);
            return Err(Error::ReferenceCycle(id));
        }
        already_seen.insert(id);

        if let Some(entry) = self.document.reference_table.get(id.0) {
            if matches!(entry, XrefEntry::Compressed { .. }) {
                return self.get_compressed_object(id);
            }
        }

        let offset = self.get_offset(id)?;
        let (_, mut obj) = self.read_object(offset as usize, Some(id), already_seen)?;

        if let Some(ref state) = self.encryption_state {
            let encrypt_ref = self
                .document
                .trailer
                .get(b"Encrypt")
                .ok()
                .and_then(|o| o.as_reference().ok());
            if let Some(enc_ref) = encrypt_ref {
                if id != enc_ref {
                    encryption::decrypt_object(state, id, &mut obj).map_err(Error::Decryption)?;
                }
            }
        }

        Ok(obj)
    }

    /// Get object offset by object ID.
    pub(super) fn get_offset(&self, id: ObjectId) -> Result<u32> {
        let entry = self.document.reference_table.get(id.0).ok_or(Error::MissingXrefEntry)?;
        match *entry {
            XrefEntry::Normal { offset, generation } if generation == id.1 => Ok(offset),
            _ => Err(Error::MissingXrefEntry),
        }
    }

    /// Load a compressed object from an object stream (for lightweight metadata extraction)
    pub(super) fn get_compressed_object(&self, id: ObjectId) -> Result<Object> {
        let entry = self.document.reference_table.get(id.0).ok_or(Error::MissingXrefEntry)?;

        let container_id = match entry {
            XrefEntry::Compressed { container, .. } => *container,
            _ => return Err(Error::MissingXrefEntry),
        };

        let container_id = (container_id, 0);
        let mut already_seen = HashSet::new();
        let container_obj = self.get_object(container_id, &mut already_seen)?;
        let mut container_stream = container_obj.as_stream()?.clone();
        let object_stream = ObjectStream::new(&mut container_stream)?;
        object_stream.objects.get(&id).cloned().ok_or(Error::MissingXrefEntry)
    }

    pub(super) fn read_object(
        &self, offset: usize, expected_id: Option<ObjectId>, already_seen: &mut HashSet<ObjectId>,
    ) -> Result<(ObjectId, Object)> {
        if offset > self.buffer.len() {
            return Err(Error::InvalidOffset(offset));
        }

        // Just parse without decryption - we'll decrypt later
        parser::indirect_object(
            ParserInput::new_extra(self.buffer, "indirect object"),
            offset,
            expected_id,
            self,
            already_seen,
        )
    }

    pub(super) fn read_stream_content(&mut self, object_id: ObjectId) -> Result<()> {
        let length = self.get_stream_length(object_id)?;
        let stream = self
            .document
            .get_object_mut(object_id)
            .and_then(Object::as_stream_mut)?;
        let start = stream
            .start_position
            .ok_or(Error::InvalidStream("missing start position".to_string()))?;

        if length < 0 {
            return Err(Error::InvalidStream("negative stream length.".to_string()));
        }

        let length = usize::try_from(length).map_err(|e| Error::NumericCast(e.to_string()))?;
        let end = start + length;

        if end > self.buffer.len() {
            return Err(Error::InvalidStream("stream extends after document end.".to_string()));
        }

        stream.set_content(self.buffer[start..end].to_vec());
        Ok(())
    }

    fn get_stream_length(&self, object_id: ObjectId) -> Result<i64> {
        let object = self.document.get_object(object_id)?;
        let stream = object.as_stream()?;
        stream
            .dict
            .get(b"Length")
            .and_then(|value| self.document.dereference(value))
            .and_then(|(_id, obj)| obj.as_i64())
            .inspect_err(|_err| {
                error!(
                    "stream dictionary of '{} {} R' is missing the Length entry",
                    object_id.0, object_id.1
                );
            })
    }
}
