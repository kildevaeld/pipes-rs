use core::any::TypeId;

use alloc::{boxed::Box, collections::btree_map::BTreeMap};
use heather::{HBoxAny, HSendSync};

/// A type-safe container for storing and retrieving values of different types.
///
/// The `Extensions` struct allows you to store values of any type that implements
/// the `HSendSync` trait and retrieve them later using their type. It uses a
/// `BTreeMap` internally to map type IDs to the stored values.
#[derive(Debug, Default)]
pub struct Extensions {
    inner: BTreeMap<TypeId, HBoxAny<'static>>,
}

impl Extensions {
    /// Inserts a value into the container.
    ///
    /// # Returns
    /// If a value of the same type already exists in the container, it is replaced, and the old value is returned.
    ///
    /// # Example
    /// ```
    /// let mut extensions = wilbur_container::Extensions::default();
    /// extensions.insert(42u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&42));
    /// ```
    pub fn insert<T: HSendSync + 'static>(&mut self, value: T) -> Option<T> {
        let tyid = TypeId::of::<T>();
        self.inner
            .insert(tyid, Box::new(value) as HBoxAny<'static>)
            .and_then(|m| m.downcast().ok())
            .map(|m| *m)
    }

    /// Retrieves a reference to a value of the specified type from the container.
    ///
    /// # Returns
    /// An `Option` containing a reference to the value if it exists, or `None` if no value of the specified type is found.
    ///
    /// # Example
    /// ```
    /// let mut extensions = wilbur_container::Extensions::default();
    /// extensions.insert(42u32);
    /// assert_eq!(extensions.get::<u32>(), Some(&42));
    /// assert_eq!(extensions.get::<i32>(), None);
    /// ```
    pub fn get<T: HSendSync + 'static>(&self) -> Option<&T> {
        self.inner
            .get(&TypeId::of::<T>())
            .and_then(|m| m.downcast_ref())
    }

    /// Retrieves a mutable reference to a value of the specified type from the container.
    ///
    /// # Returns
    /// An `Option` containing a mutable reference to the value if it exists, or `None` if no value of the specified type is found.
    ///
    /// # Example
    /// ```
    /// let mut extensions = wilbur_container::Extensions::default();
    /// extensions.insert(42u32);
    /// if let Some(value) = extensions.get_mut::<u32>() {
    ///     *value = 100;
    /// }
    /// assert_eq!(extensions.get::<u32>(), Some(&100));
    /// ```
    pub fn get_mut<T: HSendSync + 'static>(&mut self) -> Option<&mut T> {
        self.inner
            .get_mut(&TypeId::of::<T>())
            .and_then(|m| m.downcast_mut())
    }

    /// Removes a value of the specified type from the container.
    ///
    /// # Returns
    /// An `Option` containing the removed value if it exists, or `None` if no value of the specified type is found.
    ///
    /// # Example
    /// ```
    /// let mut extensions = wilbur_container::Extensions::default();
    /// extensions.insert(42u32);
    /// assert_eq!(extensions.remove::<u32>(), Some(42));
    /// assert!(extensions.get::<u32>().is_none());
    /// ```
    pub fn remove<T: HSendSync + 'static>(&mut self) -> Option<T> {
        self.inner
            .remove(&TypeId::of::<T>())
            .and_then(|m| m.downcast().ok())
            .map(|m| *m)
    }

    /// Returns the number of elements in the container.
    ///
    /// # Returns
    /// The number of elements currently stored in the container.
    ///
    /// # Example
    /// ```
    /// let mut extensions = wilbur_container::Extensions::default();
    /// extensions.insert(42u32);
    /// extensions.insert("hello");
    /// assert_eq!(extensions.len(), 2);
    /// ```
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Checks if the container is empty.
    ///
    /// # Returns
    /// `true` if the container is empty, `false` otherwise.
    ///
    /// # Example
    /// ```
    /// let extensions = wilbur_container::Extensions::default();
    /// assert!(extensions.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn contains<T: HSendSync + 'static>(&self) -> bool {
        self.inner.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_get() {
        let mut extensions = Extensions::default();
        extensions.insert(42u32);
        assert_eq!(extensions.get::<u32>(), Some(&42));
    }

    #[test]
    fn test_insert_overwrite() {
        let mut extensions = Extensions::default();
        extensions.insert(42u32);
        let old_value = extensions.insert(100u32);
        assert_eq!(old_value, Some(42));
        assert_eq!(extensions.get::<u32>(), Some(&100));
    }

    #[test]
    fn test_get_nonexistent() {
        let extensions = Extensions::default();
        assert_eq!(extensions.get::<u32>(), None);
    }

    #[test]
    fn test_get_mut() {
        let mut extensions = Extensions::default();
        extensions.insert(42u32);
        if let Some(value) = extensions.get_mut::<u32>() {
            *value = 100;
        }
        assert_eq!(extensions.get::<u32>(), Some(&100));
    }

    #[test]
    fn test_remove() {
        let mut extensions = Extensions::default();
        extensions.insert(42u32);
        let removed_value = extensions.remove::<u32>();
        assert_eq!(removed_value, Some(42));
        assert!(extensions.get::<u32>().is_none());
    }

    #[test]
    fn test_len() {
        let mut extensions = Extensions::default();
        assert_eq!(extensions.len(), 0);
        extensions.insert(42u32);
        extensions.insert("hello");
        assert_eq!(extensions.len(), 2);
    }

    #[test]
    fn test_is_empty() {
        let mut extensions = Extensions::default();
        assert!(extensions.is_empty());
        extensions.insert(42u32);
        assert!(!extensions.is_empty());
    }

    #[test]
    fn test_insert_different_types() {
        let mut extensions = Extensions::default();
        extensions.insert(42u32);
        extensions.insert("hello");
        assert_eq!(extensions.get::<u32>(), Some(&42));
        assert_eq!(extensions.get::<&str>(), Some(&"hello"));
    }
}
