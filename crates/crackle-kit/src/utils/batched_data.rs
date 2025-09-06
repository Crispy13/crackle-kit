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

    /// Gets a mutable reference to the next available slot and advances the index.
    /// Returns None if the batch is full.
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

    /// Returns true if no more items can be added.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.next_item_idx >= self.inner.len()
    }

    /// Returns the total capacity of the batch.
    pub fn capacity(&self) -> usize {
        self.inner.len()
    }
}
