pub struct BatchedData<T> {
    inner: Vec<T>,
    next_item_idx: usize,
}

impl<T: Default + Clone> BatchedData<T> {
    /// make a batched data with default().
    pub fn with_default(batch_size: usize) -> BatchedData<T> {
        Self {
            inner: vec![T::default(); batch_size],
            next_item_idx: 0,
        }
    }
}

impl<T: Clone> BatchedData<T> {
    pub fn new(data_init: impl Fn() -> T, batch_size: usize) -> BatchedData<T> {
        Self {
            inner: vec![data_init(); batch_size],
            next_item_idx: 0,
        }
    }
}

impl<T> BatchedData<T> {
    pub fn from_vec(v: Vec<T>) -> BatchedData<T> {
        Self {
            inner: v,
            next_item_idx: 0,
        }
    }

    // Suggested additions
    /// Returns a slice of the items that have been filled.
    pub fn filled(&self) -> &[T] {
        &self.inner[..self.next_item_idx]
    }

    /// Returns a mutable slice of the items that have been filled.
    pub fn filled_mut(&mut self) -> &mut [T] {
        &mut self.inner[..self.next_item_idx]
    }

    /// Gets a mutable reference to the next available slot and **advances the next item index**.
    /// 
    /// Returns `None` if the batch is full.
    pub fn next_mut(&mut self) -> Option<&mut T> {
        if self.is_full() {
            None
        } else {
            // Get the item and advance the index in one atomic step
            let item = &mut self.inner[self.next_item_idx];
            self.next_item_idx += 1;
            Some(item)
        }
    }

    #[inline]
    pub fn increment_idx(&mut self) {
        self.next_item_idx += 1;
    }

    /// modify next element
    /// note that this increment next modificaion offset.
    #[deprecated]
    pub fn modify_next<R>(&mut self, f: impl Fn(&mut T) -> R) -> Option<R> {
        match {
            if self.next_item_idx >= self.inner.len() {
                None
            } else {
                Some(&mut self.inner[self.next_item_idx])
            }
        } {
            Some(e) => {
                let r = f(e);
                self.increment_idx();

                Some(r)
            }
            None => None,
        }
    }

    /// clear batched data with given function.
    /// This map the function to each item of filled data.
    pub fn clear_with(&mut self, clear_f: impl Fn(&mut T)) {
        self.inner
            .iter_mut()
            .take(self.next_item_idx)
            .for_each(clear_f);

        self.next_item_idx = 0;
    }

    /// Set next item index to 0.
    /// Next `next_mut()` call will return the first item.
    /// 
    /// Note that inner data will be untouched. 
    /// 
    /// If you want to do something for the data, use `clear_with()` method.
    pub fn reset_index(&mut self) {
        self.next_item_idx = 0;
    }

    /// Returns true if no more items can be added.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.next_item_idx >= self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.next_item_idx == 0
    }

    /// Returns the total capacity of the batch.
    pub fn capacity(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A simple struct for testing purposes.
    #[derive(Clone, Debug, PartialEq)]
    struct TestItem {
        id: u32,
        name: String,
    }

    impl Default for TestItem {
        fn default() -> Self {
            TestItem {
                id: 0,
                name: "default".to_string(),
            }
        }
    }

    #[test]
    fn test_creation_and_capacity() {
        // Test `with_default`
        let batch_default: BatchedData<TestItem> = BatchedData::with_default(5);
        assert_eq!(batch_default.capacity(), 5);
        assert_eq!(batch_default.filled().len(), 0);
        assert_eq!(batch_default.inner[0], TestItem::default());

        // Test `from_vec`
        let data = vec![1, 2, 3];
        let batch_from_vec = BatchedData::from_vec(data);
        assert_eq!(batch_from_vec.capacity(), 3);
        assert_eq!(batch_from_vec.filled().len(), 0);

        let batch = BatchedData::new(|| 100, 4);

        // Your original code calls the closure once and clones the result.
        // This assertion correctly verifies that behavior.
        assert_eq!(batch.capacity(), 4);
        assert_eq!(batch.inner, vec![100, 100, 100, 100]);
        assert_eq!(batch.filled().len(), 0);
    }

    #[test]
    fn test_filling_with_next_mut_and_is_full() {
        let mut batch: BatchedData<i32> = BatchedData::with_default(3);

        // Fill the first item
        if let Some(item) = batch.next_mut() {
            *item = 100;
        }
        assert!(!batch.is_full());
        assert_eq!(batch.filled(), &[100]);

        // Fill the second item
        if let Some(item) = batch.next_mut() {
            *item = 200;
        }
        assert!(!batch.is_full());
        assert_eq!(batch.filled(), &[100, 200]);

        // Fill the third and final item
        if let Some(item) = batch.next_mut() {
            *item = 300;
        }
        assert!(batch.is_full());
        assert_eq!(batch.filled(), &[100, 200, 300]);

        // Try to get another item, should be None
        assert!(batch.next_mut().is_none());
    }

    #[test]
    fn test_filled_and_filled_mut() {
        let mut batch = BatchedData::from_vec(vec![0, 0, 0, 0]);

        // Fill part of the batch
        batch.next_mut().map(|v| *v = 10);
        batch.next_mut().map(|v| *v = 20);

        // Check the immutable slice
        assert_eq!(batch.filled(), &[10, 20]);

        // Get a mutable slice and modify it
        let filled_slice = batch.filled_mut();
        assert_eq!(filled_slice.len(), 2);
        filled_slice[0] = 11;

        // Verify the change
        assert_eq!(batch.filled(), &[11, 20]);
    }

    #[test]
    fn test_clear_with() {
        let mut batch = BatchedData::from_vec(vec![
            TestItem {
                id: 1,
                name: "A".to_string(),
            },
            TestItem {
                id: 2,
                name: "B".to_string(),
            },
            TestItem {
                id: 3,
                name: "C".to_string(),
            },
        ]);

        // Fill the batch completely
        batch.next_item_idx = 3;
        assert_eq!(batch.filled().len(), 3);
        assert!(batch.is_full());

        // Clear the batch, resetting the name of each item
        batch.clear_with(|item| {
            item.name = "cleared".to_string();
        });

        // Check that the index is reset and it's no longer full
        assert_eq!(batch.filled().len(), 0);
        assert!(!batch.is_full());

        // Check that the underlying data was modified by the closure
        assert_eq!(batch.inner[0].name, "cleared");
        assert_eq!(batch.inner[1].name, "cleared");
        assert_eq!(batch.inner[2].name, "cleared");
        assert_eq!(batch.inner[0].id, 1); // ID should be unchanged
    }

    #[test]
    fn test_deprecated_modify_next() {
        let mut batch: BatchedData<i32> = BatchedData::from_vec(vec![0; 2]);

        // First modification
        let res1 = batch.modify_next(|val| {
            *val = 50;
            "first".to_string()
        });
        assert_eq!(res1, Some("first".to_string()));
        assert_eq!(batch.filled(), &[50]);

        // Second modification
        let res2 = batch.modify_next(|val| {
            *val = 60;
            "second".to_string()
        });
        assert_eq!(res2, Some("second".to_string()));
        assert_eq!(batch.filled(), &[50, 60]);

        // Batch is now full, should return None
        let res3 = batch.modify_next(|_| "third".to_string());
        assert_eq!(res3, None);
    }
}
