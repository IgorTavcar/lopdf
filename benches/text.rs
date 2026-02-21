#![feature(test)]
use std::fs::File;
use std::io::{Cursor, Read};

extern crate test;
use lopdf::Document;

#[bench]
fn bench_extract_text(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();
    let pages: Vec<u32> = doc.get_pages().keys().cloned().collect();

    b.iter(|| {
        let _ = doc.extract_text(&pages);
    })
}

#[bench]
fn bench_text_replace(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    b.iter(|| {
        let mut doc = Document::load_from(Cursor::new(&buffer)).unwrap();
        let _ = doc.replace_text(1, "Hello World", "Replaced Text");
    })
}
