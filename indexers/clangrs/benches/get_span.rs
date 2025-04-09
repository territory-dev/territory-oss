use std::hint::black_box;

use testdir::testdir;
use criterion::{criterion_group, criterion_main, Criterion};

use territory_core::{NodeKind, PathID};
use clangrs::intermediate_model::{LocalSpanIndex, sqlite};


fn sqlite_span_store(c: &mut Criterion) {
    const NODES_PER_FILE: u32 = 1000;
    const FILES: u32 = 100;

    let db_path = testdir!().join("db");
    let sqlite_srevices: sqlite::SqliteServices<
        sqlite::SqliteGSMReader, sqlite::SqliteUMQuery
    > = sqlite::new(&db_path);
    sqlite_srevices.create_tables();

    let mut spans = sqlite_srevices.span_store;

    for pi in 0..FILES {
        for i in 0..NODES_PER_FILE {
            let path_id = PathID(pi);
            let node_kind = NodeKind::Definition;
            let start = i * 10;
            let end = start + 10;
            let nest_level = 1;

            let _node_id = spans.store_one(path_id, node_kind, start, end, nest_level).unwrap();
        }
    }

    c.bench_function("get from SpanStore", |b| b.iter(|| {
        let path_id = PathID(5);
        let offset = black_box(NODES_PER_FILE);

        spans.get(path_id, offset);
    }));
}

fn local_span_index(c: &mut Criterion) {
    const NODES_PER_FILE: u32 = 1000;
    const FILES: u32 = 100;


    let mut span_vec = Vec::new();
    for pi in 0..FILES {
        for i in 0..NODES_PER_FILE {
            let path_id = PathID(pi);
            let start = i * 10;
            let end = start + 10;

            span_vec.push((path_id, start, end, (pi * NODES_PER_FILE + i).into()));
        }
    }
    let spans = LocalSpanIndex::from(vec![span_vec]);

    c.bench_function("get from LocalSpanIndex", |b| b.iter(|| {
        let path_id = PathID(5);
        let offset = black_box(NODES_PER_FILE);

        spans.get(path_id, offset);
    }));
}

criterion_group!(benches, sqlite_span_store, local_span_index);
criterion_main!(benches);
