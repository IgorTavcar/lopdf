use super::{Dictionary, Document, Object, Result};
use indexmap::IndexMap;
#[derive(Debug, Clone)]
pub struct Destination(Dictionary);

impl Destination {
    pub fn new(title: Object, page: Object, typ: Object) -> Self {
        let mut dict = Dictionary::new();
        dict.set(b"Title", title);
        dict.set(b"Page", page);
        dict.set(b"Type", typ);
        Destination(dict)
    }

    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Vec<u8>>,
        V: Into<Object>,
    {
        self.0.set(key, value);
    }

    pub fn title(&self) -> Result<&Object> {
        self.0.get(b"Title")
    }

    pub fn page(&self) -> Result<&Object> {
        self.0.get(b"Page")
    }
}

impl Document {
    pub fn get_named_destinations(
        &self, tree: &Dictionary, named_destinations: &mut IndexMap<Vec<u8>, Destination>,
    ) -> Result<()> {
        if let Ok(kids) = tree.get(b"Kids") {
            for kid in kids.as_array()? {
                if let Ok(kid) = kid.as_reference().and_then(move |id| self.get_dictionary(id)) {
                    self.get_named_destinations(kid, named_destinations)?;
                }
            }
        }
        if let Ok(names) = tree.get(b"Names") {
            let mut names = names.as_array()?.iter();
            while let (Some(key), Some(val)) = (names.next(), names.next()) {
                let key_bytes = match key.as_str() {
                    Ok(s) => s.to_vec(),
                    Err(_) => continue,
                };
                if let Ok(obj_ref) = val.as_reference() {
                    if let Ok(dict) = self.get_dictionary(obj_ref) {
                        if let Ok(arr) = dict.get(b"D").and_then(|d| d.as_array()) {
                            if arr.len() >= 2 {
                                let dest = Destination::new(key.clone(), arr[0].clone(), arr[1].clone());
                                named_destinations.insert(key_bytes, dest);
                            }
                        }
                    } else if let Ok(Object::Array(val)) = self.get_object(obj_ref) {
                        if val.len() >= 2 {
                            let dest = Destination::new(key.clone(), val[0].clone(), val[1].clone());
                            named_destinations.insert(key_bytes, dest);
                        }
                    }
                } else if let Ok(dict) = val.as_dict() {
                    if let Ok(arr) = dict.get(b"D").and_then(|d| d.as_array()) {
                        if arr.len() >= 2 {
                            let dest = Destination::new(key.clone(), arr[0].clone(), arr[1].clone());
                            named_destinations.insert(key_bytes, dest);
                        }
                    }
                }
                // Silently skip unexpected node types
            }
        }
        Ok(())
    }
}
