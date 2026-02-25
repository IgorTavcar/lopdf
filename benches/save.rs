use std::fs::File;
use std::io::{Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use lopdf::{Document, SaveOptions};

fn bench_save_standard(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();

    c.bench_function("save_standard", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            let mut doc_clone = doc.clone();
            doc_clone.save_to(&mut output).unwrap();
        })
    });
}

fn bench_save_modern(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    let doc = Document::load_from(Cursor::new(&buffer)).unwrap();

    c.bench_function("save_modern", |b| {
        b.iter(|| {
            let mut output = Vec::new();
            let options = SaveOptions::builder()
                .use_xref_streams(true)
                .use_object_streams(true)
                .build();
            let mut doc_clone = doc.clone();
            doc_clone.save_with_options(&mut output, options).unwrap();
        })
    });
}

criterion_group!(benches, bench_save_standard, bench_save_modern);
criterion_main!(benches);
