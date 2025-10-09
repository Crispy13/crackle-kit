use std::ops::{Index, Range, RangeFrom, RangeFull, RangeTo};

use anyhow::{Error, anyhow};

const BASE_ARR_LEN: usize = 8;

// we use u64, so the count is 21.
macro_rules! n_bases_in_u64_chunk {
    () => {
        21
    };
}

macro_rules! n_bases_in_u16_chunk {
    () => {
        5
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
pub struct BaseArr<C = u64, const N: usize = BASE_ARR_LEN> {
    inner: [C; N],
}

macro_rules! impl_display {
    ($type:ty, $n_bases_in_chunk:expr) => {
        /// A more efficient and safer Display implementation.
        /// It iterates through each potential base position and stops
        /// as soon as it encounters a NULL_CODE terminator.
        impl<const N:usize> std::fmt::Display for BaseArr<$type, N> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                'outer: for chunk in self.inner {
                    for i in 0..$n_bases_in_chunk {
                        let code = (chunk >> (i * 3)) & 0b111;

                        if code == NULL_CODE as $type {
                            break 'outer;
                        }

                        let base_char = CODE_TO_CHAR_LOOKUP[code as usize];
                        write!(f, "{}", base_char)?;
                    }
                }
                Ok(())
            }
        }
    };
}

impl_display!(u16, n_bases_in_u16_chunk!());
impl_display!(u64, n_bases_in_u64_chunk!());

pub trait BaseArrIndex<C, const N: usize> {
    type Output;
    fn get(self, arr: &BaseArr<C, N>) -> Option<Self::Output>;
}

