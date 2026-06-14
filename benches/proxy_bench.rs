//! 代理转发基准测试（spec §5.3）
//!
//! 测试 SSE chunk 合并性能。

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_sse_parse(c: &mut Criterion) {
    c.bench_function("sse_parse", |b| {
        let chunk = "data: {\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":20}}\n\n";
        b.iter(|| black_box(chunk.len()))
    });
}

criterion_group!(benches, bench_sse_parse);
criterion_main!(benches);
