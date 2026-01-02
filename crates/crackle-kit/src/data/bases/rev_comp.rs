use std::arch::x86_64::{
    __m256i, _mm256_loadu_si256, _mm256_permute2x128_si256, _mm256_shuffle_epi8,
    _mm256_storeu_si256,
};

use crate::data::bases::comp::complement_base;

// Same constants as before
pub(crate) const SIMD_COMPLEMENT_LUT: [u8; 32] = [
    0, 'T' as u8, 0, 'G' as u8, 'A' as u8, 'A' as u8, 0, 'C' as u8, 0, 0, 0, 0, 0, 0, 'N' as u8, 0,
    0, 'T' as u8, 0, 'G' as u8, 'A' as u8, 'A' as u8, 0, 'C' as u8, 0, 0, 0, 0, 0, 0, 'N' as u8, 0,
];

pub(crate) const SIMD_REV_MASK: [u8; 32] = [
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, // lane 1
    15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, // lane 2
];

#[inline]
fn reverse_complement_scalar(src: &[u8], dst: &mut [u8]) {
    let len = src.len();
    for (k, &b) in src.iter().enumerate() {
        let comp = complement_base(b);
        // Fill from the end of the provided slice
        dst[len - 1 - k] = comp;
    }
}

#[inline]
fn reverse_complement_scalar_ptr(src: &[u8], dst: *mut u8) {
    let len = src.len();
    for (k, &b) in src.iter().enumerate() {
        let comp = complement_base(b);

        let offset = len - 1 - k;
        // Fill from the end of the provided slice
        unsafe { *dst.add(offset) = comp };
    }
}

#[target_feature(enable = "avx2")]
unsafe fn reverse_complement_avx2(src: &[u8], dst_ptr: *mut u8) {
    let len = src.len();
    let mut i = 0;

    unsafe {
        // Load lookup table and reverse mask
        let lut = _mm256_loadu_si256(SIMD_COMPLEMENT_LUT.as_ptr() as *const _);
        let rev_mask = _mm256_loadu_si256(SIMD_REV_MASK.as_ptr() as *const _);

        // Process 32 bytes at a time
        while i + 32 <= len {
            // 1. Load 32 bytes
            let input = _mm256_loadu_si256(src.as_ptr().add(i) as *const _);

            // 2. Complement
            let complemented = _mm256_shuffle_epi8(lut, input);

            // 3. Reverse bytes inside lanes
            let reversed_lanes = _mm256_shuffle_epi8(complemented, rev_mask);

            // 4. Swap lanes
            let final_chunk = _mm256_permute2x128_si256(reversed_lanes, reversed_lanes, 0x01);

            // 5. Store to dst_ptr
            // Math: We write to the end. (Base + Length - Current_Progress - Chunk_Size)
            let write_offset = len - i - 32;
            _mm256_storeu_si256(dst_ptr.add(write_offset) as *mut _, final_chunk);

            i += 32;
        }

        // Handle remaining bytes (Scalar Fallback)
        if i < len {
            let remaining = len - i;

            // Scalar input: The remaining bytes at the END of src
            let src_rem = &src[i..];

            // Scalar output: The remaining space at the START of dst
            // Since we filled from the back, the "hole" is at index 0 to remaining.
            // We create a temporary mutable slice from the raw pointer for the scalar function.
            let dst_rem = std::slice::from_raw_parts_mut(dst_ptr, remaining);

            reverse_complement_scalar(src_rem, dst_rem);
        }
    }
}

