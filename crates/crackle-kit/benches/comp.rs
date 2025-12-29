use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;

// Adjust this import path to match where you put the struct
use crackle_kit::data::bases::comp::{CompMode, Complementor};
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

fn bench_complement_only(c: &mut Criterion) {
    let mut group = c.benchmark_group("Forward Complement (No Alloc)");

    // --- TEST CASE 1: Short Reads (150bp) ---
    let len_small = 150;
    let input_small = generate_dna(len_small);
    group.throughput(Throughput::Bytes(len_small as u64));

    // 1. Scalar Baseline
    group.bench_function("150bp/Scalar", |b| {
        let mut runner = Complementor::new(CompMode::Scalar);
        // Pre-warm to allocate memory once
        runner.complement(&input_small);

        b.iter(|| {
            // We use black_box to prevent the compiler from optimizing away the result
            black_box(runner.complement(black_box(&input_small)));
        })
    });

    // 2. SIMD (AVX2)
    if is_x86_feature_detected!("avx2") {
        group.bench_function("150bp/SIMD (AVX2)", |b| {
            let mut runner = Complementor::new(CompMode::SIMD);
            runner.complement(&input_small);

            b.iter(|| {
                black_box(runner.complement(black_box(&input_small)));
            })
        });
    }

    // --- TEST CASE 2: Large Block (100KB) ---
    let len_large = 100_000;
    let input_large = generate_dna(len_large);
    group.throughput(Throughput::Bytes(len_large as u64));

    group.bench_function("100KB/Scalar", |b| {
        let mut runner = Complementor::new(CompMode::Scalar);
        runner.complement(&input_large);

        b.iter(|| {
            black_box(runner.complement(black_box(&input_large)));
        })
    });

    if is_x86_feature_detected!("avx2") {
        group.bench_function("100KB/SIMD (AVX2)", |b| {
            let mut runner = Complementor::new(CompMode::SIMD);
            runner.complement(&input_large);

            b.iter(|| {
                black_box(runner.complement(black_box(&input_large)));
            })
        });
    }

    group.finish();
}

criterion_group!(benches, bench_complement_only);
criterion_main!(benches);
