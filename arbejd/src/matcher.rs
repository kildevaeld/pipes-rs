#[cfg(feature = "alloc")]
use alloc::{boxed::Box, string::String};

pub trait Matcher<T> {
    fn is_match(&self, path: &T) -> bool;
}

#[cfg(feature = "alloc")]
impl<T: AsRef<str>> Matcher<T> for String {
    fn is_match(&self, path: &T) -> bool {
        self.as_str().is_match(path)
    }
}

impl<'a, T: AsRef<str>> Matcher<T> for &'a str {
    fn is_match(&self, path: &T) -> bool {
        path.as_ref() == *self
    }
}

#[cfg(feature = "alloc")]
impl<T> Matcher<T> for Box<dyn Matcher<T>> {
    fn is_match(&self, path: &T) -> bool {
        (**self).is_match(path)
    }
}

impl<T, F> Matcher<T> for F
where
    F: Fn(&T) -> bool,
{
    fn is_match(&self, path: &T) -> bool {
        (self)(path)
    }
}
