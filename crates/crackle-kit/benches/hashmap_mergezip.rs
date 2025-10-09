use criterion::{criterion_group, criterion_main, Criterion};
use rand::{prelude::*, rng};
use std::{borrow::Cow, collections::HashMap, hint::black_box};


// --- Mock Data Structures (to simulate your library's components) ---

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Chrom<'a> {
    Other(Cow<'a, str>),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GenomeCoordinate<'a> {
    pub contig: Chrom<'a>,
    pub pos: i64,
}

// A mock of the rust-htslib PileupColumn, just needing the position.
#[derive(Clone, Copy)]
struct MockPileupColumn {
    pos: i64,
}

impl MockPileupColumn {
    fn pos(&self) -> i64 {
        self.pos
    }
}

// --- Algorithm Implementations ---

/// Processes sites using the HashMap lookup method.
fn process_with_hashmap(
    pileup: &[MockPileupColumn],
    sites: &[GenomeCoordinate],
) -> Vec<(i64, i64)> {
    let mut target_pos_map: HashMap<i64, _> = sites
        .iter()
        .map(|g| (g.pos - 1, g))
        .collect();
    
    let mut results = Vec::with_capacity(target_pos_map.len());

    for p_col in pileup {
        if let Some(site) = target_pos_map.remove(&p_col.pos()) {
            // Simulate doing work by creating a tuple of the positions
            results.push((p_col.pos(), site.pos));
        }
    }
    results
}

/// Processes sites using the efficient merge/zip (sweep-line) algorithm.
fn process_with_merge_zip(
    pileup: &[MockPileupColumn],
    sites: &[GenomeCoordinate],
) -> Vec<(i64, i64)> {
    use std::cmp::Ordering;

    let mut pileup_iter = pileup.iter().peekable();
    let mut sites_iter = sites.iter().peekable();
    let mut results = Vec::new();

    while let (Some(p_col), Some(site)) = (pileup_iter.peek(), sites_iter.peek()) {
        let pileup_pos = p_col.pos();
        let target_pos = site.pos - 1;

        match pileup_pos.cmp(&target_pos) {
            Ordering::Less => {
                pileup_iter.next();
            }
            Ordering::Greater => {
                sites_iter.next();
            }
            Ordering::Equal => {
                // Match found, simulate work
                results.push((pileup_pos, site.pos));
                pileup_iter.next();
                sites_iter.next();
            }
        }
    }
    results
}


// --- The Benchmark ---

fn algorithm_benchmark(c: &mut Criterion) {
    const NUM_SITES: usize = 100_000;
    const PILEUP_DENSITY: f64 = 0.7; // 70% of sites will have a pileup entry

    // Generate a large, sorted list of target sites
    let mut target_sites: Vec<GenomeCoordinate> = (0..NUM_SITES)
        .map(|i| GenomeCoordinate {
            contig: Chrom::Other(Cow::Borrowed("chr1")),
            pos: (i * 10) as i64, // Spread out sites
        })
        .collect();
    
    // Generate a mock pileup that is also sorted
    let mut rng = rng();
    let mock_pileup: Vec<MockPileupColumn> = target_sites
        .iter()
        .filter(|_| rng.random_bool(PILEUP_DENSITY))
        .map(|site| MockPileupColumn { pos: site.pos - 1 })
        .collect();

    let mut group = c.benchmark_group("Algorithm Comparison");

    group.bench_function("HashMap Approach", |b| {
        b.iter(|| {
            black_box(process_with_hashmap(
                black_box(&mock_pileup),
                black_box(&target_sites),
            ));
        })
    });

    group.bench_function("Merge/Zip Approach", |b| {
        b.iter(|| {
            black_box(process_with_merge_zip(
                black_box(&mock_pileup),
                black_box(&target_sites),
            ));
        })
    });

    group.finish();
}

criterion_group!(benches, algorithm_benchmark);
criterion_main!(benches);
