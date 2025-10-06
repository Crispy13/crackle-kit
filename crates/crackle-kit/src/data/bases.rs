use std::ops::{Index, Range, RangeFrom, RangeFull, RangeTo};

use anyhow::{Error, anyhow};

const BASE_ARR_LEN: usize = 8;

// we use u64, so the count is 21.
macro_rules! n_bases_in_chunk {
    () => {
        21
    };
}

// New encoding: 0 is now a NULL terminator, and bases start from 1.
const NULL_CODE: u64 = 0b000;
const A_CODE: u64 = 0b001;
const T_CODE: u64 = 0b010;
const C_CODE: u64 = 0b011;
const G_CODE: u64 = 0b100;
const N_CODE: u64 = 0b101;

// --- Compile-time Lookup Table for performance ---
const fn build_lookup_table() -> [u8; 256] {
    let mut table = [0xFF; 256]; // 0xFF is our error sentinel
    table[b'A' as usize] = A_CODE as u8;
    table[b'T' as usize] = T_CODE as u8;
    table[b'C' as usize] = C_CODE as u8;
    table[b'G' as usize] = G_CODE as u8;
    table[b'N' as usize] = N_CODE as u8;
    table
}

const BYTE_TO_CODE_LOOKUP: [u8; 256] = build_lookup_table();

const CODE_TO_CHAR_LOOKUP: [char; 6] = {
    let mut arr = [0 as char; 6];

    arr[A_CODE as usize] = 'A';
    arr[T_CODE as usize] = 'T';
    arr[C_CODE as usize] = 'C';
    arr[G_CODE as usize] = 'G';
    arr[N_CODE as usize] = 'N';
    arr
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct BaseArr {
    inner: [u64; BASE_ARR_LEN],
}

/// A more efficient and safer Display implementation.
/// It iterates through each potential base position and stops
/// as soon as it encounters a NULL_CODE terminator.
impl std::fmt::Display for BaseArr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        'outer: for chunk in self.inner {
            for i in 0..n_bases_in_chunk!() {
                let code = (chunk >> (i * 3)) & 0b111;

                if code == NULL_CODE {
                    break 'outer
                }
                
                let base_char = CODE_TO_CHAR_LOOKUP[code as usize];
                write!(f, "{}", base_char)?;
            }
        }
        Ok(())
    }
}

pub trait BaseArrIndex {
    type Output;
    fn get(self, arr: &BaseArr) -> Option<Self::Output>;
}

impl BaseArrIndex for usize {
    type Output = Base;

    fn get(self, arr: &BaseArr) -> Option<Self::Output> {
        let (idx, offset) = (self / n_bases_in_chunk!(), self % n_bases_in_chunk!());

        // Bounds check
        if idx >= arr.inner.len() {
            return None;
        }

        let code = (arr.inner[idx] >> (offset * 3)) & 0b111; // Mask to get only the 3 bits

        BaseArr::CODE_LOOKUP_TABLE[code as usize]

        // match code {
        //     A_CODE => Some(Base::A),
        //     T_CODE => Some(Base::T),
        //     C_CODE => Some(Base::C),
        //     G_CODE => Some(Base::G),
        //     N_CODE => Some(Base::N),
        //     // NULL_CODE (0) and any other invalid codes will correctly return None.
        //     _ => None,
        // }
    }
}

/// An optimized iterator over the bases in a `BaseArr`.
/// It works chunk-by-chunk to avoid expensive division and modulo in the loop.
pub struct BaseArrIter<'a> {
    arr: &'a BaseArr,
    chunk_index: usize,
    offset_in_chunk: usize,
    total_index: usize,
    end_index: usize,
}

impl<'a> Iterator for BaseArrIter<'a> {
    type Item = Base;