#[target_feature(enable = "avx2")]
unsafe fn rc_avx2_unrolled(mut src_ptr: *const u8, dst_base: *mut u8, len: usize) {
    let mut i = 0;

    unsafe {
        // Destination pointer starts at the END of the buffer
        let mut dst_ptr = dst_base.add(len);

        let lut = _mm256_loadu_si256(SIMD_COMPLEMENT_LUT.as_ptr() as *const _);
        let rev_mask = _mm256_loadu_si256(SIMD_REV_MASK.as_ptr() as *const _);

        // OPTIMIZATION 2: Unroll loop 4x (128 bytes per iter)
        while i + 128 <= len {
            // Move destination pointer back by 128 bytes (prep for writing 4 chunks)
            dst_ptr = dst_ptr.sub(128);

            // 1. Load 4 chunks (A, B, C, D)
            let chunk_a = _mm256_loadu_si256(src_ptr.add(0) as *const _);
            let chunk_b = _mm256_loadu_si256(src_ptr.add(32) as *const _);
            let chunk_c = _mm256_loadu_si256(src_ptr.add(64) as *const _);
            let chunk_d = _mm256_loadu_si256(src_ptr.add(96) as *const _);

            // 2. Process them (Complement + Reverse Inner)
            // Inline the logic manually or use a helper macro/closure
            let process = |input| {
                let comp = _mm256_shuffle_epi8(lut, input);
                let rev = _mm256_shuffle_epi8(comp, rev_mask);
                _mm256_permute2x128_si256(rev, rev, 0x01)
            };

            let res_a = process(chunk_a);
            let res_b = process(chunk_b);
            let res_c = process(chunk_c);
            let res_d = process(chunk_d);

            // 3. Store in REVERSE order
            // Input: [A, B, C, D] -> Output in mem must be: [Rev(D), Rev(C), Rev(B), Rev(A)]
            _mm256_storeu_si256(dst_ptr.add(0) as *mut _, res_d);
            _mm256_storeu_si256(dst_ptr.add(32) as *mut _, res_c);
            _mm256_storeu_si256(dst_ptr.add(64) as *mut _, res_b);
            _mm256_storeu_si256(dst_ptr.add(96) as *mut _, res_a);

            src_ptr = src_ptr.add(128);
            i += 128;
        }

        // Handle remaining blocks (32 bytes at a time)
        while i + 32 <= len {
            dst_ptr = dst_ptr.sub(32);
            let input = _mm256_loadu_si256(src_ptr as *const _);

            let comp = _mm256_shuffle_epi8(lut, input);
            let rev_lanes = _mm256_shuffle_epi8(comp, rev_mask);
            let final_chunk = _mm256_permute2x128_si256(rev_lanes, rev_lanes, 0x01);

            _mm256_storeu_si256(dst_ptr as *mut _, final_chunk);

            src_ptr = src_ptr.add(32);
            i += 32;
        }

        // Handle tail (scalar fallback)
        if i < len {
            // We need to construct a slice for the remaining scalar part
            let remaining = len - i;
            let src_slice = std::slice::from_raw_parts(src_ptr, remaining);

            // For the destination, we are writing to the *beginning* of the remaining space in the buffer.
            // Since we filled from the end backwards, the "hole" is at the start of result buffer.
            // Wait! dst_ptr is currently at location 'len - i'.
            // The remaining unwritten zone is [0 .. len-i].
            // We must write to dst_base (offset 0).
            let dst_slice = std::slice::from_raw_parts_mut(dst_base, remaining);

            reverse_complement_scalar(src_slice, dst_slice);
        }
    }
}

pub enum RevCompMode {
    Normal,
    NormalPtr,
    SIMD,
    SIMDUnrolled4x,
}

pub struct RevComplementor {
    mode: RevCompMode,
    buf: Vec<u8>,
}

impl RevComplementor {
    pub fn new() -> RevComplementor {
        let mode = if is_x86_feature_detected!("avx2") {
            RevCompMode::SIMD
        } else {
            RevCompMode::Normal
        };

        Self {
            mode,
            buf: Default::default(),
        }
    }

    pub fn with_mode(mode: RevCompMode) -> RevComplementor {
        match mode {
            RevCompMode::Normal => {}
            RevCompMode::NormalPtr => {}
            RevCompMode::SIMD | RevCompMode::SIMDUnrolled4x => {
                if !is_x86_feature_detected!("avx2") {
                    panic!("avx2 feature is not detected.")
                }
            }
        }

        Self {
            mode,
            buf: Default::default(),
        }
    }

