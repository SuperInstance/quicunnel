// Benchmark suite for quicunnel
//
// Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn benchmark_state_transitions(c: &mut Criterion) {
    use quicunnel::state::ConnectionStateMachine;
    use quicunnel::types::TunnelState;
    use std::time::Instant;

    c.bench_function("state_transition", |b| {
        b.iter(|| {
            let sm = ConnectionStateMachine::new();
            sm.transition(TunnelState::Connecting {
                since: Instant::now(),
            });
            sm.transition(TunnelState::Connected {
                since: Instant::now(),
                latency_ms: 50,
            });
        });
    });
}

fn benchmark_stats_accumulation(c: &mut Criterion) {
    use quicunnel::types::TunnelStats;

    c.bench_function("stats_accumulation", |b| {
        b.iter(|| {
            let mut stats = TunnelStats::default();
            stats.total_bytes_sent += 1024;
            stats.requests_sent += 1;
            stats.requests_succeeded += 1;
            stats.success_rate()
        });
    });
}

criterion_group!(benches, benchmark_state_transitions, benchmark_stats_accumulation);
criterion_main!(benches);
