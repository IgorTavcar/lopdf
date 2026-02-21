#![feature(test)]
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Cursor, Read};

extern crate test;
use lopdf::{Document, Object, ObjectStreamBuilder, ObjectStreamConfig};

#[bench]
fn bench_object_stream_compress(b: &mut test::test::Bencher) {
    // Create 100 simple dictionary objects
    let mut objects: BTreeMap<(u32, u16), Object> = BTreeMap::new();
    for i in 1..=100 {
        let dict = lopdf::dictionary! {
            "Type" => "TestObj",
            "Value" => Object::Integer(i as i64),
        };
        objects.insert((i, 0), Object::Dictionary(dict));
    }

    b.iter(|| {
        let config = ObjectStreamConfig::default();
        let builder = ObjectStreamBuilder::new(config);
        let _ = builder.build_object_streams(&objects);
    })
}

#[bench]
fn bench_object_stream_parse(b: &mut test::test::Bencher) {
    let mut buffer = Vec::new();
    File::open("assets/example.pdf")
        .unwrap()
        .read_to_end(&mut buffer)
        .unwrap();

    // Benchmark parsing a PDF that may contain object streams
    b.iter(|| {
        let _ = Document::load_from(Cursor::new(&buffer));
    })
}
