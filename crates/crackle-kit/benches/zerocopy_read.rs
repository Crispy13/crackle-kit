use anyhow::Error;
use criterion::{Criterion, criterion_group, criterion_main};
use std::{
    fs::File, hint::black_box, io::{BufRead, BufReader, Read}, path::Path
};

const FILE_PATH: &str = "/home/eck/workspace/common_resources/GCF_000001405.40_GRCh38.p14_genomic.fna.gz.gzi";

// --- Method 1: The "Normal" Way (High Allocation) ---
// This allocates a new String on the heap for every single line found.
fn normal_read_lines(path: &str) -> usize {
    let file = File::open(path).expect("File not found");
    let mut reader = BufReader::new(file);
    let mut count = 0;

    // let mut buf = Vec::with_capacity(1024);
    let mut buf = [0_u8; 8192 * 2];

    while let Ok(n) = {
        reader.read(&mut buf)
    } {
        if n > 0 {
            count += n;
        } else {
            break;
        }
    }
    count
}

// No new memory is allocated for the data.
fn zero_copy_read(path: &str) -> usize {
    let file = File::open(path).expect("File not found");
    let mut reader = BufReader::with_capacity(8192 * 2, file);
    let mut count = 0;

    loop {
        // 1. Get a reference to the internal buffer (NO COPY)
        let buffer = reader.fill_buf().expect("Read error");

        // 2. Check if EOF
        let length = buffer.len();
        if length == 0 {
            break;
        }

        // 3. Process the bytes directly from the buffer
        // (Here we just sum lengths to simulate work, similar to above)
        // In a real parser, you would iterate `buffer` here.
        count += length;

        // 4. Tell the reader we are done with this chunk
        reader.consume(length);
    }
    count
}

fn benchmark(c: &mut Criterion) {
    // 1. Validate file exists before starting
    if !Path::new(FILE_PATH).exists() {
        panic!(
            "Error: The file '{}' does not exist. Please create it or change the path. cwd:{}",
            FILE_PATH,
            std::env::current_dir().unwrap().display()
        );
    }

    let mut group = c.benchmark_group("IO_Patterns");

    // Benchmark 1: Allocating Strings
    group.bench_function("normal_read (String alloc)", |b| {
        b.iter(|| {
            // black_box prevents compiler from optimizing this away
            let result = normal_read_lines(black_box(FILE_PATH));
            black_box(result);
        })
    });

    // Benchmark 2: Zero Copy Buffer
    group.bench_function("zero_copy (fill_buf)", |b| {
        b.iter(|| {
            let result = zero_copy_read(black_box(FILE_PATH));
            black_box(result);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark);
criterion_main!(benches);
