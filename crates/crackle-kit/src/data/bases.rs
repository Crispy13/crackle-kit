use std::ops::Index;

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

#[derive(Debug, PartialEq, Eq)]
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
                let base_char = match code {
                    A_CODE => 'A',
                    T_CODE => 'T',
                    C_CODE => 'C',
                    G_CODE => 'G',
                    N_CODE => 'N',
                    NULL_CODE => break 'outer, // Found the end of the sequence
                    _ => unreachable!(),    // Should not happen with 3-bit encoding
                };
                write!(f, "{}", base_char)?;
            }
        }
        Ok(())
    }
}


impl BaseArr {
    /// Creates a new BaseArr from a slice of bytes (e.g., b"ACGTN...").
    /// Returns an error if the slice is too long to fit.
    pub fn from_bytes(s: &[u8]) -> Result<Self, Error> {
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

    /// Gets the Base at a given index.
    pub fn get(&self, index: usize) -> Option<Base> {
        let (idx, offset) = (index / n_bases_in_chunk!(), index % n_bases_in_chunk!());

        // Bounds check
        if idx >= self.inner.len() {
            return None;
        }

        let code = (self.inner[idx] >> (offset * 3)) & 0b111; // Mask to get only the 3 bits

        match code {
            A_CODE => Some(Base::A),
            T_CODE => Some(Base::T),
            C_CODE => Some(Base::C),
            G_CODE => Some(Base::G),
            N_CODE => Some(Base::N),
            // NULL_CODE (0) and any other invalid codes will correctly return None.
            _ => None,
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

        // 2. Convert the new Base to its 3-bit code.
        let new_code = match new_base {
            Base::A => A_CODE,
            Base::T => T_CODE,
            Base::C => C_CODE,
            Base::G => G_CODE,
            Base::N => N_CODE,
        };

        // 3. Shift the new code to the correct position and use bitwise OR
        //    to set the new bits.
        self.inner[idx] |= new_code << bit_pos;
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

impl TryFrom<u8> for Base {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let r = match value {
            b'A' => Self::A,
            b'C' => Self::C,
            b'T' => Self::T,
            b'G' => Self::G,
            b'N' => Self::N,
            oth => Err(anyhow!("Invalid base: {}", oth as char))?,
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
}

