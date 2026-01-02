use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
// Replace 'your_crate_name' with the actual name of your library crate
use crackle_kit::data::bases::rev_comp::{RevCompMode, RevComplementor}; 
use rand::{Rng, SeedableRng};


fn generate_dna(len: usize) -> Vec<u8> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(1);
    (0..len)
        .map(|_| match rng.random_range(0..10) {
            0 => b'A',
            1 => b'C',
            2 => b'G',
            3 => b'T',
            4 => b'N',
            5 => b'a',
            6 => b'c',
            7 => b'g',
            8 => b't',
            9 => b'n',
            _ => unreachable!(),
        })
        .collect()
}

fn bench_reverse_complement(c: &mut Criterion) {
    let mut group = c.benchmark_group("Reverse Complement (No Alloc)");

    // --- TEST CASE 1: Short Reads (150bp) ---
    // Critical for NGS (Next-Gen Sequencing) latency
    let len_small = 150;
    let input_small = generate_dna(len_small);
    group.throughput(Throughput::Bytes(len_small as u64));

    // 1. Normal (Safe, includes memset/zeroing)
    group.bench_function("150bp/Scalar (Safe)", |b| {
        let mut runner = RevComplementor::with_mode(RevCompMode::Normal);
        // Pre-warm to ensure capacity is allocated
        runner.reverse_complement(&input_small); 
        
        b.iter(|| {
            black_box(runner.reverse_complement(black_box(&input_small)));
        })
    });

    // 2. Normal Ptr (Unsafe, NO zeroing) -> TRUE Scalar Baseline
    group.bench_function("150bp/Scalar (Ptr)", |b| {
        let mut runner = RevComplementor::with_mode(RevCompMode::NormalPtr);
        runner.reverse_complement(&input_small); 

        b.iter(|| {
            black_box(runner.reverse_complement(black_box(&input_small)));
        })
    });

    // 3. SIMD (AVX2)
    if is_x86_feature_detected!("avx2") {
        group.bench_function("150bp/SIMD (AVX2)", |b| {
            let mut runner = RevComplementor::with_mode(RevCompMode::SIMD);
            runner.reverse_complement(&input_small); 

            b.iter(|| {
                black_box(runner.reverse_complement(black_box(&input_small)));
            })
        });
    }

    if is_x86_feature_detected!("avx2") {
        group.bench_function("150bp/SIMD Unrolled (AVX2)", |b| {
            let mut runner = RevComplementor::with_mode(RevCompMode::SIMDUnrolled4x);
            runner.reverse_complement(&input_small); 

            b.iter(|| {
                black_box(runner.reverse_complement(black_box(&input_small)));
            })
        });
    }

    // --- TEST CASE 2: Large Block (100KB) ---
    // Critical for throughput/bandwidth
    let len_large = 100_000;
    let input_large = generate_dna(len_large);
    group.throughput(Throughput::Bytes(len_large as u64));

    group.bench_function("100KB/Scalar (Safe)", |b| {
        let mut runner = RevComplementor::with_mode(RevCompMode::Normal);
        runner.reverse_complement(&input_large); 
        b.iter(|| {
            black_box(runner.reverse_complement(black_box(&input_large)));
        })
    });

    group.bench_function("100KB/Scalar (Ptr)", |b| {
        let mut runner = RevComplementor::with_mode(RevCompMode::NormalPtr);
        runner.reverse_complement(&input_large); 
        b.iter(|| {
            black_box(runner.reverse_complement(black_box(&input_large)));
        })
    });

    if is_x86_feature_detected!("avx2") {
        group.bench_function("100KB/SIMD (AVX2)", |b| {
            let mut runner = RevComplementor::with_mode(RevCompMode::SIMD);
            runner.reverse_complement(&input_large); 
            b.iter(|| {
                black_box(runner.reverse_complement(black_box(&input_large)));
            })
        });
    }

    if is_x86_feature_detected!("avx2") {
        group.bench_function("100KB/SIMD Unrolled (AVX2)", |b| {
            let mut runner = RevComplementor::with_mode(RevCompMode::SIMDUnrolled4x);
            runner.reverse_complement(&input_large); 
            b.iter(|| {
                black_box(runner.reverse_complement(black_box(&input_large)));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_reverse_complement);
criterion_main!(benches);