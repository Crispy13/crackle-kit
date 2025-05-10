use anyhow::{Error, anyhow};
use crossbeam_channel::{bounded, Receiver, Sender};
use flate2::bufread::GzDecoder;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::thread;

enum FastqReader {
    Plain(BufReader<File>),
    Gz(BufReader<GzDecoder<BufReader<File>>>),
}

impl FastqReader {
    fn open(path: impl AsRef<Path>) -> io::Result<Self> {
        let file = BufReader::new(File::open(path.as_ref())?);
        if path.as_ref().ends_with(".gz") {
            let decoder = GzDecoder::new(file);
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
) -> thread::JoinHandle<Result<(), Error>> {
    let filename = filename.as_ref().to_path_buf();
    thread::spawn(move || {
        let mut reader = FastqReader::open(filename)?;
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

        Ok(())
    })
}

/// Public API: PairedFastqReader
///
/// This struct separates configuration/spawning from actual reading. It holds separate
/// current indices for R1 and R2 (current_index_r1 and current_index_r2). The public method
/// `read(&mut out_r1, &mut out_r2)` fills two provided mutable FastqRecord objects with the
/// next available record (if any) from each file and returns a tuple of Option<()>s indicating
/// whether a record was obtained from each side. When both sides provide a record, their headers
/// are compared (printing a message via eprintln! on mismatch).
pub struct PairedFastqReader {
    // Configuration.
    r1_filename: PathBuf,
    r2_filename: PathBuf,
    batch_size: usize,
    pool_capacity: usize,
    // Channels for each file.
    r1_out: Option<Receiver<Result<Vec<FastqRecord>, Error>>>,
    r2_out: Option<Receiver<Result<Vec<FastqRecord>, Error>>>,
    r1_pool: Option<Sender<Vec<FastqRecord>>>,
    r2_pool: Option<Sender<Vec<FastqRecord>>>,
    // Iteration state.
    current_batch_r1: Option<Vec<FastqRecord>>,
    current_batch_r2: Option<Vec<FastqRecord>>,
    current_index_r1: usize,
    current_index_r2: usize,
}

impl PairedFastqReader {
    /// Constructs a new PairedFastqReader with the given FASTQ file paths.
    /// This does not spawn the reading threads.
    pub fn new(r1_filename: impl AsRef<Path>, r2_filename: impl AsRef<Path>) -> Self {
        Self {
            r1_filename: r1_filename.as_ref().to_path_buf(),
            r2_filename: r2_filename.as_ref().to_path_buf(),
            batch_size: 10,
            pool_capacity: 10,
            r1_out: None,
            r2_out: None,
            r1_pool: None,
            r2_pool: None,
            current_batch_r1: None,
            current_batch_r2: None,
            current_index_r1: 0,
            current_index_r2: 0,
        }
    }

    /// Starts the reading process by spawning worker threads and initializing channels.
    /// This method must be called explicitly.
    pub fn start(&mut self) -> Result<(), Error> {
        // Create bounded output channels for each reader thread.
        let (tx_r1, rx_r1) = crossbeam_channel::bounded::<Result<Vec<FastqRecord>, Error>>(self.pool_capacity);
        let (tx_r2, rx_r2) = crossbeam_channel::bounded::<Result<Vec<FastqRecord>, Error>>(self.pool_capacity);

        // Create pool channels (for recycling empty batch buffers).
        let (pool_tx_r1, pool_rx_r1) = crossbeam_channel::bounded::<Vec<FastqRecord>>(self.pool_capacity);
        let (pool_tx_r2, pool_rx_r2) = crossbeam_channel::bounded::<Vec<FastqRecord>>(self.pool_capacity);

        // Preinitialize batch pools.
        for _ in 0..self.pool_capacity {
            let batch: Vec<FastqRecord> = (0..self.batch_size).map(|_| FastqRecord::new()).collect();
            pool_tx_r1.send(batch.clone())?;
            pool_tx_r2.send(batch)?;
        }

        // Spawn worker threads (assume spawn_reader_thread is defined elsewhere).
        let _handle_r1 = spawn_reader_thread(&self.r1_filename, tx_r1, pool_rx_r1);
        let _handle_r2 = spawn_reader_thread(&self.r2_filename, tx_r2, pool_rx_r2);

        self.r1_out = Some(rx_r1);
        self.r2_out = Some(rx_r2);
        self.current_batch_r1 = None;
        self.current_batch_r2 = None;
        self.current_index_r1 = 0;
        self.current_index_r2 = 0;
        self.r1_pool = Some(pool_tx_r1);
        self.r2_pool = Some(pool_tx_r2);
        Ok(())
    }

    /// Reads the next pair of FASTQ records into the provided mutable output parameters.
    ///
    /// Returns a tuple of Option<()> indicating for each file whether a record was read:
    /// - Some(()) means a new record was successfully filled.
    /// - None means that no record is available (end-of-file in that stream).
    ///
    /// If both streams provide a record, their headers are compared. In case of a mismatch,
    /// a message is printed via eprintln! and an error is returned.
    pub fn read(&mut self, out_r1: &mut FastqRecord, out_r2: &mut FastqRecord)
        -> Result<(Option<()>, Option<()>), Error> 
    {
        out_r1.clear();
        out_r2.clear();

        // For R1: Refill current batch if needed.
        let opt1 = {
            if self.current_batch_r1.is_none() || self.current_index_r1 >= self.current_batch_r1.as_ref().unwrap().len() {
                // Recycle the old batch if present.
                if let Some(batch) = self.current_batch_r1.take() {
                    if let Some(ref pool_tx) = self.r1_pool {
                        pool_tx.send(batch)?;
                    }
                }
                // Fetch the next batch.
                self.current_batch_r1 = Some(match self.r1_out.as_ref().unwrap().recv()? {
                    Ok(batch) => batch,
                    Err(e) => return Err(e),
                });
                self.current_index_r1 = 0;
            }
            let batch = self.current_batch_r1.as_mut().unwrap();
            if self.current_index_r1 < batch.len() {
                std::mem::swap(out_r1, &mut batch[self.current_index_r1]);
                Some(())
            } else {
                None
            }
        };

        // For R2: Independent batch and index.
        let opt2 = {
            if self.current_batch_r2.is_none() || self.current_index_r2 >= self.current_batch_r2.as_ref().unwrap().len() {
                if let Some(batch) = self.current_batch_r2.take() {
                    if let Some(ref pool_tx) = self.r2_pool {
                        pool_tx.send(batch)?;
                    }
                }
                self.current_batch_r2 = Some(match self.r2_out.as_ref().unwrap().recv()? {
                    Ok(batch) => batch,
                    Err(e) => return Err(e),
                });
                self.current_index_r2 = 0;
            }
            let batch = self.current_batch_r2.as_mut().unwrap();
            if self.current_index_r2 < batch.len() {
                std::mem::swap(out_r2, &mut batch[self.current_index_r2]);
                Some(())
            } else {
                None
            }
        };

        // Increment the indices independently.
        if opt1.is_some() {
            self.current_index_r1 += 1;
        }
        if opt2.is_some() {
            self.current_index_r2 += 1;
        }

        // When both sides have produced a record, compare headers.
        if opt1.is_some() && opt2.is_some() {
            if out_r1.header() != out_r2.header() {
                eprintln!("Header mismatch: {} vs {}", out_r1.header(), out_r2.header());
                return Err(anyhow!("Header mismatch: {} vs {}", out_r1.header(), out_r2.header()));
            }
        }
        Ok((opt1, opt2))
    }
}

