use std::fs::File;
use std::io::{Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use lopdf::Document;

fn bench_load_large(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/AnnotationDemo.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("load_large", |b| {
        b.iter(|| {
            Document::load_from(Cursor::new(&buffer)).unwrap();
        })
    });
}

fn bench_load_encrypted(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/encrypted.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("load_encrypted", |b| {
        b.iter(|| {
            let _ = Document::load_from(Cursor::new(&buffer));
        })
    });
}

criterion_group!(benches, bench_load_large, bench_load_encrypted);
criterion_main!(benches);
