use std::hint::black_box;

// IMPORTANT: Replace `crackle_kit` with the actual name of your crate.
use crackle_kit::data::bases::{Base, BaseArr};
use criterion::{criterion_group, criterion_main, Criterion};
use rand::prelude::*;
use rand::rngs::StdRng;
use rand::SeedableRng;

/// Generates a vector of random DNA sequences using a seeded RNG for reproducibility.
fn generate_sequences(num_seqs: usize, seq_len: usize) -> Vec<Vec<u8>> {
    let mut rng = StdRng::seed_from_u64(42);
    let bases = [b'A', b'C', b'G', b'T', b'N'];
    (0..num_seqs)
        .map(|_| {
            (0..seq_len)
                .map(|_| *bases.choose(&mut rng).unwrap())
                .collect()
        })
        .collect()
}

/*
let mut group = c.benchmark_group("Match vs Lookup Table.");
    group.bench_function("match", |b| {
        b.iter(|| {
            let mut rng = StdRng::seed_from_u64(123);

            let idx = rng.random_range(0..NUM_SEQS);

            let seq = sequences[idx].as_slice();

            for b in seq {
                match b {
                    b'A' => {
                        black_box(b);
                    },
                    b'T' => {
                        black_box(b);
                    },
                    b'C' => {
                        black_box(b);
                    },
                    b'G' => {
                        black_box(b);
                    },
                    b'N' => {
                        black_box(b);
                    },
                    oth => unreachable!(),
                }
            }

            // black_box prevents the compiler from optimizing away the collection
            // black_box(string_vec);
        })
    });

    const LOOKUP_TABLE: [u8; 256] = {
        let mut table = [0; 256];
        table[b'A' as usize] = 1 as u8;
        table[b'T' as usize] = 1 as u8;
        table[b'C' as usize] = 1 as u8;
        table[b'G' as usize] = 1 as u8;
        table[b'N' as usize] = 1 as u8;
        table
    };

    group.bench_function("lookup_table", |b| {
        b.iter(|| {
            let mut rng = StdRng::seed_from_u64(123);

            let idx = rng.random_range(0..NUM_SEQS);

            let seq = sequences[idx].as_slice();

            for b in seq {
                black_box(LOOKUP_TABLE[*b as usize]);
            }

            // black_box prevents the compiler from optimizing away the collection
            // black_box(string_vec);
        })
    });

    group.finish();
*/
fn benchmark_storage_and_access(c: &mut Criterion) {
    // --- Setup ---
    const NUM_SEQS: usize = 1000;
    const SEQ_LEN: usize = 150;
    let sequences = generate_sequences(NUM_SEQS, SEQ_LEN);

    

    
    // --- Benchmarking Group for Creation ---
    let mut group = c.benchmark_group("Creation");

    // Benchmark creating a Vec of Strings
    group.bench_function("Vec<String>", |b| {
        b.iter(|| {
            let string_vec: Vec<String> = sequences
                .iter()
                .map(|s| String::from_utf8(s.clone()).unwrap())
                .collect();
            // black_box prevents the compiler from optimizing away the collection
            black_box(string_vec);
        })
    });

    // Benchmark creating a Vec of BaseArr
    group.bench_function("Vec<BaseArr>", |b| {
        b.iter(|| {
            let base_arr_vec: Vec<BaseArr> = sequences
                .iter()
                .map(|s| BaseArr::from_bytes(s).unwrap())
                .collect();
            black_box(base_arr_vec);
        })
    });
    group.finish();

    // --- Pre-build collections for access benchmarks ---
    let string_vec: Vec<String> = sequences
        .iter()
        .map(|s| String::from_utf8(s.clone()).unwrap())
        .collect();
    let base_arr_vec: Vec<BaseArr> = sequences
        .iter()
        .map(|s| BaseArr::from_bytes(s).unwrap())
        .collect();

    // --- Benchmarking Group for Random Access ---
    let mut group = c.benchmark_group("Random Access");
    let mut rng = StdRng::seed_from_u64(123);
    let access_indices: Vec<(usize, usize)> = (0..1000)
        .map(|_| (rng.random_range(0..NUM_SEQS), rng.random_range(0..SEQ_LEN)))
        .collect();

    group.bench_function("Vec<String>", |b| {
        b.iter(|| {
            for &(seq_idx, base_idx) in &access_indices {
                let base = string_vec[seq_idx].chars().nth(base_idx).unwrap();
                black_box(base);
            }
        })
    });

    group.bench_function("Vec<BaseArr>", |b| {
        b.iter(|| {
            for &(seq_idx, base_idx) in &access_indices {
                let base = base_arr_vec[seq_idx].get(base_idx).unwrap();
                black_box(base);
            }
        })
    });
    group.finish();

    // --- Benchmarking Group for Range Access (Slicing) ---
    let mut group = c.benchmark_group("Range Access");
    let mut rng = StdRng::seed_from_u64(456);
    let access_ranges: Vec<(usize, std::ops::Range<usize>)> = (0..100)
        .map(|_| {
            let start = rng.random_range(0..SEQ_LEN - 10);
            let end = start + rng.random_range(5..10);
            (rng.random_range(0..NUM_SEQS), start..end)
        })
        .collect();

    group.bench_function("Vec<String>", |b| {
        b.iter(|| {
            for &(seq_idx, ref range) in &access_ranges {
                // Slicing a string gives a &str, which we iterate over
                let string_slice = &string_vec[seq_idx][range.clone()];
                for c in string_slice.chars() {
                    black_box(c);
                }
            }
        })
    });

    group.bench_function("Vec<BaseArr>", |b| {
        b.iter(|| {
            for &(seq_idx, ref range) in &access_ranges {
                // get_iter returns an iterator that we consume
                let iter = base_arr_vec[seq_idx].get_iter(range.clone());
                for base in iter {
                    black_box(base);
                }
            }
        })
    });
    group.finish();

    // --- Benchmarking Group for Mutation (set) ---
    let mut group = c.benchmark_group("Mutation (set)");
    let mut rng = StdRng::seed_from_u64(789);
    let bases = [Base::A, Base::T, Base::C, Base::G, Base::N];
    let mutations: Vec<(usize, usize, Base, char)> = (0..1000)
        .map(|_| {
            let seq_idx = rng.random_range(0..NUM_SEQS);
            let base_idx = rng.random_range(0..SEQ_LEN);
            let new_base = *bases.choose(&mut rng).unwrap();
            let new_char = match new_base {
                Base::A => 'A', Base::T => 'T', Base::C => 'C',
                Base::G => 'G', Base::N => 'N',
            };
            (seq_idx, base_idx, new_base, new_char)
        })
        .collect();

    group.bench_function("Vec<String>", |b| {
        b.iter_with_setup(
            || string_vec.clone(),
            |mut data| {
                for &(seq_idx, base_idx, _, new_char) in &mutations {
                    data[seq_idx].replace_range(base_idx..base_idx + 1, &new_char.to_string());
                    black_box(&mut data);
                }
            },
        )
    });

    group.bench_function("Vec<BaseArr>", |b| {
        b.iter_with_setup(
            || base_arr_vec.clone(),
            |mut data| {
                for &(seq_idx, base_idx, new_base, _) in &mutations {
                    data[seq_idx].set(base_idx, new_base);
                    black_box(&mut data);
                }
            },
        )
    });
    group.finish();
}

criterion_group!(benches, benchmark_storage_and_access);
criterion_main!(benches);

