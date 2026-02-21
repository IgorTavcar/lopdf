use super::*;
use crate::Document;

#[cfg(not(feature = "async"))]
#[test]
fn load_document() {
    let mut doc = Document::load("assets/example.pdf").unwrap();
    assert_eq!(doc.version, "1.5");

    // Create temporary folder to store file.
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test_2_load.pdf");
    doc.save(file_path).unwrap();
}

#[cfg(feature = "async")]
#[tokio::test]
async fn load_document() {
    let mut doc = Document::load("assets/example.pdf").await.unwrap();
    assert_eq!(doc.version, "1.5");

    // Create temporary folder to store file.
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("test_2_load.pdf");
    doc.save(file_path).unwrap();
}

#[test]
#[should_panic(expected = "Xref(Start)")]
fn load_short_document() {
    let _doc = Document::load_mem(b"%PDF-1.5\n%%EOF\n").unwrap();
}

#[test]
fn load_document_with_preceding_bytes() {
    let mut content = Vec::new();
    content.extend(b"garbage");
    content.extend(include_bytes!("../../assets/example.pdf"));
    let doc = Document::load_mem(&content).unwrap();
    assert_eq!(doc.version, "1.5");
}

#[test]
fn load_many_shallow_brackets() {
    let content: String = std::iter::repeat("()")
        .take(MAX_BRACKET * 10)
        .flat_map(|x| x.chars())
        .collect();
    const STREAM_CRUFT: usize = 33;
    let doc = format!(
        "%PDF-1.5
1 0 obj<</Type/Pages/Kids[5 0 R]/Count 1/Resources 3 0 R/MediaBox[0 0 595 842]>>endobj
2 0 obj<</Type/Font/Subtype/Type1/BaseFont/Courier>>endobj
3 0 obj<</Font<</F1 2 0 R>>>>endobj
5 0 obj<</Type/Page/Parent 1 0 R/Contents[4 0 R]>>endobj
6 0 obj<</Type/Catalog/Pages 1 0 R>>endobj
4 0 obj<</Length {}>>stream
BT
/F1 48 Tf
100 600 Td
({}) Tj
ET
endstream endobj\n",
        content.len() + STREAM_CRUFT,
        content
    );
    let doc = format!(
        "{}xref\n0 7\n0000000000 65535 f \n0000000009 00000 n \n0000000096 00000 n \n0000000155 00000 n \n0000000291 00000 n \n0000000191 00000 n \n0000000248 00000 n \ntrailer\n<</Root 6 0 R/Size 7>>\nstartxref\n{}\n%%EOF",
        doc,
        doc.len()
    );

    let _doc = Document::load_mem(doc.as_bytes()).unwrap();
}

#[test]
fn load_too_deep_brackets() {
    let content: Vec<u8> = std::iter::repeat(b'(')
        .take(MAX_BRACKET + 1)
        .chain(std::iter::repeat(b')').take(MAX_BRACKET + 1))
        .collect();
    let content = String::from_utf8(content).unwrap();
    const STREAM_CRUFT: usize = 33;
    let doc = format!(
        "%PDF-1.5
1 0 obj<</Type/Pages/Kids[5 0 R]/Count 1/Resources 3 0 R/MediaBox[0 0 595 842]>>endobj
2 0 obj<</Type/Font/Subtype/Type1/BaseFont/Courier>>endobj
3 0 obj<</Font<</F1 2 0 R>>>>endobj
5 0 obj<</Type/Page/Parent 1 0 R/Contents[7 0 R 4 0 R]>>endobj
6 0 obj<</Type/Catalog/Pages 1 0 R>>endobj
7 0 obj<</Length 45>>stream
BT /F1 48 Tf 100 600 Td (Hello World!) Tj ET
endstream
endobj
4 0 obj<</Length {}>>stream
BT
/F1 48 Tf
100 600 Td
({}) Tj
ET
endstream endobj\n",
        content.len() + STREAM_CRUFT,
        content
    );
    let doc = format!(
        "{}xref\n0 7\n0000000000 65535 f \n0000000009 00000 n \n0000000096 00000 n \n0000000155 00000 n \n0000000387 00000 n \n0000000191 00000 n \n0000000254 00000 n \n0000000297 00000 n \ntrailer\n<</Root 6 0 R/Size 7>>\nstartxref\n{}\n%%EOF",
        doc,
        doc.len()
    );

    let doc = Document::load_mem(doc.as_bytes()).unwrap();
    let pages = doc.get_pages().keys().cloned().collect::<Vec<_>>();
    assert_eq!("Hello World!\n", doc.extract_text(&pages).unwrap());
}

#[cfg(not(feature = "async"))]
#[test]
fn search_substring_finds_last_occurrence() {
    assert_eq!(Reader::search_substring(b"hello world", b"xyz", 0), None);
    assert_eq!(Reader::search_substring(b"hello world", b"world", 0), Some(6));

    let buffer = b"%%EOF\ntest%%EOF\nend";
    assert_eq!(Reader::search_substring(buffer, b"%%EOF", 0), Some(10));
    assert_eq!(Reader::search_substring(buffer, b"%%EOF", 6), Some(10));
    assert_eq!(Reader::search_substring(buffer, b"%%EOF", 15), None);
    assert_eq!(Reader::search_substring(b"%%EOF", b"%%EOF", 0), Some(0));

    let buffer_with_many_percents = b"%%%PDF-1.3%%%comment%%%more%%EOF";
    assert_eq!(
        Reader::search_substring(buffer_with_many_percents, b"%%EOF", 0),
        Some(27)
    );
}
