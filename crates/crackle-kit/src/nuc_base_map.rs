use std::borrow::Borrow;

pub struct NucBaseMap<T> {
    inner: [T; 5],
}

impl<T: Default> Default for NucBaseMap<T> {
    fn default() -> Self {
        Self {
            inner: std::array::from_fn(|_| T::default()),
        }
    }
}

impl<T> NucBaseMap<T> {
    const NUC_BASES: [u8; 5] = [b'A', b'T', b'C', b'G', b'N'];

    const NUC_IDX_ARR: [usize; 256] = Self::make_nuc_idx_arr();

    const fn make_nuc_idx_arr() -> [usize; 256] {
        let mut idx_arr = [u8::MAX as usize; 256];

        idx_arr[Self::NUC_BASES[0] as usize] = 0;
        idx_arr[Self::NUC_BASES[1] as usize] = 1;
        idx_arr[Self::NUC_BASES[2] as usize] = 2;
        idx_arr[Self::NUC_BASES[3] as usize] = 3;
        idx_arr[Self::NUC_BASES[4] as usize] = 4;

        idx_arr[Self::NUC_BASES[0].to_ascii_lowercase() as usize] = 0;
        idx_arr[Self::NUC_BASES[1].to_ascii_lowercase() as usize] = 1;
        idx_arr[Self::NUC_BASES[2].to_ascii_lowercase() as usize] = 2;
        idx_arr[Self::NUC_BASES[3].to_ascii_lowercase() as usize] = 3;
        idx_arr[Self::NUC_BASES[4].to_ascii_lowercase() as usize] = 4;

        idx_arr
    }

    #[inline]
    fn get_nuc_idx(nuc_base: u8) -> usize {
        Self::NUC_IDX_ARR[nuc_base as usize]
    }

    pub fn get(&self, nuc_base: u8) -> Option<&T> {
        let idx = Self::get_nuc_idx(nuc_base);

        if idx < 5 {
            Some(&self.inner[idx])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, nuc_base: u8) -> Option<&mut T> {
        let idx = Self::get_nuc_idx(nuc_base);

        if idx < 5 {
            Some(&mut self.inner[idx])
        } else {
            None
        }
    }

    /// iteration order: A T C G N
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.inner.iter()
    }

    /// iteration order: A T C G N
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.inner.iter_mut()
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    // A simple struct that implements Default but not Copy, to test flexibility.
    #[derive(Debug, PartialEq, Default)]
    struct NonCopyStruct {
        value: u32,
        name: String,
    }

    #[test]
    fn test_default_initialization() {
        // Test with a Copy type (u32)
        let map_u32: NucBaseMap<u32> = NucBaseMap::default();
        for &val in map_u32.inner.iter() {
            assert_eq!(val, 0); // Default for u32 is 0
        }

        // Test with a non-Copy type (NonCopyStruct)
        let map_non_copy: NucBaseMap<NonCopyStruct> = NucBaseMap::default();
        for val in map_non_copy.inner.iter() {
            assert_eq!(val.value, 0);
            assert_eq!(val.name, ""); // Default for String is empty
        }
    }

    #[test]
    fn test_get_and_get_mut_valid_uppercase() {
        let mut map: NucBaseMap<u32> = NucBaseMap::default();

        // Test get_mut
        *map.get_mut(b'A').unwrap() = 10;
        *map.get_mut(b'T').unwrap() = 20;
        *map.get_mut(b'C').unwrap() = 30;
        *map.get_mut(b'G').unwrap() = 40;
        *map.get_mut(b'N').unwrap() = 50;

        // Test get
        assert_eq!(*map.get(b'A').unwrap(), 10);
        assert_eq!(*map.get(b'T').unwrap(), 20);
        assert_eq!(*map.get(b'C').unwrap(), 30);
        assert_eq!(*map.get(b'G').unwrap(), 40);
        assert_eq!(*map.get(b'N').unwrap(), 50);
    }

