use std::fmt::Debug;

#[derive(Clone)]
pub struct DataWithIndex<T> {
    data: T,
    pub idx: usize,
}

impl<T> DataWithIndex<T> {
    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }

    pub fn data(&mut self) -> &T {
        &self.data
    }
}

impl<T> DataWithIndex<T> {
    pub fn new(data: T, idx: usize) -> Self {
        Self { data, idx }
    }
}

impl<T: Debug> Debug for DataWithIndex<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataWithIndex")
            .field("data", &self.data)
            .field("idx", &self.idx)
            .finish()
    }
}
