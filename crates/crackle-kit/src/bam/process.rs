//! ## Bam file process helpers
//!
//! 2 Processes are supported:
//! 1. Make a modified bam (with Producer and Consumer)
//! 2. Do a given task per genomic positions (in parallel)
//!     Examples)
//!     a. Pileup each variant position and get reads.
//!
//!
//!

use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    hash::RandomState,
    i32,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{
        Arc,
        atomic::{self, AtomicUsize},
    },
    thread::{self, sleep},
    time::{Duration, Instant},
};

use anyhow::Error;
use crossbeam_channel::{Sender, TryRecvError, bounded};
use indicatif::ProgressBar;
use rayon::{
    ThreadPoolBuilder,
    iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator},
};
use rust_htslib::bam::{
    self, Header, HeaderView, IndexedReader, Read as _, Record, Writer,
    pileup::{Pileup, PileupOption},
};
use tracing::{Level, event};

use crate::{
    data::{
        data_with_index::DataWithIndex,
        locus::{GenomeCoordinate, GenomeRegion},
    },
    utils::{
        batch_region::batch_region, batched_channel::BatchedChannel, batched_data::BatchedData,
        pbar::prepare_pbar,
    },
};

const N_1M: usize = 10_usize.pow(6);

pub trait BamLocusWorker<'a>: Send + Sync {
    type Input: BamLocusWorkInput<'a>;
    type Output: Send + Sync;
    type Error: Into<Error>;

    fn work_for_locus(&self, plp: Pileup, input: Self::Input) -> Result<Self::Output, Self::Error>;
}

pub trait BamLocusWorkInput<'a>: Send + Sync {
    fn genome_coordinate(&self) -> &GenomeCoordinate<'a>;
}

impl<'a> BamLocusWorkInput<'a> for GenomeCoordinate<'a> {
    fn genome_coordinate(&self) -> &GenomeCoordinate<'a> {
        self
    }
}

fn batch_input_by_coordinate<'a, I: BamLocusWorkInput<'a>>(
    inputs: impl IntoIterator<Item = I>,
    window_size: usize,
) -> Vec<Vec<I>> {
    let mut input_iter = inputs.into_iter();
    let mut c_vec = vec![];
    let (mut c_contig, mut c_start) = match input_iter.next() {
        Some(inp) => {
            let gc = inp.genome_coordinate();

            let contig_clone = gc.contig.clone();
            let start_val = gc.pos;
            c_vec.push(inp);
            (contig_clone, start_val)
        }
        None => return vec![],
    };

    let mut res: Vec<Vec<I>> = vec![];
    for inp in input_iter {
        let gc = inp.genome_coordinate();
        // Condition to start a new batch:
        // 1. The contig changes.
        // 2. The span from the batch's start to the current region's end exceeds window_size.
        if c_contig == gc.contig && gc.pos - c_start < window_size as i64 {
            c_vec.push(inp);
        } else {
            res.push(c_vec); // Push the completed batch
            c_contig = gc.contig.clone(); // Start a new batch: reset contig and start
            c_start = gc.pos;
            c_vec = vec![inp]; // Start new batch with current region
        }
    }

    // Push any remaining regions in the last batch
    if !c_vec.is_empty() {
        res.push(c_vec);
    }

    res
}

///
/// # Example
/// ```
/// struct MeanBPWorker {
/// }
///
/// impl BamLocusWorker for MeanBPWorker {
///     fn work_for_locus() {
///         // your code here
///     }
/// }
///
///
///
///
/// ```
pub struct ParallelLocusProcessor<W: for<'a> BamLocusWorker<'a>> {
    bam_locus_worker: W,
    n_threads: usize,
    bam_path: PathBuf,
}

impl<W: for<'a> BamLocusWorker<'a>> ParallelLocusProcessor<W> {
    pub fn new(bam_locus_worker: W, n_threads: usize, bam_path: PathBuf) -> Self {
        Self {
            bam_locus_worker,
            n_threads,
            bam_path,
        }
    }

