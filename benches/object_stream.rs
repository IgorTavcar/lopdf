use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Read};

use criterion::{criterion_group, criterion_main, Criterion};
use lopdf::{dictionary, Document, Object, ObjectStream};

fn bench_object_stream_compress(c: &mut Criterion) {
    // Create 100 simple dictionary objects
    let mut objects: BTreeMap<(u32, u16), Object> = BTreeMap::new();
    for i in 1..=100 {
        let dict = lopdf::dictionary! {
            "Type" => "TestObj",
            "Value" => Object::Integer(i as i64),
        };
        objects.insert((i, 0), Object::Dictionary(dict));
    }

    c.bench_function("object_stream_compress", |b| {
        b.iter(|| {
            let mut stream = ObjectStream::builder().build();
            for (&id, obj) in &objects {
                stream.add_object(id, obj.clone()).unwrap();
            }
            let _ = stream.to_stream_object();
        })
    });
}

fn bench_object_stream_parse(c: &mut Criterion) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    c.bench_function("object_stream_parse", |b| {
        b.iter(|| {
            let _ = Document::load_from(Cursor::new(&buffer));
        })
    });
}

criterion_group!(
    benches,
    bench_object_stream_compress,
    bench_object_stream_parse
);
criterion_main!(benches);