    fn next(&mut self) -> Option<Self::Item> {
        if self.total_index >= self.end_index {
            return None;
        }

        if self.offset_in_chunk >= n_bases_in_chunk!() {
            self.chunk_index += 1;
            self.offset_in_chunk = 0;
        }

        if self.chunk_index >= BASE_ARR_LEN {
            return None;
        }

        let chunk = self.arr.inner[self.chunk_index];
        let code = (chunk >> (self.offset_in_chunk * 3)) & 0b111;

        self.offset_in_chunk += 1;
        self.total_index += 1;

        match BaseArr::CODE_LOOKUP_TABLE[code as usize] {
            Some(b) => Some(b),
            None => {
                self.end_index = 0;
                None
            }
        }

        // match code {
        //     A_CODE => Some(Base::A),
        //     T_CODE => Some(Base::T),
        //     C_CODE => Some(Base::C),
        //     G_CODE => Some(Base::G),
        //     N_CODE => Some(Base::N),
        //     NULL_CODE => {
        //         // Stop the iterator permanently if we hit a null terminator
        //         self.end_index = 0;
        //         None
        //     }
        //     _ => unreachable!(),
        // }
    }
}

impl BaseArr {
    const CODE_LOOKUP_TABLE: [Option<Base>; 6] = {
        let mut arr = [None; 6];

        arr[A_CODE as usize] = Some(Base::A);
        arr[T_CODE as usize] = Some(Base::T);
        arr[C_CODE as usize] = Some(Base::C);
        arr[G_CODE as usize] = Some(Base::G);
        arr[N_CODE as usize] = Some(Base::N);
        // arr[NULL_CODE as usize] = None;

        arr
    };

    const BASE_TO_CODE_TABLE: [u64; 8] = {
        let mut arr = [0; 8];

        arr[Base::A as usize] = A_CODE;
        arr[Base::T as usize] = T_CODE;
        arr[Base::C as usize] = C_CODE;
        arr[Base::G as usize] = G_CODE;
        arr[Base::N as usize] = N_CODE;
        // arr[NULL_CODE as usize] = None;

        arr
    };

    /// Creates a new BaseArr from a slice of bytes (e.g., b"ACGTN...").
    /// Returns an error if the slice is too long to fit.
    pub fn from_bytes1(s: &[u8]) -> Result<Self, Error> {
        let mut inner = [0u64; BASE_ARR_LEN];

        if s.len() > BASE_ARR_LEN * n_bases_in_chunk!() {
            return Err(anyhow!(
                "Input slice is too long: {} bases, max is {}",
                s.len(),
                BASE_ARR_LEN * n_bases_in_chunk!()
            ));
        }

        for (i, &byte) in s.iter().enumerate() {
            // 1. Convert the DNA base byte into its 3-bit u64 code.
            let code = match byte {
                b'A' => A_CODE,
                b'T' => T_CODE,
                b'C' => C_CODE,
                b'G' => G_CODE,
                b'N' => N_CODE,
                _ => return Err(anyhow!("Invalid base '{}' at position {}", byte as char, i)),
            };

            // 2. Calculate where to place the bits.
            let idx = i / n_bases_in_chunk!(); // Which u64 in the array
            let offset = i % n_bases_in_chunk!(); // Which 3-bit slot within that u64

            // 3. Shift the code to the correct position and use bitwise OR
            //    to set the bits without disturbing the other bases.
            inner[idx] |= code << (offset * 3);
        }

        Ok(BaseArr { inner })
    }

    /// Creates a new BaseArr from a slice of bytes using a fast, chunk-based approach.
    pub fn from_bytes(s: &[u8]) -> Result<Self, Error> {
        let max_len = BASE_ARR_LEN * n_bases_in_chunk!();
        if s.len() > max_len {
            return Err(anyhow!(
                "Input slice is too long: {} bases, max is {}",
                s.len(),
                max_len
            ));
        }

        let mut inner = [0u64; BASE_ARR_LEN];

        // Process full chunks of 21 bytes for maximum efficiency
        for (chunk_idx, chunk) in s.chunks(n_bases_in_chunk!()).enumerate() {
            let mut current_u64 = 0u64;
            for (offset, &byte) in chunk.iter().enumerate() {
                // Use the fast lookup table instead of a match statement
                let code = BYTE_TO_CODE_LOOKUP[byte as usize];
                if code == 0xFF {
                    let global_idx = chunk_idx * n_bases_in_chunk!() + offset;
                    return Err(anyhow!(
                        "Invalid base '{}' at position {}",
                        byte as char,
                        global_idx
                    ));
                }
                current_u64 |= (code as u64) << (offset * 3);
            }
            inner[chunk_idx] = current_u64;
        }

        Ok(BaseArr { inner })
    }

    /// Gets the Base at a given index.
    pub fn get<I: BaseArrIndex>(&self, index: I) -> Option<I::Output> {
        index.get(self)
    }