    pub fn process_with_batch<'a>(
        &self,
        inputs: Vec<<W as BamLocusWorker<'a>>::Input>,
        batch_window_size: usize,
    ) -> Result<Vec<<W as BamLocusWorker<'a>>::Output>, Error> {
        // make batch
        let batched_regions = batch_input_by_coordinate(inputs.into_iter(), batch_window_size);

        event!(
            Level::DEBUG,
            "batched_regions len={}",
            batched_regions.len()
        );

        // open threadpool and distribute the jobs.
        let tp = ThreadPoolBuilder::new()
            .num_threads(self.n_threads)
            .build()?;

        let batch_res = tp.scope(|_scope| {
            event!(Level::DEBUG, "Parallel Processing...");

            let r = batched_regions
                .into_par_iter()
                .map(|batch| {
                    if batch.is_empty() {
                        return Ok(vec![]);
                    }

                    let mut ir = IndexedReader::from_path(&self.bam_path)?;

                    let first_elem = batch.first().unwrap();
                    let last_elem = batch.last().unwrap();

                    let batch_contig = first_elem.genome_coordinate().contig.as_str();
                    let batch_pileup_start = first_elem.genome_coordinate().pos - 1; // batch is not empty, by the if condition of function start point.
                    let batch_pileup_end = last_elem.genome_coordinate().pos;

                    ir.fetch((batch_contig, batch_pileup_start, batch_pileup_end))?;
                    let mut pileups = ir
                        .pileup_with_option(PileupOption {
                            max_depth: i32::MAX,
                            ignore_overlaps: true,
                        })
                        .peekable();

                    // Create peekable iterators for both the pileups and the batch of inputs.
                    let mut res = Vec::with_capacity(batch.len());

                    let mut batch_peekable = batch.into_iter().peekable();

                    // This is the efficient "merge/zip" sweep-line algorithm
                    while let (Some(Ok(pileup_col)), Some(input)) =
                        (pileups.peek(), batch_peekable.peek())
                    {
                        let pileup_pos = pileup_col.pos() as i64;
                        // Assuming you've updated the trait to use GenomeCoordinate
                        let target_pos = input.genome_coordinate().pos - 1;

                        match pileup_pos.cmp(&target_pos) {
                            Ordering::Less => {
                                // Case 1: Pileup is before our target site.
                                // Discard the pileup and advance the pileup iterator.
                                pileups.next();
                            }
                            Ordering::Greater => {
                                // Case 2: We've passed our target site, but there was no pileup (zero coverage).
                                // Discard the target and advance the site iterator.
                                batch_peekable.next();
                            }
                            Ordering::Equal => {
                                // Case 3: Match found! Process it.
                                // We must consume both items from the iterators to advance.
                                if let (Some(Ok(plp)), Some(inp)) =
                                    (pileups.next(), batch_peekable.next())
                                {
                                    let r = self
                                        .bam_locus_worker
                                        .work_for_locus(plp, inp)
                                        .map_err(|err| err.into())?;
                                    res.push(r);
                                }
                            }
                        }
                    }

                    Ok::<_, Error>(res)
                })
                .collect::<Result<Vec<_>, Error>>()?;

            event!(Level::DEBUG, "Flatten Batched Results...");

            let r2 = r.into_iter().flatten().collect::<Vec<_>>();

            event!(Level::DEBUG, "Done.");

            Ok::<_, Error>(r2)
        })?;

        Ok(batch_res)
    }
}

// pub trait RecordModifierInput {

// }

pub trait RecordModifier: Send + Sync {
    // type Input: RecordModifierInput;
    // type Output: Send + Sync;
    type Error: Into<Error>;

    /// modify record and return `Option<()>`,   
    /// `None` means this record should not be written to the output bamfile.
    fn modify_record(&self, record: &mut bam::Record) -> Result<Option<()>, Self::Error>;
}

/// Read a bam file, modify reads and write bam.
///
/// Use Producer Consumer Method.
pub struct ParallelBamProcessor<R: RecordModifier> {
    record_modifier: R,
    // bam_path: PathBuf,
    // n_threads: usize,
}

