use std::{
    ops::{Bound, Deref, RangeBounds},
    sync::Arc,
};

/// Internal storage for SharedBytes - either owned data or external owner.
#[derive(Clone)]
enum SharedBytesInner {
    /// Directly owned byte data (common case, no vtable overhead).
    Owned(Arc<[u8]>),
    /// External owner that provides byte data (for zero-copy integration).
    External(Arc<dyn AsRef<[u8]> + Send + Sync>),
}

impl SharedBytesInner {
    fn as_slice(&self) -> &[u8] {
        match self {
            SharedBytesInner::Owned(data) => data,
            SharedBytesInner::External(owner) => owner.as_ref().as_ref(),
        }
    }
}

/// A reference-counted byte buffer that supports zero-copy slicing.
///
/// Can be backed by either owned `Arc<[u8]>` data or an external owner
/// implementing `AsRef<[u8]>` (e.g., GStreamer's MappedBuffer).
#[derive(Clone)]
pub struct SharedBytes {
    inner: SharedBytesInner,
    start: usize,
    end: usize,
}

impl SharedBytes {
    /// Creates a new `SharedBytes` from an `Arc<[u8]>`.
    pub fn new(data: Arc<[u8]>) -> Self {
        let end = data.len();
        Self {
            inner: SharedBytesInner::Owned(data),
            start: 0,
            end,
        }
    }

    /// Creates a `SharedBytes` from an external owner (zero-copy).
    ///
    /// The owner must implement `AsRef<[u8]>` and will be kept alive
    /// via reference counting. This enables zero-copy integration with
    /// external buffers like GStreamer's MappedBuffer.
    pub fn from_owner<T>(owner: T) -> Self
    where
        T: AsRef<[u8]> + Send + Sync + 'static,
    {
        let len = owner.as_ref().len();
        Self {
            inner: SharedBytesInner::External(Arc::new(owner)),
            start: 0,
            end: len,
        }
    }

    /// Creates a `SharedBytes` from an Arc-wrapped external owner (zero-copy).
    ///
    /// Use this when you already have an `Arc<T>` to avoid double-wrapping.
    pub fn from_arc_owner<T>(owner: Arc<T>) -> Self
    where
        T: AsRef<[u8]> + Send + Sync + 'static,
    {
        let len = owner.as_ref().as_ref().len();
        Self {
            inner: SharedBytesInner::External(owner),
            start: 0,
            end: len,
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
            inner: self.inner.clone(),
            start: self.start + start,
            end: self.start + end,
        }
    }

    /// Returns the underlying `Arc<[u8]>` if this is owned data covering the entire buffer.
    ///
    /// Returns `None` if this is a slice, or if backed by an external owner.
    pub fn into_arc(self) -> Option<Arc<[u8]>> {
        match self.inner {
            SharedBytesInner::Owned(data) if self.start == 0 && self.end == data.len() => {
                Some(data)
            }
            _ => None,
        }
    }
}

impl Deref for SharedBytes {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        &self.inner.as_slice()[self.start..self.end]
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
        let first = bytes.slice(2..6);
        let second = first.slice(1..3);
        assert_eq!(second.len(), 2);
        assert_eq!(second.as_ref(), &[4, 5]);
    }

    #[test]
    fn test_clone_shares_data() {
        let bytes: SharedBytes = vec![1, 2, 3].into();
        let cloned = bytes.clone();
        assert_eq!(bytes.as_ref(), cloned.as_ref());
        assert!(std::ptr::eq(bytes.as_ref(), cloned.as_ref()));
    }

    #[test]
    fn test_slice_shares_data() {
        let bytes: SharedBytes = vec![1, 2, 3, 4, 5].into();
        let slice1 = bytes.slice(0..2);
        let slice2 = bytes.slice(2..5);
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
        assert!(arc.is_none());
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

    // Tests for external owner support

    /// Mock external buffer owner for testing
    struct MockBuffer {
        data: Vec<u8>,
    }

    impl AsRef<[u8]> for MockBuffer {
        fn as_ref(&self) -> &[u8] {
            &self.data
        }
    }

    #[test]
    fn test_from_owner() {
        let buffer = MockBuffer {
            data: vec![10, 20, 30, 40, 50],
        };
        let bytes = SharedBytes::from_owner(buffer);
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes.as_ref(), &[10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_from_arc_owner() {
        let buffer = Arc::new(MockBuffer {
            data: vec![10, 20, 30, 40, 50],
        });
        let bytes = SharedBytes::from_arc_owner(buffer);
        assert_eq!(bytes.len(), 5);
        assert_eq!(bytes.as_ref(), &[10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_external_owner_slice() {
        let buffer = MockBuffer {
            data: vec![1, 2, 3, 4, 5, 6, 7, 8],
        };
        let bytes = SharedBytes::from_owner(buffer);
        let y_plane = bytes.slice(0..4);
        let uv_plane = bytes.slice(4..8);

        assert_eq!(y_plane.as_ref(), &[1, 2, 3, 4]);
        assert_eq!(uv_plane.as_ref(), &[5, 6, 7, 8]);
    }

    #[test]
    fn test_external_owner_clone_shares_data() {
        let buffer = MockBuffer {
            data: vec![1, 2, 3],
        };
        let bytes = SharedBytes::from_owner(buffer);
        let cloned = bytes.clone();

        assert_eq!(bytes.as_ref(), cloned.as_ref());
        assert!(std::ptr::eq(bytes.as_ref(), cloned.as_ref()));
    }

    #[test]
    fn test_external_owner_into_arc_returns_none() {
        let buffer = MockBuffer {
            data: vec![1, 2, 3],
        };
        let bytes = SharedBytes::from_owner(buffer);
        assert!(bytes.into_arc().is_none());
    }

    #[test]
    fn test_external_owner_nested_slice() {
        let buffer = MockBuffer {
            data: vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
        };
        let bytes = SharedBytes::from_owner(buffer);
        let first = bytes.slice(2..8);
        let second = first.slice(1..4);
        assert_eq!(second.as_ref(), &[3, 4, 5]);
    }
}
