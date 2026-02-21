# lopdf

[![Crates.io](https://img.shields.io/crates/v/lopdf.svg)](https://crates.io/crates/lopdf)
[![CI](https://github.com/J-F-Liu/lopdf/actions/workflows/ci.yml/badge.svg)](https://github.com/J-F-Liu/lopdf/actions/workflows/ci.yml)
[![Docs]( https://docs.rs/lopdf/badge.svg)](https://docs.rs/lopdf)

A Rust library for PDF document manipulation. Read, modify, merge, create, encrypt, decrypt, and extract text from PDF files.

Useful references for working with PDF internals:
- [PDF 1.7 Reference](https://opensource.adobe.com/dc-acrobat-sdk-docs/pdfstandards/PDF32000_2008.pdf)
- [PDF 2.0 Specification](https://www.pdfa.org/announcing-no-cost-access-to-iso-32000-2-pdf-2-0/)

## Requirements

- **Rust 1.85+** (2024 edition)

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
lopdf = "0.39"
```

### Feature Flags

| Feature | Default | Description |
|---------|---------|-------------|
| `rayon` | Yes | Parallel object loading |
| `chrono` | Yes | Date/time parsing with chrono |
| `jiff` | Yes | Date/time parsing with jiff |
| `time` | Yes | Date/time parsing with time |
| `async` | No | Async I/O with tokio |
| `embed_image` | No | Image embedding support |
| `serde` | No | Serialization for TOC structures |
| `wasm_js` | No | WebAssembly support |

```toml
# Minimal (no date parsing, no parallel loading)
lopdf = { version = "0.39", default-features = false }

# With async support
lopdf = { version = "0.39", features = ["async"] }

# With image embedding
lopdf = { version = "0.39", features = ["embed_image"] }
```

---

## Quick Start

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    // Load a PDF
    let doc = Document::load("assets/example.pdf").unwrap();

    // Get page count
    let pages = doc.get_pages();
    println!("{} pages", pages.len());

    // Extract text
    let page_numbers: Vec<u32> = pages.keys().cloned().collect();
    let text = doc.extract_text(&page_numbers).unwrap();
    println!("{}", text);
}
```

---

## Examples

### Load and Inspect a PDF

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let doc = Document::load("assets/example.pdf").unwrap();

    println!("PDF version: {}", doc.version);
    println!("Pages: {}", doc.get_pages().len());

    // Iterate over all objects
    for (&(obj_num, gen_num), object) in &doc.objects {
        println!("Object ({}, {}): {}", obj_num, gen_num, object.enum_variant());
    }

    // Access the document catalog
    let catalog = doc.catalog().unwrap();
    println!("Catalog keys: {:?}", catalog.as_hashmap().keys().collect::<Vec<_>>());
}
```

### Load from Different Sources

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    // From file
    let doc = Document::load("assets/example.pdf").unwrap();

    // From memory
    let bytes = std::fs::read("assets/example.pdf").unwrap();
    let doc = Document::load_mem(&bytes).unwrap();

    // From any reader
    let file = std::fs::File::open("assets/example.pdf").unwrap();
    let doc = Document::load_from(file).unwrap();
}
```

### Extract Text from Specific Pages

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let doc = Document::load("assets/example.pdf").unwrap();

    // Extract from page 1 only
    let text = doc.extract_text(&[1]).unwrap();
    println!("Page 1: {}", text);

    // Extract from all pages
    let page_numbers: Vec<u32> = doc.get_pages().keys().cloned().collect();
    let text = doc.extract_text(&page_numbers).unwrap();

    // For multi-page PDFs you can pick specific pages:
    // let text = doc.extract_text(&[1, 3, 5]).unwrap();
}
```

### Extract Metadata (Fast)

Extract title, author, page count, and other metadata without loading the entire document. This is much faster for large PDFs.

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let metadata = Document::load_metadata("assets/example.pdf").unwrap();

    println!("Title: {:?}", metadata.title);
    println!("Author: {:?}", metadata.author);
    println!("Subject: {:?}", metadata.subject);
    println!("Creator: {:?}", metadata.creator);
    println!("Producer: {:?}", metadata.producer);
    println!("Pages: {}", metadata.page_count);
    println!("Version: {}", metadata.version);
    println!("Created: {:?}", metadata.creation_date);
    println!("Modified: {:?}", metadata.modification_date);

    // Also works from memory
    let bytes = std::fs::read("assets/example.pdf").unwrap();
    let metadata = Document::load_metadata_mem(&bytes).unwrap();
}
```

### Modify an Existing PDF

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let mut doc = Document::load("assets/example.pdf").unwrap();

    // Change PDF version
    doc.version = "1.4".to_string();

    // Replace text on page 1
    doc.replace_text(1, "Hello World!", "Modified text!", None);

    // Replace partial text matches
    let count = doc.replace_partial_text(1, "Hello", "Hi", None).unwrap();
    println!("Replaced {} occurrences", count);

    // Save
    if false { // excluded from doctest
        doc.save("modified.pdf").unwrap();
    }
}
```

### Rotate Pages

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let mut doc = Document::load("assets/example.pdf").unwrap();

    for (_, page_id) in doc.get_pages() {
        let page_dict = doc.get_object_mut(page_id)
            .and_then(|obj| obj.as_dict_mut())
            .unwrap();

        let current = page_dict.get(b"Rotate")
            .and_then(|obj| obj.as_i64())
            .unwrap_or(0);

        page_dict.set("Rotate", (current + 90) % 360);
    }

    if false { doc.save("rotated.pdf").unwrap(); }
}
```

### Delete Pages

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let mut doc = Document::load("assets/example.pdf").unwrap();

    // Delete page 1 (by page object ID, not page number)
    let pages = doc.get_pages();
    if let Some(&page_id) = pages.get(&1) {
        doc.remove_object(&page_id).ok();
    }
}
```

### Work with Page Content Streams

```rust
use lopdf::Document;
use lopdf::content::{Content, Operation};

#[cfg(not(feature = "async"))]
{
    let doc = Document::load("assets/example.pdf").unwrap();
    let pages = doc.get_pages();
    let page_id = *pages.get(&1).unwrap();

    // Get raw content stream
    let content_bytes = doc.get_page_content(page_id).unwrap();

    // Decode into structured operations
    let content = Content::decode(&content_bytes).unwrap();

    for operation in &content.operations {
        println!("{} {:?}", operation.operator, operation.operands);
    }
}
```

### Work with Annotations

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    let doc = Document::load("assets/AnnotationDemo.pdf").unwrap();

    for (page_num, page_id) in doc.get_pages() {
        if let Ok(annotations) = doc.get_page_annotations(page_id) {
            println!("Page {} has {} annotations", page_num, annotations.len());
            for annot in annotations {
                if let Ok(subtype) = annot.get(b"Subtype") {
                    println!("  Type: {:?}", subtype);
                }
            }
        }
    }
}
```

### Create a PDF from Scratch

```rust
use lopdf::dictionary;
use lopdf::{Document, Object, Stream};
use lopdf::content::{Content, Operation};

let mut doc = Document::with_version("1.5");
let pages_id = doc.new_object_id();

// Add a font
let font_id = doc.add_object(dictionary! {
    "Type" => "Font",
    "Subtype" => "Type1",
    "BaseFont" => "Courier",
});

let resources_id = doc.add_object(dictionary! {
    "Font" => dictionary! {
        "F1" => font_id,
    },
});

// Create page content
let content = Content {
    operations: vec![
        Operation::new("BT", vec![]),
        Operation::new("Tf", vec!["F1".into(), 48.into()]),
        Operation::new("Td", vec![100.into(), 600.into()]),
        Operation::new("Tj", vec![Object::string_literal("Hello World!")]),
        Operation::new("ET", vec![]),
    ],
};

let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));

let page_id = doc.add_object(dictionary! {
    "Type" => "Page",
    "Parent" => pages_id,
    "Contents" => content_id,
});

let pages = dictionary! {
    "Type" => "Pages",
    "Kids" => vec![page_id.into()],
    "Count" => 1,
    "Resources" => resources_id,
    "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
};

doc.objects.insert(pages_id, Object::Dictionary(pages));

let catalog_id = doc.add_object(dictionary! {
    "Type" => "Catalog",
    "Pages" => pages_id,
});

doc.trailer.set("Root", catalog_id);
doc.compress();

if false { // excluded from doctest
    // Traditional save
    doc.save("example.pdf").unwrap();

    // Or save with modern compression (object streams + xref streams)
    let mut file = std::fs::File::create("example_compressed.pdf").unwrap();
    doc.save_modern(&mut file).unwrap();
}
```

### Merge Multiple PDFs

```rust
use lopdf::dictionary;
use std::collections::BTreeMap;
use lopdf::content::{Content, Operation};
use lopdf::{Document, Object, ObjectId, Stream, Bookmark};

pub fn generate_fake_document() -> Document {
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.new_object_id();
    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Courier",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });
    let content = Content {
        operations: vec![
            Operation::new("BT", vec![]),
            Operation::new("Tf", vec!["F1".into(), 48.into()]),
            Operation::new("Td", vec![100.into(), 600.into()]),
            Operation::new("Tj", vec![Object::string_literal("Hello World!")]),
            Operation::new("ET", vec![]),
        ],
    };
    let content_id = doc.add_object(Stream::new(dictionary! {}, content.encode().unwrap()));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Contents" => content_id,
        "Resources" => resources_id,
        "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
    });
    let pages = dictionary! {
        "Type" => "Pages",
        "Kids" => vec![page_id.into()],
        "Count" => 1,
    };
    doc.objects.insert(pages_id, Object::Dictionary(pages));
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);

    doc
}

fn main() -> std::io::Result<()> {
    let documents = vec![
        generate_fake_document(),
        generate_fake_document(),
        generate_fake_document(),
        generate_fake_document(),
    ];

    let mut max_id = 1;
    let mut pagenum = 1;
    let mut documents_pages = BTreeMap::new();
    let mut documents_objects = BTreeMap::new();
    let mut document = Document::with_version("1.5");

    for mut doc in documents {
        let mut first = false;
        doc.renumber_objects_with(max_id);
        max_id = doc.max_id + 1;

        documents_pages.extend(
            doc.get_pages()
                .into_iter()
                .map(|(_, object_id)| {
                    if !first {
                        let bookmark = Bookmark::new(
                            String::from(format!("Page_{}", pagenum)),
                            [0.0, 0.0, 1.0], 0, object_id,
                        );
                        document.add_bookmark(bookmark, None);
                        first = true;
                        pagenum += 1;
                    }
                    (object_id, doc.get_object(object_id).unwrap().to_owned())
                })
                .collect::<BTreeMap<ObjectId, Object>>(),
        );
        documents_objects.extend(doc.objects);
    }

    let mut catalog_object: Option<(ObjectId, Object)> = None;
    let mut pages_object: Option<(ObjectId, Object)> = None;

    for (object_id, object) in documents_objects.iter() {
        match object.type_name().unwrap_or(b"") {
            b"Catalog" => {
                catalog_object = Some((
                    if let Some((id, _)) = catalog_object { id } else { *object_id },
                    object.clone(),
                ));
            }
            b"Pages" => {
                if let Ok(dictionary) = object.as_dict() {
                    let mut dictionary = dictionary.clone();
                    if let Some((_, ref object)) = pages_object {
                        if let Ok(old_dictionary) = object.as_dict() {
                            dictionary.extend(old_dictionary);
                        }
                    }
                    pages_object = Some((
                        if let Some((id, _)) = pages_object { id } else { *object_id },
                        Object::Dictionary(dictionary),
                    ));
                }
            }
            b"Page" | b"Outlines" | b"Outline" => {}
            _ => { document.objects.insert(*object_id, object.clone()); }
        }
    }

    if pages_object.is_none() || catalog_object.is_none() {
        println!("Required objects not found.");
        return Ok(());
    }

    for (object_id, object) in documents_pages.iter() {
        if let Ok(dictionary) = object.as_dict() {
            let mut dictionary = dictionary.clone();
            dictionary.set("Parent", pages_object.as_ref().unwrap().0);
            document.objects.insert(*object_id, Object::Dictionary(dictionary));
        }
    }

    let catalog_object = catalog_object.unwrap();
    let pages_object = pages_object.unwrap();

    if let Ok(dictionary) = pages_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Count", documents_pages.len() as u32);
        dictionary.set(
            "Kids",
            documents_pages.into_iter()
                .map(|(object_id, _)| Object::Reference(object_id))
                .collect::<Vec<_>>(),
        );
        document.objects.insert(pages_object.0, Object::Dictionary(dictionary));
    }

    if let Ok(dictionary) = catalog_object.1.as_dict() {
        let mut dictionary = dictionary.clone();
        dictionary.set("Pages", pages_object.0);
        dictionary.remove(b"Outlines");
        document.objects.insert(catalog_object.0, Object::Dictionary(dictionary));
    }

    document.trailer.set("Root", catalog_object.0);
    document.max_id = document.objects.len() as u32;
    document.renumber_objects();
    document.adjust_zero_pages();

    if let Some(n) = document.build_outline() {
        if let Ok(Object::Dictionary(dict)) = document.get_object_mut(catalog_object.0) {
            dict.set("Outlines", Object::Reference(n));
        }
    }

    document.compress();

    if false { // excluded from doctest
        document.save("merged.pdf").unwrap();
    }

    Ok(())
}
```

### Encrypted PDFs

lopdf supports reading and writing encrypted PDFs using RC4, AES-128, and AES-256 encryption.

#### Load an Encrypted PDF

```rust
use lopdf::Document;

#[cfg(not(feature = "async"))]
{
    // Automatic decryption with empty password
    let doc = Document::load("assets/encrypted.pdf").unwrap();

    // Load with a specific password
    // let doc = Document::load_with_password("protected.pdf", "secret").unwrap();

    // Check if it was originally encrypted
    if doc.was_encrypted() {
        println!("Document was encrypted, now decrypted");
        let pages = doc.get_pages();
        let page_numbers: Vec<u32> = pages.keys().cloned().collect();
        let text = doc.extract_text(&page_numbers).unwrap();
        println!("Extracted {} chars", text.len());
    }

    // Load from memory with password
    let bytes = std::fs::read("assets/encrypted.pdf").unwrap();
    let doc = Document::load_mem(&bytes).unwrap();
    // let doc = Document::load_mem_with_password(&bytes, "secret").unwrap();

    // Metadata from encrypted PDFs (fast, no full load)
    // let meta = Document::load_metadata_with_password("protected.pdf", "secret").unwrap();
}
```

#### Encrypt a PDF

```rust,no_run
use lopdf::{Document, EncryptionState, EncryptionVersion, Permissions};

fn main() {
    let mut doc = Document::load("input.pdf").unwrap();

    let permissions = Permissions::PRINTABLE
        | Permissions::COPYABLE
        | Permissions::COPYABLE_FOR_ACCESSIBILITY
        | Permissions::PRINTABLE_IN_HIGH_QUALITY;

    // RC4 40-bit encryption (V1)
    let version = EncryptionVersion::V1 {
        document: &doc,
        owner_password: "owner_pass",
        user_password: "user_pass",
        permissions,
    };

    let state = EncryptionState::try_from(version).unwrap();
    doc.encrypt(&state).unwrap();
    doc.save("encrypted.pdf").unwrap();
}
```

#### Decrypt a PDF

```rust,no_run
use lopdf::Document;

fn main() {
    let mut doc = Document::load("encrypted.pdf").unwrap();

    if doc.is_encrypted() {
        doc.decrypt("password").unwrap();
        doc.save("decrypted.pdf").unwrap();
    }
}
```

### Save with Object Streams (Modern Format)

Object streams (PDF 1.5+) compress multiple objects together, reducing file size by 11-61%.

```rust
use lopdf::{Document, SaveOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut doc = Document::with_version("1.5");
    // ... add content ...

    // Quick modern save (recommended for most cases)
    let mut file = std::fs::File::create("/dev/null")?;
    doc.save_modern(&mut file)?;

    // Fine-grained control
    let options = SaveOptions::builder()
        .use_object_streams(true)       // Compress objects together
        .use_xref_streams(true)         // Binary cross-reference streams
        .max_objects_per_stream(200)    // Default: 100
        .compression_level(9)           // 0-9, default: 6
        .build();

    let mut file2 = std::fs::File::create("/dev/null")?;
    doc.save_with_options(&mut file2, options)?;

    Ok(())
}
```

### Create Object Streams Directly

```rust
use lopdf::{Object, ObjectStream, dictionary};

# fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut obj_stream = ObjectStream::builder()
        .max_objects(100)
        .compression_level(6)
        .build();

    obj_stream.add_object((1, 0), Object::Integer(42))?;
    obj_stream.add_object((2, 0), Object::Name(b"Example".to_vec()))?;
    obj_stream.add_object((3, 0), Object::Dictionary(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica"
    }))?;

    let stream = obj_stream.to_stream_object()?;
    # Ok::<(), Box<dyn std::error::Error>>(())
# }
```

### Embed a TrueType Font

```rust,no_run
use lopdf::{Document, FontData};

fn main() {
    let mut doc = Document::with_version("1.5");

    let font_bytes = std::fs::read("font.ttf").unwrap();
    let font_data = FontData::new(&font_bytes, "MyFont".to_string());
    let font_id = doc.add_font(font_data).unwrap();

    // Use font_id in your page resources
}
```

### Incremental Updates

Append changes to an existing PDF without rewriting the entire file.

```rust
use lopdf::IncrementalDocument;

#[cfg(not(feature = "async"))]
{
    let mut inc_doc = IncrementalDocument::load("assets/example.pdf").unwrap();

    // Modify through new_document
    // inc_doc.new_document.set_object(id, new_object);

    let prev = inc_doc.get_prev_documents();
    println!("Previous version: {}", prev.version);
}
```

### Filter Objects During Loading

Load only the objects you need for faster processing:

```rust
use lopdf::{Document, Object};

#[cfg(not(feature = "async"))]
{
    fn filter(object_id: (u32, u16), object: &mut Object) -> Option<((u32, u16), Object)> {
        // Skip image and XObject streams to load faster
        if let Ok(dict) = object.as_dict() {
            if dict.has_type(b"XObject") {
                return None;
            }
        }
        Some((object_id, object.clone()))
    }

    let doc = Document::load_filtered("assets/example.pdf", filter).unwrap();
    println!("Loaded {} objects (images excluded)", doc.objects.len());
}
```

### Async Loading

```rust,no_run
// Enable with: lopdf = { version = "0.39", features = ["async"] }

#[cfg(feature = "async")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use lopdf::Document;

    let doc = Document::load("input.pdf").await?;
    let doc = Document::load_with_password("encrypted.pdf", "pass").await?;
    let meta = Document::load_metadata("input.pdf").await?;

    Ok(())
}

#[cfg(not(feature = "async"))]
fn main() {}
```

---

## API Overview

### Document

| Method | Description |
|--------|-------------|
| `Document::load(path)` | Load from file |
| `Document::load_with_password(path, pw)` | Load encrypted PDF |
| `Document::load_mem(bytes)` | Load from memory |
| `Document::load_from(reader)` | Load from any `Read` |
| `Document::load_filtered(path, filter)` | Load with object filter |
| `Document::load_metadata(path)` | Fast metadata extraction |
| `Document::with_version(ver)` | Create new PDF |
| `doc.save(path)` | Save to file |
| `doc.save_to(writer)` | Save to writer |
| `doc.save_modern(writer)` | Save with object/xref streams |
| `doc.save_with_options(writer, opts)` | Save with custom options |
| `doc.get_pages()` | Get page number to ID mapping |
| `doc.page_iter()` | Iterate page IDs |
| `doc.extract_text(pages)` | Extract text from pages |
| `doc.replace_text(page, old, new, font)` | Replace text on a page |
| `doc.replace_partial_text(page, old, new, font)` | Replace partial text matches |
| `doc.get_page_content(page_id)` | Get decompressed page content |
| `doc.get_page_fonts(page_id)` | Get fonts used on a page |
| `doc.get_page_annotations(page_id)` | Get page annotations |
| `doc.get_page_images(page_id)` | Extract images from a page |
| `doc.get_toc()` | Extract table of contents |
| `doc.catalog()` | Access document catalog |
| `doc.get_object(id)` | Get object by ID |
| `doc.add_object(obj)` | Add object, returns ID |
| `doc.compress()` | Compress all streams |
| `doc.encrypt(state)` | Encrypt the document |
| `doc.decrypt(password)` | Decrypt the document |
| `doc.is_encrypted()` | Check if encrypted |
| `doc.was_encrypted()` | Check if was originally encrypted |
| `doc.add_bookmark(bookmark, parent)` | Add a bookmark |
| `doc.add_font(font_data)` | Embed a TrueType font |
| `doc.renumber_objects()` | Renumber all object IDs |

### Object

PDF objects are represented by the `Object` enum:

```rust,ignore
Object::Null
Object::Boolean(bool)
Object::Integer(i64)
Object::Real(f32)
Object::Name(Vec<u8>)
Object::String(Vec<u8>, StringFormat)
Object::Array(Vec<Object>)
Object::Dictionary(Dictionary)
Object::Stream(Stream)
Object::Reference(ObjectId)   // (u32, u16)
```

Type conversions: `as_bool()`, `as_i64()`, `as_f32()`, `as_name()`, `as_str()`, `as_reference()`, `as_array()`, `as_dict()`, `as_stream()`, and mutable variants.

Many Rust types convert to `Object` via `Into`: `bool`, `i64`, `f64`, `&str`, `String`, `Vec<Object>`, `Dictionary`, `Stream`, `ObjectId`.

### Dictionary

```rust,ignore
let mut dict = lopdf::dictionary! {
    "Type" => "Page",
    "Parent" => parent_id,
    "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
};

dict.set("Rotate", 90);
dict.has(b"Type");          // true
dict.get(b"Type");          // Ok(&Object)
dict.remove(b"Rotate");     // Some(Object)
```

### Stream

```rust,ignore
let stream = Stream::new(dictionary! {}, content_bytes);

stream.compress()?;                    // Compress with FlateDecode
let plain = stream.get_plain_content()?; // Decompress
stream.filters()?;                     // Get filter names
```

### PdfMetadata

```rust,ignore
pub struct PdfMetadata {
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub creator: Option<String>,
    pub producer: Option<String>,
    pub creation_date: Option<String>,
    pub modification_date: Option<String>,
    pub page_count: u32,
    pub version: String,
}
```

### SaveOptions

```rust,ignore
SaveOptions::builder()
    .use_object_streams(true)       // default: false
    .use_xref_streams(true)         // default: false
    .max_objects_per_stream(200)    // default: 100
    .compression_level(9)           // 0-9, default: 6
    .build()
```

### Permissions (for encryption)

```rust,ignore
use lopdf::Permissions;

let perms = Permissions::PRINTABLE
    | Permissions::COPYABLE
    | Permissions::ANNOTABLE
    | Permissions::MODIFIABLE
    | Permissions::FILLABLE
    | Permissions::COPYABLE_FOR_ACCESSIBILITY
    | Permissions::ASSEMBLABLE
    | Permissions::PRINTABLE_IN_HIGH_QUALITY;
```

---

## FAQ

**Why does the library keep everything in memory?**

Most PDFs range from tens of KB to hundreds of MB. Keeping the whole document in memory allows stream lengths to be pre-calculated, producing smaller files that are faster for PDF consumers to process.

**What PDF versions support object streams?**

Object streams were introduced in PDF 1.5. When using `save_modern()`, lopdf ensures the version is at least 1.5. For compatibility with older readers, use `save()`.

**How do object streams affect memory usage?**

Object streams reduce memory during save by grouping and compressing small objects together. The in-memory representation stays the same until `save_modern()` or `save_with_options()` is called.

**Can I read PDFs that already use object streams?**

Yes. `Document::load()` automatically handles object streams in existing PDFs.

## License

lopdf is available under the MIT license, with the exception of the Montserrat font.
