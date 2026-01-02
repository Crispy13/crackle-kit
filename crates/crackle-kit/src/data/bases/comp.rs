use std::arch::x86_64::{_mm256_loadu_si256, _mm256_shuffle_epi8, _mm256_storeu_si256};

use crate::data::bases::rev_comp::SIMD_COMPLEMENT_LUT;

#[inline]
pub fn complement_base(b: u8) -> u8 {
    match b {
        b'A' | b'a' => b'T',
        b'C' | b'c' => b'G',
        b'G' | b'g' => b'C',
        b'T' | b't' => b'A',
        b'N' | b'n' => b'N',
        _ => b'N',
    }
}

fn complement_seq_scalar(seq: &[u8], dst: &mut [u8]) {
    for (k, &b) in seq.iter().enumerate() {
        let comp = complement_base(b);
        // Fill from the end of the provided slice
        dst[k] = comp;
    }
}

#[target_feature(enable = "avx2")]
unsafe fn complement_avx2(src: &[u8], dst_ptr: *mut u8) {
    let len = src.len();
    let mut i = 0;

    unsafe {
        // Load lookup table and reverse mask
        let lut = _mm256_loadu_si256(SIMD_COMPLEMENT_LUT.as_ptr() as *const _);

        // Process 32 bytes at a time
        while i + 32 <= len {
            // 1. Load 32 bytes
            let input = _mm256_loadu_si256(src.as_ptr().add(i) as *const _);

            // 2. Complement
            let complemented = _mm256_shuffle_epi8(lut, input);

            _mm256_storeu_si256(dst_ptr.add(i) as *mut _, complemented);

            i += 32;
        }

        // Handle remaining bytes (Scalar Fallback)
        if i < len {
            let remaining = len - i;

            // Scalar input: The remaining bytes at the END of src
            let src_rem = &src[i..];

            let dst_rem = std::slice::from_raw_parts_mut(dst_ptr.add(i), remaining);

            complement_seq_scalar(src_rem, dst_rem);
        }
    }
}

pub enum CompMode {
    Scalar,
    SIMD,
}

/// Sequence complementor
///
/// e.g. A->T G->C T->A C->G
pub struct Complementor {
    mode: CompMode,
    buf: Vec<u8>,
}

impl Complementor {
    pub fn new(mode: CompMode) -> Self {
        if let CompMode::SIMD = mode {
            if !is_x86_feature_detected!("avx2") {
                panic!("AVX2 not detected");
            }
        }
        Self {
            mode,
            buf: Default::default(),
        }
    }

    pub fn complement(&mut self, seq: &[u8]) -> &[u8] {
        let len = seq.len();
        self.buf.clear();
        if self.buf.capacity() < len {
            self.buf.reserve(len);
        }

        match self.mode {
            CompMode::Scalar => {
                self.buf.resize(len, 0);
                complement_seq_scalar(seq, &mut self.buf);
            }
            CompMode::SIMD => unsafe {
                // Pass the raw pointer to the SIMD function
                complement_avx2(seq, self.buf.as_mut_ptr());
                // Critical: Manually update length since we wrote via pointer
                self.buf.set_len(len);
            },
        }
        self.buf.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import complement_avx2, complement_seq_scalar
    use rand::Rng;

    // --- Wrapper for Testing ---

    // --- The Equality Test ---
    #[test]
    fn test_complement_equality() {
        let mut rng = rand::thread_rng();

        // 1. Setup Runners
        let mut scalar_runner = Complementor::new(CompMode::Scalar);

        // Only run SIMD checks if CPU supports it
        let mut simd_runner = if is_x86_feature_detected!("avx2") {
            Some(Complementor::new(CompMode::SIMD))
        } else {
            println!("Skipping SIMD test: AVX2 not detected.");
            None
        };

        // 2. Define lengths to test boundaries
        // 0: Empty
        // 31: Pure scalar fallback
        // 32: Exact one AVX chunk
        // 33: One chunk + 1 byte tail (Tests your fix!)
        // 1000: Larger block
        let lengths = [0, 1, 15, 31, 32, 33, 64, 65, 127, 128, 150, 1000];

        for len in lengths {
            // Generate Random DNA
            let input: Vec<u8> = (0..len)
                .map(|_| match rng.gen_range(0..10) {
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
                .collect();

            // 3. Run Scalar
            let res_scalar = scalar_runner.complement(&input).to_vec();

            // 4. Run SIMD and Compare
            if let Some(simd) = &mut simd_runner {
                let res_simd = simd.complement(&input).to_vec();

                eprintln!(
                    "{} {} {}",
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(&res_scalar),
                    String::from_utf8_lossy(&res_simd)
                );

                assert_eq!(
                    res_scalar,
                    res_simd,
                    "Mismatch at length {}.\nInput:  {}\nScalar: {}\nSIMD:   {}",
                    len,
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(&res_scalar),
                    String::from_utf8_lossy(&res_simd)
                );
            }
        }
    }
}