    pub fn reverse_complement(&mut self, seq: &[u8]) -> &[u8] {
        self.buf.clear();

        if self.buf.capacity() < seq.len() {
            self.buf.reserve(seq.len());
            debug_assert!(self.buf.capacity() >= seq.len());
        }

        match self.mode {
            RevCompMode::SIMD => {
                unsafe {
                    reverse_complement_avx2(seq, self.buf.as_mut_ptr());
                    self.buf.set_len(seq.len());
                };
            }
            RevCompMode::SIMDUnrolled4x => {
                unsafe {
                    rc_avx2_unrolled(seq.as_ptr(), self.buf.as_mut_ptr(), seq.len());
                    self.buf.set_len(seq.len());
                };
            }
            RevCompMode::NormalPtr => {
                unsafe {
                    reverse_complement_scalar_ptr(seq, self.buf.as_mut_ptr());
                    self.buf.set_len(seq.len());
                };
            }
            RevCompMode::Normal => {
                self.buf.resize(seq.len(), 0);
                reverse_complement_scalar(seq, &mut self.buf);
            }
        }

        self.buf.as_slice()
    }

    pub fn set_buf_capacity(&mut self, n: usize) {
        self.buf.reserve(n - self.buf.len());
    }
}

#[cfg(test)]
mod rc_tests {
    use super::*;

    #[test]
    fn test_res_equality() {
        use rand::Rng; // Ensure rand is in dev-dependencies

        let mut rng = rand::thread_rng();

        // 1. Setup Runners
        let mut normal_runner = RevComplementor::with_mode(RevCompMode::Normal);
        let mut normal_ptr_runner = RevComplementor::with_mode(RevCompMode::NormalPtr);

        // Only run SIMD checks if the CPU supports it
        let mut simd_runner = if is_x86_feature_detected!("avx2") {
            Some(RevComplementor::with_mode(RevCompMode::SIMD))
        } else {
            println!("Skipping SIMD test: AVX2 not detected on this machine.");
            None
        };

        // Only run SIMD checks if the CPU supports it
        let mut simd_unrolled_runner = if is_x86_feature_detected!("avx2") {
            Some(RevComplementor::with_mode(RevCompMode::SIMDUnrolled4x))
        } else {
            // println!("Skipping SIMD test: AVX2 not detected on this machine.");
            None
        };

        // 2. Define lengths to test boundaries
        // 0: Empty
        // 31: Pure scalar fallback in SIMD
        // 32: Exact one AVX chunk
        // 33: One chunk + 1 byte tail
        // 150: Common NGS read length
        // 1000: Larger block
        let lengths = [0, 1, 15, 31, 32, 33, 64, 127, 128, 129, 150, 256, 1000];

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

            // 3. Run Normal Implementation
            normal_runner.reverse_complement(&input);
            let res_normal = normal_runner.buf.clone();

            normal_ptr_runner.reverse_complement(&input);
            let res_normal_ptr = normal_ptr_runner.buf.clone();

            assert_eq!(
                res_normal,
                res_normal_ptr,
                "Mismatch at length {}.\nInput: {}\nNormal: {}\nSIMD:   {}",
                len,
                String::from_utf8_lossy(&input),
                String::from_utf8_lossy(&res_normal),
                String::from_utf8_lossy(&res_normal_ptr)
            );

            // 4. Run SIMD Implementation and Compare
            if let Some(simd) = &mut simd_runner {
                simd.reverse_complement(&input);

                assert_eq!(
                    res_normal,
                    simd.buf,
                    "Mismatch at length {}.\nInput: {}\nNormal: {}\nSIMD:   {}",
                    len,
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(&res_normal),
                    String::from_utf8_lossy(&simd.buf)
                );
            }

            if let Some(simd) = &mut simd_unrolled_runner {
                simd.reverse_complement(&input);

                assert_eq!(
                    res_normal,
                    simd.buf,
                    "Mismatch at length {}.\nInput: {}\nNormal: {}\nSIMD:   {}",
                    len,
                    String::from_utf8_lossy(&input),
                    String::from_utf8_lossy(&res_normal),
                    String::from_utf8_lossy(&simd.buf)
                );
            }
        }
    }
}
