//! 记忆存储性能基准测试

use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use bee::domain::memory::store::MemoryStore;
use bee::infrastructure::memory::{InMemoryStore, SqliteMemoryStore};
use bee::memory::Message;
use tokio::runtime::Runtime;

fn bench_memory_store_append(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_store_append");
    group.throughput(Throughput::Elements(1));

    // InMemoryStore 基准测试
    group.bench_function("in_memory_append", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryStore::new();
                let _: () = store.append("conv1", &Message::user("test message")).await.unwrap();
            })
        });
    });

    // SqliteMemoryStore 基准测试
    group.bench_function("sqlite_append", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteMemoryStore::in_memory().unwrap();
                let _: () = store.append("conv1", &Message::user("test message")).await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_memory_store_load(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_store_load");

    // 不同消息数量的基准测试
    for &size in &[10, 100, 1000] {
        group.bench_with_input(BenchmarkId::new("in_memory_load", size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let store = InMemoryStore::new();
                    for i in 0..size {
                        let _: () = store.append("conv1", &Message::user(&format!("msg{}", i))).await.unwrap();
                    }
                    let _: Vec<Message> = store.load("conv1", 0).await.unwrap();
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("sqlite_load", size), &size, |b, &size| {
            b.iter(|| {
                rt.block_on(async {
                    let store = SqliteMemoryStore::in_memory().unwrap();
                    for i in 0..size {
                        let _: () = store.append("conv1", &Message::user(&format!("msg{}", i))).await.unwrap();
                    }
                    let _: Vec<Message> = store.load("conv1", 0).await.unwrap();
                })
            });
        });
    }

    group.finish();
}

fn bench_memory_store_load_with_limit(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_store_load_with_limit");

    let size = 1000;

    // InMemoryStore 带限制的加载
    group.bench_with_input(BenchmarkId::new("in_memory_limit", size), &size, |b, &size| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryStore::new();
                for i in 0..size {
                    let _: () = store.append("conv1", &Message::user(&format!("msg{}", i))).await.unwrap();
                }
                let _: Vec<Message> = store.load("conv1", 10).await.unwrap();
            })
        });
    });

    // SqliteMemoryStore 带限制的加载
    group.bench_with_input(BenchmarkId::new("sqlite_limit", size), &size, |b, &size| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteMemoryStore::in_memory().unwrap();
                for i in 0..size {
                    let _: () = store.append("conv1", &Message::user(&format!("msg{}", i))).await.unwrap();
                }
                let _: Vec<Message> = store.load("conv1", 10).await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_memory_store_delete(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_store_delete");
    group.throughput(Throughput::Elements(1));

    group.bench_function("in_memory_delete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = InMemoryStore::new();
                let _: () = store.append("conv1", &Message::user("test")).await.unwrap();
                let _: () = store.delete("conv1").await.unwrap();
            })
        });
    });

    group.bench_function("sqlite_delete", |b| {
        b.iter(|| {
            rt.block_on(async {
                let store = SqliteMemoryStore::in_memory().unwrap();
                let _: () = store.append("conv1", &Message::user("test")).await.unwrap();
                let _: () = store.delete("conv1").await.unwrap();
            })
        });
    });

    group.finish();
}

fn bench_memory_store_multiple_conversations(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();

    let mut group = c.benchmark_group("memory_store_multiple_conversations");

    for &conv_count in &[5, 10, 50] {
        group.bench_with_input(BenchmarkId::new("in_memory_multi_conv", conv_count), &conv_count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let store = InMemoryStore::new();
                    for i in 0..count {
                        let conv_id = format!("conv{}", i);
                        let _: () = store.append(&conv_id, &Message::user("message")).await.unwrap();
                    }
                    // 加载所有对话
                    for i in 0..count {
                        let conv_id = format!("conv{}", i);
                        let _: Vec<Message> = store.load(&conv_id, 0).await.unwrap();
                    }
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("sqlite_multi_conv", conv_count), &conv_count, |b, &count| {
            b.iter(|| {
                rt.block_on(async {
                    let store = SqliteMemoryStore::in_memory().unwrap();
                    for i in 0..count {
                        let conv_id = format!("conv{}", i);
                        let _: () = store.append(&conv_id, &Message::user("message")).await.unwrap();
                    }
                    // 加载所有对话
                    for i in 0..count {
                        let conv_id = format!("conv{}", i);
                        let _: Vec<Message> = store.load(&conv_id, 0).await.unwrap();
                    }
                })
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_memory_store_append,
    bench_memory_store_load,
    bench_memory_store_load_with_limit,
    bench_memory_store_delete,
    bench_memory_store_multiple_conversations,
);

criterion_main!(benches);
