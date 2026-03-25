//! 会话存储性能基准测试

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use bee::domain::session::store::{InMemorySessionStore, SessionStore};
use bee::infrastructure::session::SqliteSessionStore;
use bee::domain::session::{SessionConfig, SessionId};
use tokio::runtime::Runtime;

fn bench_session_store_create(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("session_store_create");
    group.throughput(Throughput::Elements(1));

    // InMemorySessionStore 基准测试
    group.bench_function("in_memory_create", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemorySessionStore::new();
                let config = SessionConfig::new().with_system_prompt("test");
                let _: SessionId = store.create(config).await.unwrap();
            })
        });
    });

    // SqliteSessionStore 基准测试
    group.bench_function("sqlite_create", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteSessionStore::in_memory().unwrap();
                let config = SessionConfig::new().with_system_prompt("test");
                let _: SessionId = store.create(config).await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_session_store_get(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("session_store_get");
    group.throughput(Throughput::Elements(1));

    // InMemorySessionStore 基准测试
    group.bench_function("in_memory_get", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemorySessionStore::new();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let _ = store.get(&id).await.unwrap();
            })
        });
    });

    // SqliteSessionStore 基准测试
    group.bench_function("sqlite_get", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteSessionStore::in_memory().unwrap();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let _ = store.get(&id).await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_session_store_update(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("session_store_update");
    group.throughput(Throughput::Elements(1));

    // InMemorySessionStore 基准测试
    group.bench_function("in_memory_update", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemorySessionStore::new();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let mut session = store.get(&id).await.unwrap().unwrap();
                session.config.max_turns = 100;
                let _: () = store.update(session).await.unwrap();
            })
        });
    });

    // SqliteSessionStore 基准测试
    group.bench_function("sqlite_update", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteSessionStore::in_memory().unwrap();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let mut session = store.get(&id).await.unwrap().unwrap();
                session.config.max_turns = 100;
                let _: () = store.update(session).await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_session_store_list(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("session_store_list");

    // 不同会话数量的基准测试
    for &size in &[10, 50, 100] {
        group.bench_with_input(BenchmarkId::new("in_memory_list", size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let store = InMemorySessionStore::new();
                    for i in 0..size {
                        let config = SessionConfig::new().with_system_prompt(&format!("session{}", i));
                        let _: SessionId = store.create(config).await.unwrap();
                    }
                    let _: Vec<SessionId> = store.list().await.unwrap();
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("sqlite_list", size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let store = SqliteSessionStore::in_memory().unwrap();
                    for i in 0..size {
                        let config = SessionConfig::new().with_system_prompt(&format!("session{}", i));
                        let _: SessionId = store.create(config).await.unwrap();
                    }
                    let _: Vec<SessionId> = store.list().await.unwrap();
                })
            });
        });
    }

    group.finish();
}

fn bench_session_store_delete(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("session_store_delete");
    group.throughput(Throughput::Elements(1));

    group.bench_function("in_memory_delete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemorySessionStore::new();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let _: () = store.delete(&id).await.unwrap();
            })
        });
    });

    group.bench_function("sqlite_delete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteSessionStore::in_memory().unwrap();
                let config = SessionConfig::new();
                let id = store.create(config).await.unwrap();
                let _: () = store.delete(&id).await.unwrap();
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_session_store_create,
    bench_session_store_get,
    bench_session_store_update,
    bench_session_store_list,
    bench_session_store_delete,
);

criterion_main!(benches);
