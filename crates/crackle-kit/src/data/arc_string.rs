use std::{
    borrow::Borrow,
    hash::Hash,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// A Arc<String> Wrapper to use it for HashMap, Hashset
#[derive(PartialEq, Eq, Debug, Hash, Clone)]
pub struct ArcString {
    inner: Arc<String>,
}

impl From<String> for ArcString {
    fn from(value: String) -> Self {
        Self { inner: Arc::new(value) }
    }
}

impl From<Arc<String>> for ArcString {
    fn from(value: Arc<String>) -> Self {
        Self { inner: value }
    }
}

impl Borrow<str> for ArcString {
    fn borrow(&self) -> &str {
        self.inner.as_str()
    }
}

impl DerefMut for ArcString {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Deref for ArcString {
    type Target = Arc<String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::*;

    #[test]
    fn test_hashmap() {
        // just a compile-test.

        let mut h = HashSet::new();

        let ars = Arc::new(String::new());
        h.insert(ArcString::from(ars));

        h.get("asdf");
    }

    #[test]
    fn test_hashset_with_arc_string() {
        let mut h = HashSet::new();
        let s1 = "hello";
        let s2 = "world";

        // Insert two strings.
        h.insert(ArcString::from(Arc::new(s1.to_string())));
        h.insert(ArcString::from(s2.to_string()));

        // We can now correctly find the entry using a &str.
        assert!(h.contains(s1));
        assert!(h.contains(s2));
        assert!(!h.contains("goodbye"));

        // The `get` method also works correctly now.
        assert_eq!(h.get(s1).unwrap().inner.as_str(), "hello");
    }
}
