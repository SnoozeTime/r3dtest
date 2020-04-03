pub struct OptionArray<T> {
    inner: Vec<Option<T>>,
    free: Vec<usize>,
}

impl<T> OptionArray<T> {
    /// Create a new OptionArray with the given size.
    pub fn new(size: usize) -> Self {
        let mut inner = Vec::with_capacity(size);
        for _ in 0..size {
            inner.push(None);
        }

        let free = (0..size).collect();
        Self { inner, free }
    }

    /// Add an entry to the array. Return maybe an index.
    /// if None, it means that the array is full.
    pub fn add(&mut self, data: T) -> Option<usize> {
        if let Some(free_index) = self.free.pop() {
            self.inner[free_index] = Some(data);
            Some(free_index)
        } else {
            None
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index < self.inner.len() {
            self.free.push(index);
            self.inner[index].take()
        } else {
            None
        }
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.inner.get(index).and_then(|opt| opt.as_ref())
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.inner.get_mut(index).and_then(|opt| opt.as_mut())
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<T> std::ops::Deref for OptionArray<T> {
    type Target = Vec<Option<T>>;
    fn deref(&self) -> &Vec<Option<T>> {
        &self.inner
    }
}