    /// Returns an optimized iterator for a given range.
    /// This is now generic and accepts `start..end`, `start..`, `..end`, and `..`.
    pub fn get_iter<'a, R>(&'a self, range: R) -> BaseArrIter<'a>
    where
        R: BaseArrRange<'a>,
    {
        range.get_iter(self)
    }

    /// Returns an iterator over all the bases in the sequence.
    pub fn iter(&self) -> BaseArrIter<'_> {
        BaseArrIter {
            arr: self,
            chunk_index: 0,
            offset_in_chunk: 0,
            total_index: 0,
            end_index: BASE_ARR_LEN * n_bases_in_chunk!(),
        }
    }

    /// Sets the Base at a given index to a new value.
    pub fn set(&mut self, index: usize, new_base: Base) {
        let (idx, offset) = (index / n_bases_in_chunk!(), index % n_bases_in_chunk!());

        // Bounds check
        if idx >= self.inner.len() {
            panic!("Index out of bounds");
        }

        let bit_pos = offset * 3;

        // 1. Create a mask to clear the 3 bits at the target location.
        //    e.g., for offset 1: `...111111111000111`
        let clear_mask = !(0b111 << bit_pos);
        self.inner[idx] &= clear_mask;

        let new_code = Self::BASE_TO_CODE_TABLE[new_base as usize];

        // // 2. Convert the new Base to its 3-bit code.
        // let new_code = match new_base {
        //     Base::A => A_CODE,
        //     Base::T => T_CODE,
        //     Base::C => C_CODE,
        //     Base::G => G_CODE,
        //     Base::N => N_CODE,
        // };

        // 3. Shift the new code to the correct position and use bitwise OR
        //    to set the new bits.
        self.inner[idx] |= new_code << bit_pos;
    }
}

/// A trait for types that can be used to create an iterator over a `BaseArr`.
pub trait BaseArrRange<'a> {
    fn get_iter(self, arr: &'a BaseArr) -> BaseArrIter<'a>;
}

impl<'a> BaseArrRange<'a> for Range<usize> {
    fn get_iter(self, arr: &'a BaseArr) -> BaseArrIter<'a> {
        let start = self.start;
        let end = self.end;
        let start_chunk = start / n_bases_in_chunk!();
        let start_offset = start % n_bases_in_chunk!();
        BaseArrIter {
            arr,
            chunk_index: start_chunk,
            offset_in_chunk: start_offset,
            total_index: start,
            end_index: end.min(BASE_ARR_LEN * n_bases_in_chunk!()),
        }
    }
}

impl<'a> BaseArrRange<'a> for RangeFrom<usize> {
    fn get_iter(self, arr: &'a BaseArr) -> BaseArrIter<'a> {
        let start = self.start;
        let end = BASE_ARR_LEN * n_bases_in_chunk!();
        let start_chunk = start / n_bases_in_chunk!();
        let start_offset = start % n_bases_in_chunk!();
        BaseArrIter {
            arr,
            chunk_index: start_chunk,
            offset_in_chunk: start_offset,
            total_index: start,
            end_index: end,
        }
    }
}

impl<'a> BaseArrRange<'a> for RangeTo<usize> {
    fn get_iter(self, arr: &'a BaseArr) -> BaseArrIter<'a> {
        BaseArrIter {
            arr,
            chunk_index: 0,
            offset_in_chunk: 0,
            total_index: 0,
            end_index: self.end.min(BASE_ARR_LEN * n_bases_in_chunk!()),
        }
    }
}

impl<'a> BaseArrRange<'a> for RangeFull {
    fn get_iter(self, arr: &'a BaseArr) -> BaseArrIter<'a> {
        arr.iter()
    }
}

// Added Clone and Copy, which are necessary for the tests to work correctly.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Base {
    A,
    T,
    C,
    G,
    N,
}

impl Base {
    const BYTE_TO_BASE_TABLE: [Option<Self>; 256] = {
        {
            let mut table = [None; 256]; // 0xFF is our error sentinel
            table[b'A' as usize] = Some(Self::A);
            table[b'T' as usize] = Some(Self::T);
            table[b'C' as usize] = Some(Self::C);
            table[b'G' as usize] = Some(Self::G);
            table[b'N' as usize] = Some(Self::N);
            table
        }
    };