impl<R: RecordModifier> ParallelBamProcessor<R> {
    fn process_bam(
        &self,
        input_bam_path: impl AsRef<Path>,
        read_thread: usize,
        worker_thread: usize,
        write_thread: usize,
        out_bam_path: impl AsRef<Path>,
        batch_size: usize,
        channel_capacity: usize,
    ) -> Result<(), Error> {
        let input_bam_path = input_bam_path.as_ref();
        let out_bam_path = out_bam_path.as_ref();

        // check bam path exists
        if !input_bam_path.exists() {
            Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                input_bam_path.to_string_lossy(),
            ))?
        }

        // prepare channels
        let (tx_read, rx_read) = bounded::<BatchedData<DataWithIndex<Record>>>(channel_capacity);
        let (tx_worker, rx_worker) =
            bounded::<BatchedData<DataWithIndex<Record>>>(channel_capacity);
        // let (tx_write, rx_write) = bounded::<BatchedData<DataWithIndex<Record>>>(channel_capacity);
        let (tx_buf, rx_buf) = bounded::<BatchedData<DataWithIndex<Record>>>(channel_capacity);

        let batch_init =
            || BatchedData::new(|| DataWithIndex::new(Record::default(), 0), batch_size);
        for _ in 0..channel_capacity {
            tx_buf.send(batch_init())?;
        }

        // read header first
        let reader = IndexedReader::from_path(&input_bam_path)?;
        let header_view_bytes = Arc::new(reader.header().as_bytes().to_vec());

        let bam_path_clone = input_bam_path.to_path_buf();
        let rx_buf_clone = rx_buf.clone();
        // reader thread
        thread::scope(|s| {
            let reader_handle = s.spawn(move || {
                let mut reader = IndexedReader::from_path(bam_path_clone)?;

                if read_thread > 1 {
                    reader.set_threads(read_thread)?; // Use shared pool for internal I/O [1]
                }

                reader.fetch(".")?; // Read all records from the file

                let mut i = 0;

                'batched_process_loop: loop {
                    let mut record_batch = match rx_buf_clone.try_recv() {
                        Ok(v) => v,
                        Err(TryRecvError::Empty) => {
                            sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            break;
                        }
                    };

                    while let Some(record_with_idx) = record_batch.next_mut() {
                        let record = record_with_idx.data_mut();

                        if let Some(res) = reader.read(record) {
                            match res {
                                Ok(_) => {
                                    record.remove_header();
                                    record_with_idx.idx = i;
                                    i += 1;
                                }
                                Err(e) => {
                                    event!(Level::WARN, "Error reading record: {:?}", e);
                                    // Decide how to handle: propagate error, skip read, etc.
                                    // For this example, we'll just continue.
                                }
                            }

                            // if i % const { 10_usize.pow(6) } == 0 {
                            //     // break 'batched_process_loop;
                            //     // event!(
                            //     //     Level::DEBUG,
                            //     //     "Reading speed: {:.1}/s",
                            //     //     i as f64 / timer.elapsed().as_secs_f64()
                            //     // );
                            // }
                        } else {
                            if !record_batch.is_empty() {
                                tx_read.send(record_batch)?;
                            }
                            drop(tx_read);
                            break 'batched_process_loop;
                        }
                    }

                    tx_read.send(record_batch)?;
                }

                event!(Level::DEBUG, "Reader thread ended.");

                Ok::<_, Error>(())
            });

            // worker threads
            let mut worker_handles = Vec::with_capacity(worker_thread);
            let n_processed = Arc::new(AtomicUsize::new(0));
            // let work_timer = Instant::now();
            for i in 0..worker_thread {
                // let rx_reads_clone = rx_reads.clone();
                // let tx_modified_reads_clone = tx_modified_reads.clone();
                // let input_bam_path_clone = input_bam_path.clone();
                let header_view = header_view_bytes.clone();
                let n_processed = n_processed.clone();
                let rx_read_clone = rx_read.clone();
                let tx_worker_clone = tx_worker.clone();

                worker_handles.push(s.spawn(move || {
                    // event!(
                    //     Level::INFO,
                    //     "Process worker {:?} start. linux thread id={}",
                    //     thread::current().id(),
                    //     unsafe { libc::syscall(libc::SYS_gettid) },
                    // );

                    let header_view = Rc::new(HeaderView::from_bytes(&header_view));

                    loop {
                        let mut record_batch = match rx_read_clone.try_recv() {
                            Ok(v) => v,
                            Err(TryRecvError::Empty) => {
                                sleep(Duration::from_millis(10));
                                continue;
                            }
                            Err(TryRecvError::Disconnected) => {
                                event!(
                                    Level::DEBUG,
                                    "Checked end of channel from reader, thread {:?}",
                                    thread::current().id()
                                );
                                break;
                            }
                        };

                        for record_with_idx in record_batch.filled_mut() {
                            let record = record_with_idx.data_mut();

                            record.set_header(Rc::clone(&header_view));
                            // `iter()` blocks until a message is available or channel is disconnected [2]

                            // how to remove the record from the batch? the problem is,
                            // writer thread use the idx as order, so the writer will wait for this missing index forever

                            match self.record_modifier.modify_record(record) {
                                Ok(Some(_)) => {
                                    record.remove_header();
                                }
                                Ok(None) => {
                                    *record = Record::default(); // re-assign empty record for writer not to write this record.
                                }
                                Err(err) => {
                                    event!(
                                        Level::WARN,
                                        "Error: {}. drop this read:{}",
                                        err.into(),
                                        str::from_utf8(record.qname())?
                                    );
                                    *record = Record::default();
                                    continue;
                                }
                            };

                            n_processed.fetch_add(1, atomic::Ordering::Relaxed);

                            // let n_proc = n_processed.load(atomic::Ordering::Relaxed);
                            // if n_proc % N_1M == 0 {
                            //     event!(
                            //         Level::DEBUG,
                            //         "Processing speed: {:.1}/s",
                            //         n_proc as f64 / work_timer.elapsed().as_secs_f64()
                            //     );
                            // }
                        }

                        match tx_worker_clone.send(record_batch) {
                            Ok(_) => {}
                            Err(err) => {
                                event!(Level::ERROR, "Unexpected send failure.");
                                sleep(Duration::from_secs(2));
                                Err(err)?
                            }
                        };
                    }

                    event!(
                        Level::DEBUG,
                        "Processing thread {:?} ended.",
                        thread::current().id()
                    );

                    Ok::<(), anyhow::Error>(()) // Return Result from the thread
                }));
            }

            // Spawn the Consumer (Writer) Thread
            // let input_bam_path_clone = input_bam_path.clone();
            let header_view = header_view_bytes.clone();
            let writer_handle = s.spawn(move || {
                // event!(
                //     Level::INFO,
                //     "Writer worker start. linux thread id ={}",
                //     unsafe { libc::syscall(libc::SYS_gettid) }
                // );
                let pbar = prepare_pbar(0);
                let mut i = 0;

                let header_view = Rc::new(HeaderView::from_bytes(&header_view));

                let header = Header::from_template(&header_view);

                let mut writer =
                    Writer::from_path(&out_bam_path, &header, rust_htslib::bam::Format::Bam)?;

                if write_thread > 1 {
                    writer.set_threads(write_thread)?; // Use shared pool for internal I/O
                }

                let default_record = Record::default();

                let mut ordered_buf_map: HashMap<usize, BatchedData<DataWithIndex<Record>>> =
                    HashMap::with_capacity(1024 * 16);

                #[inline]
                fn write_and_send_batch(
                    mut next_batch_to_write: BatchedData<DataWithIndex<Record>>,
                    writer: &mut Writer,
                    tx_buffer: &Sender<BatchedData<DataWithIndex<Record>>>,
                    i: &mut usize,
                    // work_timer: &Instant,
                    pbar: &ProgressBar,
                    default_record: &Record,
                    send_empty_batch: bool,
                ) -> Result<(), Error> {
                    for record_with_idx in next_batch_to_write.filled_mut() {
                        // record.set_header(Rc::clone(&header_view));
                        let record = record_with_idx.data_mut();

                        if record != default_record {
                            writer.write(&record)?;
                        }

                        *i += 1;
                        if *i % N_1M == 0 {
                            // event!(
                            //     Level::DEBUG,
                            //     "Writing speed: {:.1}/s",
                            //     *i as f64 / work_timer.elapsed().as_secs_f64()
                            // );
                            pbar.inc(N_1M as u64);
                        }
                    }

                    next_batch_to_write.reset_index();
                    if send_empty_batch {
                        tx_buffer.send(next_batch_to_write)?;
                    }
                    Ok(())
                }

                loop {
                    let mut record_batch_from_chan = match rx_worker.try_recv() {
                        Ok(v) => v,
                        Err(TryRecvError::Empty) => {
                            sleep(Duration::from_millis(10));
                            continue;
                        }
                        Err(TryRecvError::Disconnected) => {
                            event!(Level::DEBUG, "rx processed closed.");
                            sleep(Duration::from_secs(2));
                            break;
                        }
                    };

                    let maximum_batch_gen = 1024;
                    let mut n_batch_gen = 0;

                    let start_idx_from_channel = match record_batch_from_chan.filled().iter().next()
                    {
                        Some(v) => v.idx,
                        None => panic!("Code failed: Reader sent empty batch!"),
                    };

                    let mut next_batch_to_write = if start_idx_from_channel == i {
                        record_batch_from_chan
                    } else {
                        ordered_buf_map.insert(start_idx_from_channel, record_batch_from_chan);

                        match ordered_buf_map.remove(&i) {
                            Some(b) => b,
                            None => {
                                if n_batch_gen < maximum_batch_gen {
                                    tx_buf.send(batch_init())?; // make new empty batch for compensating keeping a batch.
                                    n_batch_gen += 1;
                                }

                                continue;
                            }
                        }
                    };

                    write_and_send_batch(
                        next_batch_to_write,
                        &mut writer,
                        &tx_buf,
                        &mut i,
                        // &work_timer,
                        &pbar,
                        &default_record,
                        true,
                    )?;
                }

                // write remained records in ordered_buf_map.
                event!(
                    Level::DEBUG,
                    "Writing remaining records ({}) in ordered_buffer...",
                    ordered_buf_map.len()
                );

                loop {
                    if let Some(next_batch_to_write) = ordered_buf_map.remove(&i) {
                        write_and_send_batch(
                            next_batch_to_write,
                            &mut writer,
                            &tx_buf,
                            &mut i,
                            // &work_timer,
                            &pbar,
                            &default_record,
                            false,
                        )?;
                    } else {
                        break;
                    }
                }

                debug_assert!(ordered_buf_map.is_empty());

                pbar.inc(i as u64 - pbar.position());
                pbar.tick();
                pbar.finish();

                event!(Level::DEBUG, "writer thread ended.");
                sleep(Duration::from_secs(2));

                Ok::<(), anyhow::Error>(()) // Return Result from the thread
            });

            // 5. Wait for all threads to complete
            reader_handle.join().expect("Reader thread panicked")?;

            for handle in worker_handles {
                handle.join().expect("Processor thread panicked")?;
            }

            drop(tx_worker);

            writer_handle.join().expect("Writer thread panicked")?;

            Ok::<_, Error>(())
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use tracing::level_filters::LevelFilter;

    use super::*;
    use crate::{
        bam::process::BamLocusWorker, data::chrom::Chrom, tracing::{setup_logging_stderr_only, setup_logging_stderr_only_debug},
    };

    struct MeanBPWorker;

    impl<'a> BamLocusWorker<'a> for MeanBPWorker {
        type Output = f64;
        type Input = GenomeCoordinate<'a>;
        type Error = Error;

        fn work_for_locus(
            &self,
            plp: Pileup,
            inp: Self::Input,
        ) -> Result<Self::Output, Self::Error> {
            let mut bq_sum: u64 = 0;
            let alignments = plp.alignments();
            let len = alignments.len();
            for alignment in alignments {
                let qpos = match alignment.qpos() {
                    Some(qpos) => qpos,
                    None => continue,
                };

                let record = alignment.record();

                let bq = record.qual().get(qpos).unwrap();
                bq_sum += *bq as u64;
            }

            Ok(bq_sum as f64 / len as f64)
        }
    }

    #[test]
    fn parallel_locus_processor1() -> Result<(), Box<dyn std::error::Error>> {
        setup_logging_stderr_only_debug(LevelFilter::DEBUG)?;

        let bam_path = "/home/eck/workspace/common_resources/NA12878.chrom20.ILLUMINA.bwa.CEU.low_coverage.20121211.bam";
        let plp = ParallelLocusProcessor {
            bam_locus_worker: MeanBPWorker,
            n_threads: 4,
            bam_path: bam_path.into(),
        };

        let regions = (60000..(60000 + 1_000_000))
            .step_by(1000)
            .map(|p| GenomeCoordinate {
                contig: Chrom::Other("20".into()),
                pos: p,
            })
            .collect::<Vec<_>>();

        let r = plp.process_with_batch(regions, 100_000)?;

        eprintln!("{} {:?}", r.len(), &r[..10]);

        Ok(())
    }

    // Helper function to create coordinates for tests
    fn coord<'a>(contig: &'a str, pos: i64) -> GenomeCoordinate<'a> {
        GenomeCoordinate {
            contig: Chrom::Other(Cow::Borrowed(contig)),
            pos,
        }
    }

    #[test]
    fn test_standard_batching() {
        let inputs = vec![
            coord("chr1", 100),
            coord("chr1", 200),
            coord("chr1", 10000), // This should start a new batch
            coord("chr1", 10100),
        ];
        let window_size = 1000;
        let batches = batch_input_by_coordinate(inputs, window_size);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[0][0].pos, 100);
        assert_eq!(batches[1][0].pos, 10000);
    }

    #[test]
    fn test_contig_change() {
        let inputs = vec![
            coord("chr1", 100),
            coord("chr1", 200),
            coord("chr2", 300), // This must start a new batch
            coord("chr2", 400),
        ];
        let window_size = 1000;
        let batches = batch_input_by_coordinate(inputs, window_size);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2);
        assert_eq!(batches[1].len(), 2);
        assert_eq!(batches[0][0].contig, Chrom::Other(Cow::Borrowed("chr1")));
        assert_eq!(batches[1][0].contig, Chrom::Other(Cow::Borrowed("chr2")));
    }

    #[test]
    fn test_window_size_boundary() {
        let window_size = 1000;
        let inputs = vec![
            coord("chr1", 100),
            coord("chr1", 1099), // gc.pos (1099) - c_start (100) = 999. This is < 1000, so it fits.
            coord("chr1", 1100), // gc.pos (1100) - c_start (100) = 1000. This is NOT < 1000, new batch.
        ];

        let batches = batch_input_by_coordinate(inputs, window_size);

        assert_eq!(batches.len(), 2);
        assert_eq!(batches[0].len(), 2); // The first two items should be in the first batch
        assert_eq!(batches[1].len(), 1); // The third item starts a new batch
        assert_eq!(batches[1][0].pos, 1100);
    }

    #[test]
    fn test_empty_input() {
        let inputs: Vec<GenomeCoordinate> = vec![];
        let window_size = 1000;
        let batches = batch_input_by_coordinate(inputs, window_size);
        assert!(batches.is_empty());
    }

    #[test]
    fn test_single_input() {
        let inputs = vec![coord("chr1", 100)];
        let window_size = 1000;
        let batches = batch_input_by_coordinate(inputs, window_size);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].pos, 100);
    }

    #[test]
    fn test_all_in_one_batch() {
        let inputs = vec![coord("chr1", 100), coord("chr1", 200), coord("chr1", 300)];
        let window_size = 1000;
        let batches = batch_input_by_coordinate(inputs, window_size);
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 3);
    }

    struct OnlyOddPosRecord {}

    impl RecordModifier for OnlyOddPosRecord {
        type Error = Error;

        fn modify_record(&self, record: &mut bam::Record) -> Result<Option<()>, Self::Error> {
            let r = if (record.pos()+1) % 2 == 0 {
                None
            } else {
                Some(())
            };

            Ok(r)
        }
    }

    #[test]
    fn test_parallel_bam_processor() -> Result<(), Box<dyn std::error::Error>> {
        setup_logging_stderr_only(LevelFilter::DEBUG)?;

        let pbp = ParallelBamProcessor {
            record_modifier: OnlyOddPosRecord {},
        };

        let input_bam_path = "/home/eck/workspace/common_resources/NA12878.chrom20.ILLUMINA.bwa.CEU.low_coverage.20121211.bam";
        let read_thread = 1;
        let worker_thread = 2;
        let write_thread = 4;
        let out_bam_path = "NA12878.chrom20.ILLUMINA.bwa.CEU.low_coverage.20121211.oddposonly.bam";
        let batch_size = 1024;
        let channel_capacity = 128;

        pbp.process_bam(
            input_bam_path,
            read_thread,
            worker_thread,
            write_thread,
            out_bam_path,
            batch_size,
            channel_capacity,
        )?;


        Ok(())
    }
}
