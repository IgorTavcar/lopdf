use std::fs::File;
use std::io::{Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use lopdf::Document;

fn bench_load(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("load", |b| {
        b.iter(|| {
            Document::load_from(Cursor::new(&buffer)).unwrap();
        })
    });
}

fn bench_load_incremental_pdf(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/Incremental.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("load_incremental_pdf", |b| {
        b.iter(|| {
            Document::load_from(Cursor::new(&buffer)).unwrap();
        })
    });
}

criterion_group!(benches, bench_load, bench_load_incremental_pdf);
criterion_main!(benches);
