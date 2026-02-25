use std::fs::File;
use std::io::{Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use lopdf::Document;

fn bench_extract_text(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();
    let pages: Vec<u32> = doc.get_pages().keys().cloned().collect();

    c.bench_function("extract_text", |b| {
        b.iter(|| {
            let _ = doc.extract_text(&pages);
        })
    });
}

fn bench_text_replace(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("text_replace", |b| {
        b.iter(|| {
            let mut doc = Document::load_from(Cursor::new(&buffer)).unwrap();
            let _ = doc.replace_text(1, "Hello World", "Replaced Text", None);
        })
    });
}

criterion_group!(benches, bench_extract_text, bench_text_replace);
criterion_main!(benches);
