#![feature(test)]
use std::fs::File;
use std::io::{Cursor, Read};

extern crate test;
use lopdf::Document;

#[bench]
fn bench_load_large(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/AnnotationDemo.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    b.iter(|| {
        Document::load_from(Cursor::new(&buffer)).unwrap();
    })
}

#[bench]
fn bench_load_encrypted(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/encrypted.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    b.iter(|| {
        let _ = Document::load_from(Cursor::new(&buffer));
    })
}
