#![feature(test)]
use std::fs::File;
use std::io::{Cursor, Read};

extern crate test;
use lopdf::{Document, SaveOptions};

#[bench]
fn bench_save_standard(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();

    b.iter(|| {
        let mut output = Vec::new();
        let mut doc_clone = doc.clone();
        doc_clone.save_to(&mut output).unwrap();
    })
}

#[bench]
fn bench_save_modern(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();

    b.iter(|| {
        let mut output = Vec::new();
        let options = SaveOptions::builder()
            .use_xref_streams(true)
            .use_object_streams(true)
            .build();
        let mut doc_clone = doc.clone();
        doc_clone.save_with_options(&mut output, options).unwrap();
    })
}
