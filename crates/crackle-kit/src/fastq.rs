use anyhow::{Error, anyhow};
use crossbeam_channel::{Receiver, Sender, bounded, select};
use flate2::bufread::MultiGzDecoder;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::thread::{self, JoinHandle, sleep};
use std::time::Duration;

enum FastqReader {
    Plain(BufReader<File>),
    Gz(BufReader<MultiGzDecoder<BufReader<File>>>),
}

impl FastqReader {
    fn from_path(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = BufReader::new(File::open(path.as_ref())?);
        if let Some(true) = path.as_ref().extension().map(|s| s == "gz") {
            let decoder = MultiGzDecoder::new(file);
            Ok(FastqReader::Gz(BufReader::new(decoder)))
        } else {
            Ok(FastqReader::Plain(file))
        }
    }
}

impl io::Read for FastqReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            FastqReader::Plain(buf_reader) => buf_reader.read(buf),
            FastqReader::Gz(buf_reader) => buf_reader.read(buf),
        }
    }
}

impl BufRead for FastqReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        match self {
            FastqReader::Plain(r) => r.fill_buf(),
            FastqReader::Gz(r) => r.fill_buf(),
        }
    }

    fn consume(&mut self, amt: usize) {
        match self {
            FastqReader::Plain(r) => r.consume(amt),
            FastqReader::Gz(r) => r.consume(amt),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FastqRecord {
    buf: Vec<u8>,
    indices: [usize; 4],
    idx_offset: usize,
}

impl FastqRecord {
    /// Creates a new FastqRecord with preallocated buffer space.
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(8192),
            indices: [0; 4],
            idx_offset: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn clear(&mut self) {
        self.buf.clear();
        self.idx_offset = 0;
        self.indices.iter_mut().for_each(|x| *x = 0);
    }

    pub fn push_line_from(&mut self, mut reader: impl BufRead) -> Result<bool, Error> {
        if self.idx_offset >= self.indices.len() {
            panic!("4 lines has been added already.");
        }

        let n = reader.read_until(b'\n', &mut self.buf)?;

        if n > 0 {
            self.buf.pop_if(|x| x.is_ascii_whitespace());

            self.indices[self.idx_offset] = self.buf.len();
            self.idx_offset += 1;

            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn load_record(&mut self, mut reader: impl BufRead) -> Result<bool, Error> {
        self.clear();
        for i in 0..4 {
            if !self.push_line_from(&mut reader)? {
                if i == 0 {
                    return Ok(false);
                } else {
                    Err(io::Error::new(
                        io::ErrorKind::UnexpectedEof,
                        "Unexpected EOF in fastq.",
                    ))?
                }
            }
        }
        Ok(true)
    }

    /// Returns the header as a &str.
    pub fn header(&self) -> &str {
        std::str::from_utf8(&self.buf[0..self.indices[0]]).expect("Invalid UTF-8 in header")
    }

    pub fn header_bytes(&self) -> &[u8] {
        &self.buf[0..self.indices[0]]
    }

    pub fn header_id_bytes(&self) -> &[u8] {
        let header = &self.buf[0..self.indices[0]];
        // Find the position of the first whitespace
        match header.iter().position(|b| b.is_ascii_whitespace()) {
            Some(pos) => &header[0..pos],
            None => header, // Return the entire header if no whitespace found
        }
    }

    /// Returns the sequence as a &str.
    pub fn sequence(&self) -> &str {
        std::str::from_utf8(&self.buf[self.indices[0]..self.indices[1]])
            .expect("Invalid UTF-8 in sequence")
    }

    /// Returns the plus line as a &str.
    pub fn plus(&self) -> &str {
        std::str::from_utf8(&self.buf[self.indices[1]..self.indices[2]])
            .expect("Invalid UTF-8 in plus line")
    }

    /// Returns the quality line as a &str.
    pub fn quality(&self) -> &str {
        std::str::from_utf8(&self.buf[self.indices[2]..]).expect("Invalid UTF-8 in quality")
    }
}

/// Spawns a thread that continuously loads FASTQ records from the file at `filename`
/// and sends them on a bounded crossbeam channel.
fn spawn_reader_thread(
    filename: impl AsRef<Path>,
    sender: Sender<Result<Vec<FastqRecord>, Error>>,
    buf_receiver: Receiver<Vec<FastqRecord>>,
) -> Result<thread::JoinHandle<Result<(), Error>>, Error> {
    let filename = filename.as_ref().to_path_buf();
    let mut reader = FastqReader::from_path(filename)?;

    let r = thread::spawn(move || {
        'w: loop {
            let mut record_buf = buf_receiver.recv()?;

            for record in record_buf.iter_mut() {
                match record.load_record(&mut reader) {
                    Ok(true) => {}
                    Ok(false) => {
                        // EOF reached.
                        sender.send(Ok(record_buf))?;
                        break 'w;
                    }
                    Err(e) => {
                        let _ = sender.send(Err(e));
                        break 'w;
                    }
                }
            }

            sender.send(Ok(record_buf))?;
        }

        while !sender.is_empty() {
            sleep(Duration::from_millis(200));
        }

        Ok(())
    });

    Ok(r)
}

// =============================================================================
// Assume that the following types/functions are defined elsewhere:
//
// - FastqReader (enum) with its Read/BufRead impls.
// - FastqRecord with its methods (new, clear, push_line_from, load_record, header, etc.)
// - spawn_reader_thread(filename: &Path, out_sender: Sender<Result<Vec<FastqRecord>, Error>>, pool_receiver: Receiver<Vec<FastqRecord>>)
// =============================================================================

// -----------------------------------------------------------------------------
// Configuration: PairedFastqReaderConfig
// -----------------------------------------------------------------------------

/// The configuration for a paired FASTQ reader.
/// This struct stores only configuration (file paths and batch settings) and
/// does not start any background threads until you call `run()`.
pub struct PairedFastqReaderConfig {
    r1_filename: PathBuf,
    r2_filename: PathBuf,
    batch_size: usize,
    pool_capacity: usize,
}

impl PairedFastqReaderConfig {
    /// Constructs a new configuration with the given FASTQ filenames.
    pub fn new(r1_filename: impl AsRef<Path>, r2_filename: impl AsRef<Path>) -> Self {
        Self {
            r1_filename: r1_filename.as_ref().to_path_buf(),
            r2_filename: r2_filename.as_ref().to_path_buf(),
            batch_size: 1024,    // Fixed records per batch.
            pool_capacity: 512, // Fixed number of batches.
        }
    }

    /// Spawns the worker threads based on the configuration and returns the runtime reader.
    pub fn run(self) -> Result<PairedFastqReader, Error> {
        // Create output channels from the worker threads.
        let (tx_r1, rx_r1) = bounded::<Result<Vec<FastqRecord>, Error>>(self.pool_capacity);
        let (tx_r2, rx_r2) = bounded::<Result<Vec<FastqRecord>, Error>>(self.pool_capacity);

        // Create pool channels for recycling empty batch buffers.
        let (pool_tx_r1, pool_rx_r1) = bounded::<Vec<FastqRecord>>(self.pool_capacity);
        let (pool_tx_r2, pool_rx_r2) = bounded::<Vec<FastqRecord>>(self.pool_capacity);

        // Preinitialize the batch pools.
        for _ in 0..self.pool_capacity {
            let batch: Vec<FastqRecord> =
                (0..self.batch_size).map(|_| FastqRecord::new()).collect();
            pool_tx_r1.send(batch.clone())?; // Clone one for r1.
            pool_tx_r2.send(batch)?; // r2 gets its own copy.
        }

        // Spawn worker threads (using your spawn_reader_thread function).
        let handle_r1 = spawn_reader_thread(&self.r1_filename, tx_r1, pool_rx_r1)?;
        let handle_r2 = spawn_reader_thread(&self.r2_filename, tx_r2, pool_rx_r2)?;

        Ok(PairedFastqReader {
            // Initialize channels.
            r1_out: rx_r1,
            r2_out: rx_r2,
            // Store pool senders.
            r1_pool: pool_tx_r1,
            r2_pool: pool_tx_r2,
            // Start with no current batch and indices at 0.
            current_batch_r1: None,
            current_batch_r2: None,
            current_index_r1: 0,
            current_index_r2: 0,
            // Save join handles for later shutdown.
            handles: vec![handle_r1, handle_r2],
        })
    }
}

enum ProcessResult {
    Done(Option<Result<(), Error>>),
    ChannelEmpty,
    NotDone,
}

// -----------------------------------------------------------------------------
// Runtime Handle: PairedFastqReader
// -----------------------------------------------------------------------------

pub struct PairedFastqReader {
    // Channels for receiving filled batches.
    r1_out: Receiver<Result<Vec<FastqRecord>, Error>>,
    r2_out: Receiver<Result<Vec<FastqRecord>, Error>>,
    // Pool channels for recycling empty batch buffers.
    r1_pool: Sender<Vec<FastqRecord>>,
    r2_pool: Sender<Vec<FastqRecord>>,
    // Current batch state and independent indices for each stream.
    current_batch_r1: Option<Vec<FastqRecord>>,
    current_batch_r2: Option<Vec<FastqRecord>>,
    current_index_r1: usize,
    current_index_r2: usize,
    // Join handles for background threads.
    handles: Vec<JoinHandle<Result<(), Error>>>,
}

impl PairedFastqReader {
    fn process_one(
        out_r: &mut FastqRecord,
        current_batch: &mut Option<Vec<FastqRecord>>,
        current_index: &mut usize,
        pool: &Sender<Vec<FastqRecord>>,
        r_out: &Receiver<Result<Vec<FastqRecord>, Error>>,
    ) -> ProcessResult {
        // If no current batch or the current batch is exhausted…
        if current_batch.is_none() || *current_index >= current_batch.as_ref().unwrap().len() {
            // Recycle an old batch, if available.
            if let Some(batch) = current_batch.take() {
                let _ = pool.send(batch);
            }
            // Try to receive a new batch nonblocking.
            match r_out.try_recv() {
                Ok(Ok(batch)) => {
                    let _ = current_batch.insert(batch);
                    *current_index = 0;
                }
                Ok(Err(e)) => return ProcessResult::Done(Some(Err(e))),
                Err(e) => match e {
                    crossbeam_channel::TryRecvError::Empty => {
                        return ProcessResult::ChannelEmpty;
                    }
                    crossbeam_channel::TryRecvError::Disconnected => {
                        return ProcessResult::Done(None); // Treat disconnection as EOF.
                    }
                },
            }
        }
        // Now, if a current batch is available, extract the next record.
        if let Some(batch) = current_batch {
            if *current_index < batch.len() {
                std::mem::swap(out_r, &mut batch[*current_index]);

                if !out_r.is_empty() {
                    *current_index += 1;
                    ProcessResult::Done(Some(Ok(())))
                } else {
                    ProcessResult::Done(None)
                }
            } else {
                panic!(
                    "Invariant failure: current_index {} >= batch.len() {}",
                    *current_index,
                    batch.len()
                );
            }
        } else {
            ProcessResult::Done(None)
        }
    }

    ///
    /// Reads the next pair of FASTQ records, filling the provided output parameters.
    ///
    /// Returns a tuple of Option<()> for each side:
    ///   - Some(()) indicates that a record was successfully read from that stream;
    ///   - None indicates that no record is available from that stream (EOF or not ready).
    ///
    /// (Header comparison is left to the caller.)
    ///
    pub fn read(
        &mut self,
        out_r1: &mut FastqRecord,
        out_r2: &mut FastqRecord,
    ) -> (Option<Result<(), Error>>, Option<Result<(), Error>>) {
        // Clear the output buffers.
        out_r1.clear();
        out_r2.clear();

        let mut proc_res1 = ProcessResult::NotDone;
        let mut proc_res2 = ProcessResult::NotDone;

        loop {
            // Process R1 if not yet successful.
            if !matches!(proc_res1, ProcessResult::Done(_)) {
                proc_res1 = Self::process_one(
                    out_r1,
                    &mut self.current_batch_r1,
                    &mut self.current_index_r1,
                    &self.r1_pool,
                    &self.r1_out,
                );
            }
            // Process R2 if not yet successful.
            if !matches!(proc_res2, ProcessResult::Done(_)) {
                proc_res2 = Self::process_one(
                    out_r2,
                    &mut self.current_batch_r2,
                    &mut self.current_index_r2,
                    &self.r2_pool,
                    &self.r2_out,
                );
            }

            // Once both sides have produced a result (record or EOF), break.
            if matches!(proc_res1, ProcessResult::Done(_))
                && matches!(proc_res2, ProcessResult::Done(_))
            {
                break;
            }
            // Optionally: add a short sleep or yield here to avoid busy looping.
            thread::sleep(Duration::from_millis(1));
        }

        match (proc_res1, proc_res2) {
            (ProcessResult::Done(r1_res), ProcessResult::Done(r2_res)) => (r1_res, r2_res),
            _ => panic!("Unexpected state in read()."),
        }
    }

    /// Shuts down the background worker threads by joining them.
    /// Returns an error if any thread panicked or returned an error.
    pub fn join(self) -> Result<(), Error> {
        for handle in self.handles {
            handle
                .join()
                .map_err(|e| anyhow!("Thread panicked: {:?}", e))??;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::env::current_dir;

    use super::*;

    const R1: &str = "/home/eck/workspace/crackle-kit/NA12891_R1.fastq.gz";
    const R2: &str = "/home/eck/workspace/crackle-kit/NA12891_R2.fastq.gz";

    const GF_R1: &str = "/home/eck/workspace/crackle-kit/gf_R1.fastq.gz";

    #[test]
    fn test_reader() -> Result<(), Box<dyn std::error::Error>> {
        let mut handle = PairedFastqReaderConfig::new(R1, R2);

        // handle.pool_capacity = 512;

        let mut handle = handle.run()?;
        
        let mut record1 = FastqRecord::new();
        let mut record2 = FastqRecord::new();

        let mut r1_bases = 0;
        let mut r2_bases = 0;
        for _ in 0..10000000 {
            match handle.read(&mut record1, &mut record2) {
                (Some(r1), Some(r2)) => {
                    r1?;
                    r2?;
                }
                _ => {}
            };
            r1_bases += record1.sequence().len();
            r2_bases += record2.sequence().len();
        }

        eprintln!("r1_bases={r1_bases}");
        eprintln!("r2_bases={r2_bases}");

        // handle.join()

        Ok(())
    }

    #[test]
    fn test_spawn_reader_thread() -> Result<(), Error> {
        // Create the sender/receiver pair for batches from the reader thread.
        let (sender, receiver) = bounded::<Result<Vec<FastqRecord>, Error>>(10);

        // Create the pool channels for recycling batch buffers.
        // We intentionally keep pool_sender in scope so that the pool channel
        // remains connected.
        let (pool_sender, pool_receiver) = bounded::<Vec<FastqRecord>>(10);

        // Prepopulate the pool with one batch.
        let batch_size = 10; // adjust if necessary
        let initial_batch: Vec<FastqRecord> = (0..batch_size).map(|_| FastqRecord::new()).collect();
        pool_sender.send(initial_batch)?;

        // Spawn the reader thread.
        let handle = spawn_reader_thread(R1, sender, pool_receiver)?;

        // Attempt to receive a batch from the reader thread.
        // This call will block until the reader thread sends a batch or errors.
        let result = receiver.recv();

        match result {
            Ok(Ok(batch)) => {
                // If a batch was received, assert that it is nonempty (if the file has data).
                assert!(!batch.is_empty(), "Received an empty batch");
                println!("Received batch with {} records", batch.len());
            }
            Ok(Err(e)) => {
                panic!("Reader thread returned an error: {:?}", e);
            }
            Err(e) => {
                panic!("Receiver error: {:?}", e);
            }
        }

        // Wait for the reader thread to complete.
        // (If the FASTQ file reaches EOF, the thread should close gracefully.)
        match handle.join() {
            Ok(Ok(())) => {
                println!("Reader thread joined successfully");
            }
            Ok(Err(e)) => {
                panic!("Reader thread exited with error: {:?}", e);
            }
            Err(e) => {
                panic!("Reader thread panicked: {:?}", e);
            }
        }

        // Keep pool_sender alive until after join.
        // (It will be dropped here at the end of the test.)
        Ok(())
    }

    #[test]
    fn test_fastq_reader_gz() -> Result<(), Error> {
        // Create a FastqReader from the path.
        // eprintln!("{}", current_dir()?.display());
        // eprintln!("{}", env!("CARGO_MANIFEST_DIR"));

        let mut reader = FastqReader::from_path(GF_R1)?;

        let mut record = FastqRecord::new();

        record.load_record(&mut reader)?;

        // eprintln!("{}", String::from_utf8_lossy(&record.buf));
        // eprintln!("{}", String::from_utf8(record.buf.to_vec())?);

        eprintln!("{}", record.header());
        eprintln!("{}", record.sequence());
        eprintln!("{}", record.plus());
        eprintln!("{}", record.quality());

        // eprintln!("{record:?}");

        Ok(())
    }
}
