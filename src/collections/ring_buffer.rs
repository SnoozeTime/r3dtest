/// Ring buffer that will keep track of the latest states of the game
/// in the server. This data structure is absolutely not thread-safe.
pub struct RingBuffer<T> {
    /// Inner buffer. It is a vector but it should not grow. The size
    /// is reserved at creation.
    inner: Vec<Option<T>>,

    size: usize,
    next: usize,
}

impl<T> RingBuffer<T> {
    pub fn new(size: usize) -> Self {
        let mut inner = Vec::with_capacity(size);
        for _ in 0..size {
            inner.push(None);
        }
        let next = 0;
        Self { inner, size, next }
    }

    /// Push an element at the current head position.
    pub fn push(&mut self, data: T) {
        self.inner[self.next] = Some(data);
        self.next = (self.next + 1) % self.size;
    }

    /// Just get the element at the give index.
    pub fn get(&self, idx: usize) -> Option<&T> {
        self.inner.get(idx).and_then(|opt| opt.as_ref())
    }

    pub fn head(&self) -> Option<&T> {
        self.get(self.head_index())
    }

    pub fn head_index(&self) -> usize {
        if self.next == 0 {
            self.size - 1
        } else {
            self.next - 1
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_circular() {
        let mut circular: RingBuffer<u8> = RingBuffer::new(2);
        assert_eq!(circular.inner.len(), 2);
        assert_eq!(circular.inner.capacity(), 2);

        assert_eq!(None, circular.get(0));
        assert_eq!(None, circular.get(1));

        circular.push(23);
        assert_eq!(Some(&23), circular.get(0));
        assert_eq!(None, circular.get(1));

        circular.push(22);
        assert_eq!(Some(&23), circular.get(0));
        assert_eq!(Some(&22), circular.get(1));

        circular.push(0);
        assert_eq!(Some(&0), circular.get(0));
        assert_eq!(Some(&22), circular.get(1));

        assert_eq!(Some(&0), circular.head());
    }
}