    #[test]
    fn test_get_and_get_mut_valid_lowercase() {
        let mut map: NucBaseMap<u32> = NucBaseMap::default();

        // Test get_mut with lowercase
        *map.get_mut(b'a').unwrap() = 10;
        *map.get_mut(b't').unwrap() = 20;
        *map.get_mut(b'c').unwrap() = 30;
        *map.get_mut(b'g').unwrap() = 40;
        *map.get_mut(b'n').unwrap() = 50;

        // Test get with lowercase
        assert_eq!(*map.get(b'a').unwrap(), 10);
        assert_eq!(*map.get(b't').unwrap(), 20);
        assert_eq!(*map.get(b'c').unwrap(), 30);
        assert_eq!(*map.get(b'g').unwrap(), 40);
        assert_eq!(*map.get(b'n').unwrap(), 50);

        // Also check cross-case access
        assert_eq!(*map.get(b'A').unwrap(), 10);
        assert_eq!(*map.get(b'T').unwrap(), 20);
    }

    #[test]
    fn test_get_and_get_mut_invalid_nuc_base() {
        let mut map: NucBaseMap<u32> = NucBaseMap::default();

        // Test get with invalid base
        assert_eq!(map.get(b'X'), None);
        assert_eq!(map.get(b'Z'), None);
        assert_eq!(map.get(0), None); // Null byte
        assert_eq!(map.get(255), None); // Max u8 value
        assert_eq!(map.get(b'P'), None);

        // Test get_mut with invalid base
        assert_eq!(map.get_mut(b'X'), None);
        assert_eq!(map.get_mut(b'Z'), None);
        assert_eq!(map.get_mut(0), None);
        assert_eq!(map.get_mut(255), None);
        assert_eq!(map.get_mut(b'P'), None);
    }

    #[test]
    fn test_iter() {
        let mut map: NucBaseMap<u32> = NucBaseMap::default();
        *map.get_mut(b'A').unwrap() = 1;
        *map.get_mut(b'T').unwrap() = 2;
        *map.get_mut(b'C').unwrap() = 3;
        *map.get_mut(b'G').unwrap() = 4;
        *map.get_mut(b'N').unwrap() = 5;

        let mut iter = map.iter();
        // The iteration order is A, T, C, G, N, based on the `inner` array's indices.
        assert_eq!(iter.next(), Some(&1));
        assert_eq!(iter.next(), Some(&2));
        assert_eq!(iter.next(), Some(&3));
        assert_eq!(iter.next(), Some(&4));
        assert_eq!(iter.next(), Some(&5));
        assert_eq!(iter.next(), None); // End of iteration
    }

    #[test]
    fn test_iter_mut() {
        let mut map: NucBaseMap<u32> = NucBaseMap::default();
        *map.get_mut(b'A').unwrap() = 10;
        *map.get_mut(b'T').unwrap() = 20;

        // Iterate and modify
        let mut sum_after_mod = 0;
        for (i, val_ref) in map.iter_mut().enumerate() {
            if i == 0 { // This corresponds to 'A' (index 0)
                *val_ref += 1; // 10 -> 11
            } else if i == 1 { // This corresponds to 'T' (index 1)
                *val_ref -= 5; // 20 -> 15
            }
            sum_after_mod += *val_ref;
        }

        // Verify changes by getting values from the map
        assert_eq!(*map.get(b'A').unwrap(), 11);
        assert_eq!(*map.get(b'T').unwrap(), 15);
        assert_eq!(*map.get(b'C').unwrap(), 0); // Still default
        assert_eq!(*map.get(b'G').unwrap(), 0); // Still default
        assert_eq!(*map.get(b'N').unwrap(), 0); // Still default
        assert_eq!(sum_after_mod, 11 + 15 + 0 + 0 + 0);
    }
    
    // Dedicated test to ensure NUC_IDX_ARR and get_nuc_idx correctly handle all cases
    #[test]
    fn test_nuc_idx_arr_mapping() {
        // Test uppercase bases
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'A'), 0);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'T'), 1);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'C'), 2);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'G'), 3);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'N'), 4);

        // Test lowercase bases (now directly mapped in make_nuc_idx_arr)
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'a'), 0);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b't'), 1);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'c'), 2);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'g'), 3);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'n'), 4);

        // Ensure invalid characters map to u8::MAX as usize (255), which leads to `None` from `get`/`get_mut`
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'X'), u8::MAX as usize);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(b'P'), u8::MAX as usize);
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(0), u8::MAX as usize); // Null byte
        assert_eq!(NucBaseMap::<u8>::get_nuc_idx(255), u8::MAX as usize); // Max u8
    }
}