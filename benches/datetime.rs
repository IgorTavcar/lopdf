use criterion::{criterion_group, criterion_main, Criterion};

use chrono::prelude::{Local, Timelike};
use lopdf::Object;

fn create_and_parse_datetime(c: &mut Criterion) {
    c.bench_function("create_and_parse_datetime", |b| {
        b.iter(|| {
            let time = Local::now().with_nanosecond(0).unwrap();
            let text: Object = time.into();
            let time2 = text.as_datetime();
            assert!(time2.is_some());
        });
    });
}

fn bench_integer_write(c: &mut Criterion) {
    c.bench_function("integer_write", |b| {
        b.iter(|| {
            let mut buf = std::io::Cursor::new(Vec::<u8>::new());
            let mut doc = lopdf::Document::new();
            doc.add_object(Object::Integer(5));
            doc.save_to(&mut buf).unwrap();
        })
    });
}

fn bench_floating_point_write(c: &mut Criterion) {
    c.bench_function("floating_point_write", |b| {
        b.iter(|| {
            let mut buf = std::io::Cursor::new(Vec::<u8>::new());
            let mut doc = lopdf::Document::new();
            doc.add_object(Object::Real(5.0));
            doc.save_to(&mut buf).unwrap();
        })
    });
}

fn bench_boolean_write(c: &mut Criterion) {
    c.bench_function("boolean_write", |b| {
        b.iter(|| {
            let mut buf = std::io::Cursor::new(Vec::<u8>::new());
            let mut doc = lopdf::Document::new();
            doc.add_object(Object::Boolean(false));
            doc.save_to(&mut buf).unwrap();
        })
    });
}

criterion_group!(
    benches,
    create_and_parse_datetime,
    bench_integer_write,
    bench_floating_point_write,
    bench_boolean_write
);
criterion_main!(benches);