macro_rules! impl_basearr_idx {
    ($type:ty, $n_bases_in_chunk:expr) => {
        impl<const N: usize> BaseArrIndex<$type, N> for usize {
            type Output = Base;

            fn get(self, arr: &BaseArr<$type, N>) -> Option<Self::Output> {
                let (idx, offset) = (self / $n_bases_in_chunk, self % $n_bases_in_chunk);

                // Bounds check
                if idx >= arr.inner.len() {
                    return None;
                }

                let code = (arr.inner[idx] >> (offset * 3)) & 0b111; // Mask to get only the 3 bits

                BaseArr::<$type>::CODE_TO_BASE_LOOKUP[code as usize]

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
    };
}

impl_basearr_idx!(u16, n_bases_in_u16_chunk!());
impl_basearr_idx!(u64, n_bases_in_u64_chunk!());

/// A high-performance iterator that avoids division/modulo in the `next` method.
pub struct BaseArrIter<'a, C, const N: usize> {
    arr: &'a BaseArr<C, N>,
    chunk_index: usize,
    offset_in_chunk: usize,
    total_index: usize,
    end_index: usize,
}

macro_rules! impl_basearr_iter {
    ($type:ty, $n_bases_in_chunk:expr) => {
        impl<'a, const N: usize> BaseArrIter<'a, $type, N> {
            /// Creates a new iterator for a given range.
            fn new(arr: &'a BaseArr<$type, N>, start: usize, end: usize) -> Self {
                let end = end.min(N * $n_bases_in_chunk);
                Self {
                    arr,
                    chunk_index: start / $n_bases_in_chunk,
                    offset_in_chunk: start % $n_bases_in_chunk,
                    total_index: start,
                    end_index: end,
                }
            }
        }

        impl<'a, const N: usize> Iterator for BaseArrIter<'a, $type, N> {
            type Item = Base;

            #[inline]
            fn next(&mut self) -> Option<Self::Item> {
                if self.total_index >= self.end_index {
                    return None;
                }

                if self.offset_in_chunk >= $n_bases_in_chunk {
                    self.chunk_index += 1;
                    self.offset_in_chunk = 0;
                }

                if self.chunk_index >= N {
                    return None;
                }

                let chunk = self.arr.inner[self.chunk_index];
                let code = (chunk >> (self.offset_in_chunk * 3)) & 0b111;

                self.offset_in_chunk += 1;
                self.total_index += 1;

                let base = BaseArr::<$type>::CODE_TO_BASE_LOOKUP[code as usize];
                if base.is_none() {
                    // Found a NULL terminator, stop the iterator permanently.
                    self.end_index = 0;
                }
                base
            }
        }
    };
}

impl_basearr_iter!(u16, n_bases_in_u16_chunk!());
impl_basearr_iter!(u64, n_bases_in_u64_chunk!());

impl<C, const N: usize> BaseArr<C, N> {
    const CODE_TO_BASE_LOOKUP: [Option<Base>; 6] = {
        let mut arr = [None; 6];

        arr[A_CODE as usize] = Some(Base::A);
        arr[T_CODE as usize] = Some(Base::T);
        arr[C_CODE as usize] = Some(Base::C);
        arr[G_CODE as usize] = Some(Base::G);
        arr[N_CODE as usize] = Some(Base::N);
        // arr[NULL_CODE as usize] = None;

        arr
    };
}

macro_rules! impl_basearr {
    ($type:ty, $n_bases_in_chunk:expr) => {
        impl<const N: usize> BaseArr<$type, N> {
            const BASE_TO_CODE_TABLE: [$type; 8] = {
                let mut arr = [0; 8];

                arr[Base::A as usize] = A_CODE as $type;
                arr[Base::T as usize] = T_CODE as $type;
                arr[Base::C as usize] = C_CODE as $type;
                arr[Base::G as usize] = G_CODE as $type;
                arr[Base::N as usize] = N_CODE as $type;
                // arr[NULL_CODE as usize] = None;

                arr
            };
            /// Creates a new `BaseArr` from any iterator of bytes.
            pub fn from_iter(iter: impl IntoIterator<Item = u8>) -> Result<Self, Error> {
                let mut inner = [0; N];
                let mut iter = iter.into_iter().peekable();

                for chunk_idx in 0..N {
                    if iter.peek().is_none() {
                        break; // Stop if the iterator is empty
                    }

                    let mut current_u64 = 0;
                    for i in 0..$n_bases_in_chunk {
                        if let Some(byte) = iter.next() {
                            let code = BYTE_TO_CODE_LOOKUP[byte as usize];
                            if code == 0xFF {
                                let global_idx = chunk_idx * $n_bases_in_chunk + i;
                                return Err(anyhow!(
                                    "Invalid base '{}' at position {}",
                                    byte as char,
                                    global_idx
                                ));
                            }
                            current_u64 |= (code as $type) << (i * 3);
                        } else {
                            break; // Stop if the chunk is not full
                        }
                    }
                    inner[chunk_idx] = current_u64;
                }

                // Check if there is still data left in the iterator, which means it's too long.
                if iter.peek().is_some() {
                    let max_len = N * $n_bases_in_chunk;
                    return Err(anyhow!("Input iterator is too long, max is {}", max_len));
                }

                Ok(BaseArr { inner })
            }

            /// Creates a new BaseArr from a slice of bytes using a fast, chunk-based approach.
            pub fn from_bytes(s: &[u8]) -> Result<Self, Error> {
                let max_len = N * $n_bases_in_chunk;
                if s.len() > max_len {
                    return Err(anyhow!(
                        "Input slice is too long: {} bases, max is {}",
                        s.len(),
                        max_len
                    ));
                }

                let mut inner = [0; N];

                // Process full chunks of 21 bytes for maximum efficiency
                for (chunk_idx, chunk) in s.chunks($n_bases_in_chunk).enumerate() {
                    let mut current_u64 = 0;
                    for (offset, &byte) in chunk.iter().enumerate() {
                        // Use the fast lookup table instead of a match statement
                        let code = BYTE_TO_CODE_LOOKUP[byte as usize];
                        if code == 0xFF {
                            let global_idx = chunk_idx * $n_bases_in_chunk + offset;
                            return Err(anyhow!(
                                "Invalid base '{}' at position {}",
                                byte as char,
                                global_idx
                            ));
                        }
                        current_u64 |= (code as $type) << (offset * 3);
                    }
                    inner[chunk_idx] = current_u64;
                }

                Ok(BaseArr { inner })
            }

            /// Gets the Base at a given index.
            pub fn get<I: BaseArrIndex<$type, N>>(&self, index: I) -> Option<I::Output> {
                index.get(self)
            }

            /// Returns an optimized iterator for a given range.
            /// This is now generic and accepts `start..end`, `start..`, `..end`, and `..`.
            pub fn get_iter<'a, R>(&'a self, range: R) -> BaseArrIter<'a, $type, N>
            where
                R: BaseArrRange<'a, $type, N>,
            {
                range.get_iter(self)
            }

            /// Returns an iterator over all the bases in the sequence.
            pub fn iter(&self) -> BaseArrIter<'_, $type, N> {
                BaseArrIter::<$type, N>::new(self, 0, N * $n_bases_in_chunk)
            }

            /// Sets the Base at a given index to a new value.
            pub fn set(&mut self, index: usize, new_base: Base) {
                let (idx, offset) = (index / $n_bases_in_chunk, index % $n_bases_in_chunk);

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
    };
}

impl_basearr!(u16, n_bases_in_u16_chunk!());
impl_basearr!(u64, n_bases_in_u64_chunk!());

/// A trait for range types that can be used to create an iterator over a `BaseArr`.
pub trait BaseArrRange<'a, C, const N: usize> {
    fn get_iter(self, arr: &'a BaseArr<C, N>) -> BaseArrIter<'a, C, N>;
}

macro_rules! impl_base_arr_range {
    ($type:ty, $n_bases_in_chunk:expr) => {
        impl<'a, const N: usize> BaseArrRange<'a, $type, N> for Range<usize> {
            fn get_iter(self, arr: &'a BaseArr<$type, N>) -> BaseArrIter<'a, $type, N> {
                BaseArrIter::<$type, N>::new(arr, self.start, self.end)
            }
        }

        impl<'a, const N: usize> BaseArrRange<'a, $type, N> for RangeFrom<usize> {
            fn get_iter(self, arr: &'a BaseArr<$type, N>) -> BaseArrIter<'a, $type, N> {
                BaseArrIter::<$type, N>::new(arr, self.start, N * $n_bases_in_chunk)
            }
        }

        impl<'a, const N: usize> BaseArrRange<'a, $type, N> for RangeTo<usize> {
            fn get_iter(self, arr: &'a BaseArr<$type, N>) -> BaseArrIter<'a, $type, N> {
                BaseArrIter::<$type, N>::new(arr, 0, self.end)
            }
        }

        impl<'a, const N: usize> BaseArrRange<'a, $type, N> for RangeFull {
            fn get_iter(self, arr: &'a BaseArr<$type, N>) -> BaseArrIter<'a, $type, N> {
                arr.iter()
            }
        }
    };
}

impl_base_arr_range!(u16, n_bases_in_u16_chunk!());
impl_base_arr_range!(u64, n_bases_in_u64_chunk!());

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

    macro_rules! make_test_functions {
        ($type_name:ident, $type:ty) => {
            mod $type_name {
                use super::*;

                #[test]
                fn test_new_and_get_simple() -> Result<(), Error> {
                    let seq = b"ATCGN"; // Corrected sequence for clarity
                    let arr = BaseArr::<$type>::from_bytes(seq)?;

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

                    let arr = BaseArr::<$type>::from_bytes(&seq_bytes)?;

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
                    let mut arr = BaseArr::<$type>::from_bytes(initial_seq)?;

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
                    let err = BaseArr::<$type>::from_bytes(seq).unwrap_err();
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
                    let result = BaseArr::<$type>::from_bytes(&seq);
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
                    let mut arr = BaseArr::<$type>::from_bytes(b"A").unwrap();
                    arr.set(200, Base::C); // This should panic
                }

                #[test]
                fn test_get_out_of_bounds() {
                    let arr = BaseArr::<$type>::from_bytes(b"ACGT").unwrap();
                    assert_eq!(arr.get(200), None);
                }

                #[test]
                fn test_to_string_impl() -> Result<(), Box<dyn std::error::Error>> {
                    let v = b"ACCTG";
                    let r = BaseArr::<$type>::from_bytes(v)?;
                    assert_eq!(r.to_string(), "ACCTG");

                    let v_long = b"ACCTGACCTGACCTGACCTGACCTG"; // 25 bases
                    let r_long = BaseArr::<$type>::from_bytes(v_long)?;
                    assert_eq!(r_long.to_string(), "ACCTGACCTGACCTGACCTGACCTG");

                    Ok(())
                }

                #[test]
                fn test_get_iter() -> Result<(), Error> {
                    let seq = b"ATCGNATCGN"; // 10 bases
                    let arr = BaseArr::<$type>::from_bytes(seq)?;

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
                    let mut original_bytes =
                        b"ATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGNATCGN".to_vec();
                    assert_eq!(original_bytes.len(), 50);

                    let mut arr = BaseArr::<$type, 10>::from_bytes(&original_bytes)?;

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

                #[test]
                fn test_from_iter_simple() -> Result<(), Error> {
                    let seq = vec![b'A', b'T', b'C', b'G', b'N'];
                    let arr = BaseArr::<$type>::from_iter(seq)?;
                    assert_eq!(arr.to_string(), "ATCGN");
                    Ok(())
                }

                #[test]
                fn test_from_iter_empty() -> Result<(), Error> {
                    let seq: Vec<u8> = vec![];
                    let arr = BaseArr::<$type>::from_iter(seq)?;
                    assert_eq!(arr.to_string(), "");
                    Ok(())
                }

                #[test]
                fn test_from_iter_spans_chunks() -> Result<(), Error> {
                    let seq = "ATCGNATCGNATCGNATCGNATCGN".bytes().collect::<Vec<u8>>(); // 25 bases
                    let arr = BaseArr::<$type>::from_iter(seq)?;
                    assert_eq!(arr.to_string(), "ATCGNATCGNATCGNATCGNATCGN");
                    assert_eq!(arr.get(20), Some(Base::A));
                    assert_eq!(arr.get(21), Some(Base::T));
                    Ok(())
                }

                #[test]
                fn test_from_iter_invalid_char() {
                    let seq = "ATCGZ".bytes().collect::<Vec<u8>>();
                    let result = BaseArr::<$type>::from_iter(seq);
                    assert!(result.is_err());
                    assert!(
                        result
                            .unwrap_err()
                            .to_string()
                            .contains("Invalid base 'Z' at position 4")
                    );
                }

                #[test]
                fn test_from_iter_too_long() {
                    let seq = vec![b'A'; 200];
                    let result = BaseArr::<$type>::from_iter(seq);
                    assert!(result.is_err());
                    assert!(
                        result
                            .unwrap_err()
                            .to_string()
                            .contains("Input iterator is too long")
                    );
                }
            }
        };
    }

    make_test_functions!(u16, u16);
    make_test_functions!(u64, u64);
}