    // const STRING_LOOKUP_STABLE: [std::string::String; 256] = {
    //     let mut table = [const { String::new() }; 256];
        
    //     table[Self::A as usize].push_str("A");
    // }
}

impl TryFrom<u8> for Base {
    type Error = Error;

    // fn try_from(value: u8) -> Result<Self, Self::Error> {
    //     let r = match value {
    //         b'A' => Self::A,
    //         b'C' => Self::C,
    //         b'T' => Self::T,
    //         b'G' => Self::G,
    //         b'N' => Self::N,
    //         oth => Err(anyhow!("Invalid base: {}", oth as char))?,
    //     };

    //     Ok(r)
    // }
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let r = match Self::BYTE_TO_BASE_TABLE[value as usize] {
            Some(b) => b,
            None => Err(anyhow!("Invalid base: {}", value as char))?,
        };

        Ok(r)
    }
}

// --- TEST FUNCTIONS ---
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_get_simple() -> Result<(), Error> {
        let seq = b"ATCGN"; // Corrected sequence for clarity
        let arr = BaseArr::from_bytes(seq)?;

        assert_eq!(arr.get(0), Some(Base::A));
        assert_eq!(arr.get(1), Some(Base::T));
        assert_eq!(arr.get(2), Some(Base::C));
        assert_eq!(arr.get(3), Some(Base::G));
        assert_eq!(arr.get(4), Some(Base::N));
        // Uninitialized bits are now 0 (NULL_CODE), so get should return None.
        assert_eq!(arr.get(5), None, "Uninitialized bits should be None");

        Ok(())
    }

    #[test]
    fn test_get_across_u64_boundary() -> Result<(), Error> {
        // Create a sequence that is guaranteed to cross the 21-base boundary
        let mut seq_bytes = Vec::with_capacity(25);
        for _ in 0..20 {
            seq_bytes.push(b'C');
        } // 20 'C's
        seq_bytes.push(b'G'); // Index 20
        seq_bytes.push(b'T'); // Index 21
        seq_bytes.push(b'A'); // Index 22

        let arr = BaseArr::from_bytes(&seq_bytes)?;

        assert_eq!(arr.get(19), Some(Base::C));
        assert_eq!(
            arr.get(20),
            Some(Base::G),
            "Should get correct base at u64 boundary"
        );
        assert_eq!(
            arr.get(21),
            Some(Base::T),
            "Should get correct base after u64 boundary"
        );
        assert_eq!(arr.get(22), Some(Base::A));

        Ok(())
    }

    #[test]
    fn test_set_and_get() -> Result<(), Error> {
        let initial_seq = b"AAAAAAAAAAAAAAAAAAAAA"; // 21 'A's
        let mut arr = BaseArr::from_bytes(initial_seq)?;

        // Check initial state
        assert_eq!(arr.get(1), Some(Base::A));
        assert_eq!(arr.get(5), Some(Base::A));
        assert_eq!(arr.get(20), Some(Base::A));

        // Set a few values
        arr.set(1, Base::G);
        arr.set(5, Base::N);
        arr.set(20, Base::C);

        // Verify changes and that other bases are unaffected
        assert_eq!(arr.get(0), Some(Base::A));
        assert_eq!(
            arr.get(1),
            Some(Base::G),
            "Base at index 1 should be updated to G"
        );
        assert_eq!(arr.get(2), Some(Base::A));
        assert_eq!(arr.get(4), Some(Base::A));
        assert_eq!(
            arr.get(5),
            Some(Base::N),
            "Base at index 5 should be updated to N"
        );
        assert_eq!(arr.get(6), Some(Base::A));
        assert_eq!(arr.get(19), Some(Base::A));
        assert_eq!(
            arr.get(20),
            Some(Base::C),
            "Base at index 20 should be updated to C"
        );

        Ok(())
    }

    #[test]
    fn test_new_invalid_character() {
        let seq = b"ACGT_Z";
        let err = BaseArr::from_bytes(seq).unwrap_err();
        // The error should be about the first invalid character, which is '_' at index 4.
        let expected_msg = "Invalid base '_' at position 4";
        assert!(
            err.to_string().contains(expected_msg),
            "Expected error message to contain '{}', but got '{}'",
            expected_msg,
            err
        );
    }

