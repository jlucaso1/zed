use std::{
    ops::{Bound, Deref, RangeBounds},
    sync::Arc,
};

/// A reference-counted byte buffer that supports zero-copy slicing.
#[derive(Clone)]
pub struct SharedBytes {
    data: Arc<[u8]>,
    start: usize,
    end: usize,
}

impl SharedBytes {
    /// Creates a new `SharedBytes` from an `Arc<[u8]>`.
    pub fn new(data: Arc<[u8]>) -> Self {
        let end = data.len();
        Self {
            data,
            start: 0,
            end,
        }
    }

    /// Returns the length of this byte slice.
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// Returns `true` if this byte slice is empty.
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    /// Creates a new `SharedBytes` that references a subslice of this buffer (O(1), no copy).
    pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
        let start = match range.start_bound() {
            Bound::Included(&n) => n,
            Bound::Excluded(&n) => n + 1,
            Bound::Unbounded => 0,
        };
        let end = match range.end_bound() {
            Bound::Included(&n) => n + 1,
            Bound::Excluded(&n) => n,
            Bound::Unbounded => self.len(),
        };

        debug_assert!(start <= end, "slice start must be <= end");
        debug_assert!(end <= self.len(), "slice end out of bounds");

        Self {
            data: Arc::clone(&self.data),
            start: self.start + start,
            end: self.start + end,
        }
    }

    /// Returns the underlying Arc if this covers the entire buffer, otherwise `None`.
    pub fn into_arc(self) -> Option<Arc<[u8]>> {
        if self.start == 0 && self.end == self.data.len() {
            Some(self.data)
        } else {
            None
        }
    }
}

impl Deref for SharedBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.data[self.start..self.end]
    }
}

impl AsRef<[u8]> for SharedBytes {
    fn as_ref(&self) -> &[u8] {
        self
    }
}

impl From<Vec<u8>> for SharedBytes {
    fn from(vec: Vec<u8>) -> Self {
        Self::new(vec.into())
    }
}

impl From<Arc<[u8]>> for SharedBytes {
    fn from(data: Arc<[u8]>) -> Self {
        Self::new(data)
    }
}

impl From<&[u8]> for SharedBytes {
    fn from(slice: &[u8]) -> Self {
        Vec::from(slice).into()
    }
}

impl From<Box<[u8]>> for SharedBytes {
    fn from(boxed: Box<[u8]>) -> Self {
        Self::new(boxed.into())
    }
}

impl std::fmt::Debug for SharedBytes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedBytes")
            .field("len", &self.len())
            .field("start", &self.start)
            .field("end", &self.end)
            .finish()
    }
}

impl PartialEq for SharedBytes {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref() == other.as_ref()
    }
}

impl Eq for SharedBytes {}

impl std::hash::Hash for SharedBytes {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_ref().hash(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_vec() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes.as_ref(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_slice_full_range() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let sliced = bytes.slice(..);
        assert_eq!(sliced.len(), 5);
        assert_eq!(sliced.as_ref(), &[1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_slice_start() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let sliced = bytes.slice(2..);
        assert_eq!(sliced.len(), 3);
        assert_eq!(sliced.as_ref(), &[3, 4, 5]);
    }

    #[test]
    fn test_slice_end() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let sliced = bytes.slice(..3);
        assert_eq!(sliced.len(), 3);
        assert_eq!(sliced.as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn test_slice_middle() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let sliced = bytes.slice(1..4);
        assert_eq!(sliced.len(), 3);
        assert_eq!(sliced.as_ref(), &[2, 3, 4]);
    }

    #[test]
    fn test_nested_slice() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5, 6, 7, 8].into();
        let first = bytes.slice(2..6); // [3, 4, 5, 6]
        let second = first.slice(1..3); // [4, 5]
        assert_eq!(second.len(), 2);
        assert_eq!(second.as_ref(), &[4, 5]);
    }

    #[test]
    fn test_clone_shares_data() {
        let bytes: SharedBytes = vec![1, 2, 3].into();
        let cloned = bytes.clone();

        // Both should point to the same underlying data
        assert_eq!(bytes.as_ref(), cloned.as_ref());
        assert_eq!(Arc::strong_count(&bytes.data), 2);
    }

    #[test]
    fn test_slice_shares_data() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let slice1 = bytes.slice(0..2);
        let slice2 = bytes.slice(2..5);

        // All three should share the same underlying Arc
        assert_eq!(Arc::strong_count(&bytes.data), 3);
        assert_eq!(slice1.as_ref(), &[1, 2]);
        assert_eq!(slice2.as_ref(), &[3, 4, 5]);
    }

    #[test]
    fn test_into_arc_full() {
        let bytes: SharedBytes = vec![1, 2, 3].into();
        let arc = bytes.into_arc();
        assert!(arc.is_some());
        assert_eq!(arc.unwrap().as_ref(), &[1, 2, 3]);
    }

    #[test]
    fn test_into_arc_slice() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let sliced = bytes.slice(1..4);
        let arc = sliced.into_arc();
        assert!(arc.is_none()); // Slice doesn't cover full buffer
    }

    #[test]
    fn test_empty() {
        let bytes: SharedBytes = vec![].into();
        assert!(bytes.is_empty());
        assert_eq!(bytes.len(), 0);
    }

    #[test]
    fn test_equality() {
        let a: SharedBytes = vec![1, 2, 3].into();
        let b: SharedBytes = vec![1, 2, 3].into();
        let c: SharedBytes = vec![1, 2, 4].into();

        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
