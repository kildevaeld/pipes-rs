use heather::HSendSync;

use crate::extensions::Extensions;

/// A trait representing a readable container that allows retrieving immutable references
/// to stored values of a specific type.
///
/// This trait is implemented for types that implement the `Extensible` trait, which provides
/// access to an internal `Extensions` storage.
///
pub trait ReadableContainer {
    /// Retrieves an immutable reference to a value of type `T` stored in the container.
    ///
    /// Returns `None` if no value of type `T` is found.
    ///
    /// # Returns
    /// An `Option` containing a reference to the value of type `T`, or `None` if not found.
    fn get<T: HSendSync + 'static>(&self) -> Option<&T>;
}

/// A trait representing a mutable container that allows retrieving, removing, and registering
/// values of specific types.
///
/// This trait is implemented for types that implement the `ExtensibleMut` trait, which provides
/// mutable access to an internal `Extensions` storage.
///
pub trait Container {
    /// Retrieves a mutable reference to a value of type `T` stored in the container.
    ///
    /// Returns `None` if no value of type `T` is found.
    ///
    /// # Returns
    /// An `Option` containing a mutable reference to the value of type `T`, or `None` if not found.
    fn get_mut<T: HSendSync + 'static>(&mut self) -> Option<&mut T>;

    /// Removes a value of type `T` from the container.
    ///
    /// Returns `None` if no value of type `T` is found.
    ///
    /// # Returns
    /// An `Option` containing the removed value of type `T`, or `None` if not found.
    fn remove<T: HSendSync>(&mut self) -> Option<T>
    where
        T: 'static;

    /// Registers a value of type `T` in the container.
    ///
    /// If a value of the same type already exists, it will be replaced, and the old value
    /// will be returned.
    ///
    /// # Returns
    /// An `Option` containing the old value of type `T` if it was replaced, or `None` if no
    /// value of the same type existed.
    fn register<T: HSendSync + 'static>(&mut self, value: T) -> Option<T>;
}

/// A trait representing a type that provides access to an internal `Extensions` storage.
///
/// Types implementing this trait allow immutable access to their `Extensions` storage,
/// which can be used to retrieve values of specific types.
pub trait Extensible {
    /// Returns an immutable reference to the internal `Extensions` storage.
    ///
    /// # Returns
    /// A reference to the `Extensions` storage.
    fn extensions(&self) -> &Extensions;
}

/// A trait representing a type that provides mutable access to an internal `Extensions` storage.
///
/// This trait extends the `Extensible` trait, adding methods for mutable access to the
/// `Extensions` storage, which can be used to retrieve, remove, or register values of
/// specific types.
pub trait ExtensibleMut: Extensible {
    /// Returns a mutable reference to the internal `Extensions` storage.
    ///
    /// # Returns
    /// A mutable reference to the `Extensions` storage.
    fn extensions_mut(&mut self) -> &mut Extensions;
}

impl<V> ReadableContainer for V
where
    V: Extensible,
{
    fn get<T: HSendSync + 'static>(&self) -> Option<&T> {
        self.extensions().get()
    }
}

impl<V> Container for V
where
    V: ExtensibleMut,
{
    fn get_mut<T: HSendSync + 'static>(&mut self) -> Option<&mut T> {
        self.extensions_mut().get_mut()
    }

    fn remove<T>(&mut self) -> Option<T>
    where
        T: HSendSync + 'static,
    {
        self.extensions_mut().remove()
    }

    fn register<T: HSendSync + 'static>(&mut self, value: T) -> Option<T> {
        self.extensions_mut().insert(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{
        string::{String, ToString},
        sync::Arc,
    };

    struct TestType(i32);

    #[derive(Default)]
    struct TestContainer {
        extensions: Extensions,
    }

    impl Extensible for TestContainer {
        fn extensions(&self) -> &Extensions {
            &self.extensions
        }
    }

    impl ExtensibleMut for TestContainer {
        fn extensions_mut(&mut self) -> &mut Extensions {
            &mut self.extensions
        }
    }

    #[test]
    fn test_readable_container_get() {
        let mut container = TestContainer::default();
        container.register(TestType(42));

        let value = container.get::<TestType>();
        assert!(value.is_some());
        assert_eq!(value.unwrap().0, 42);
    }

    #[test]
    fn test_container_get_mut() {
        let mut container = TestContainer::default();
        container.register(TestType(42));

        if let Some(value) = container.get_mut::<TestType>() {
            value.0 = 100;
        }

        let value = container.get::<TestType>();
        assert!(value.is_some());
        assert_eq!(value.unwrap().0, 100);
    }

    #[test]
    fn test_container_remove() {
        let mut container = TestContainer::default();
        container.register(TestType(42));

        let removed = container.remove::<TestType>();
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().0, 42);

        let value = container.get::<TestType>();
        assert!(value.is_none());
    }

    #[test]
    fn test_container_register_replace() {
        let mut container = TestContainer::default();
        container.register(TestType(42));

        let old_value = container.register(TestType(100));
        assert!(old_value.is_some());
        assert_eq!(old_value.unwrap().0, 42);

        let value = container.get::<TestType>();
        assert!(value.is_some());
        assert_eq!(value.unwrap().0, 100);
    }

    #[test]
    fn test_register_and_remove_multiple_types() {
        let mut container = TestContainer::default();
        container.register(TestType(42));
        container.register(Arc::new("Hello".to_string()));

        let value = container.get::<TestType>();
        assert!(value.is_some());
        assert_eq!(value.unwrap().0, 42);

        let string_value = container.get::<Arc<String>>();
        assert!(string_value.is_some());
        assert_eq!(**string_value.unwrap(), "Hello");

        let removed_string = container.remove::<Arc<String>>();
        assert!(removed_string.is_some());
        assert_eq!(*removed_string.unwrap(), "Hello");

        let removed_test_type = container.remove::<TestType>();
        assert!(removed_test_type.is_some());
        assert_eq!(removed_test_type.unwrap().0, 42);
    }
}