    #[test]
    fn test_new_too_long() {
        let seq = vec![b'A'; 200]; // Max is 8 * 21 = 168
        let result = BaseArr::from_bytes(&seq);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Input slice is too long")
        );
    }

    #[test]
    #[should_panic(expected = "Index out of bounds")]
    fn test_set_out_of_bounds() {
        let mut arr = BaseArr::from_bytes(b"A").unwrap();
        arr.set(200, Base::C); // This should panic
    }

    #[test]
    fn test_get_out_of_bounds() {
        let arr = BaseArr::from_bytes(b"ACGT").unwrap();
        assert_eq!(arr.get(200), None);
    }

    #[test]
    fn test_to_string_impl() -> Result<(), Box<dyn std::error::Error>> {
        let v = b"ACCTG";
        let r = BaseArr::from_bytes(v)?;
        assert_eq!(r.to_string(), "ACCTG");

        let v_long = b"ACCTGACCTGACCTGACCTGACCTG"; // 25 bases
        let r_long = BaseArr::from_bytes(v_long)?;
        assert_eq!(r_long.to_string(), "ACCTGACCTGACCTGACCTGACCTG");

        Ok(())
    }

    #[test]
    fn test_get_iter() -> Result<(), Error> {
        let seq = b"ATCGNATCGN"; // 10 bases
        let arr = BaseArr::from_bytes(seq)?;

        // Test a sub-slice (Range)
        let sub: Vec<Base> = arr.get_iter(2..6).collect();
        assert_eq!(sub, vec![Base::C, Base::G, Base::N, Base::A]);

        // Test a slice that goes to the end (RangeFrom)
        let sub_to_end: Vec<Base> = arr.get_iter(7..).collect();
        assert_eq!(sub_to_end, vec![Base::C, Base::G, Base::N]);

        // Test a slice from the beginning (RangeTo)
        let sub_from_start: Vec<Base> = arr.get_iter(..3).collect();
        assert_eq!(sub_from_start, vec![Base::A, Base::T, Base::C]);

        // Test a full slice (RangeFull)
        let sub_full: Vec<Base> = arr.get_iter(..).collect();
        assert_eq!(
            sub_full,
            vec![
                Base::A,
                Base::T,
                Base::C,
                Base::G,
                Base::N,
                Base::A,
                Base::T,
                Base::C,
                Base::G,
                Base::N
            ]
        );

        Ok(())
    }

    #[test]
    fn test_long_sequence_operations() -> Result<(), Error> {
        // Create a long sequence (50 bases) that spans multiple u64 chunks.
        let mut original_bytes = b"ATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGN".to_vec();
        assert_eq!(original_bytes.len(), 50);

        let mut arr = BaseArr::from_bytes(&original_bytes)?;

        // 1. Verify `from_bytes` and `get` for the entire long sequence.
        for i in 0..original_bytes.len() {
            let expected_base = Base::try_from(original_bytes[i])?;
            assert_eq!(arr.get(i), Some(expected_base), "Mismatch at index {}", i);
        }

        // 2. Verify `set` at multiple positions, including across chunk boundaries.
        // Boundary between chunk 0 and 1 is at index 21.
        // Boundary between chunk 1 and 2 is at index 42.
        arr.set(5, Base::T);
        original_bytes[5] = b'T';
        arr.set(21, Base::C);
        original_bytes[21] = b'C';
        arr.set(45, Base::G);
        original_bytes[45] = b'G';

        assert_eq!(arr.get(5), Some(Base::T));
        assert_eq!(arr.get(21), Some(Base::C));
        assert_eq!(arr.get(45), Some(Base::G));
        // Verify that a non-modified base is still correct.
        assert_eq!(arr.get(10), Some(Base::A));

        // 3. Verify `get_iter` over a range spanning chunks.
        let sub_seq: Vec<Base> = arr.get_iter(20..25).collect();
        let expected_sub_seq: Vec<Base> = original_bytes[20..25]
            .iter()
            .map(|&b| Base::try_from(b).unwrap())
            .collect();
        assert_eq!(sub_seq, expected_sub_seq);

        // 4. Verify `to_string` for the modified long sequence.
        let expected_string = std::str::from_utf8(&original_bytes)?.to_string();
        assert_eq!(arr.to_string(), expected_string);

        Ok(())
    }
}
