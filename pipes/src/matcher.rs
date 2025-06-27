use alloc::{boxed::Box, string::String};

pub trait Matcher<T: ?Sized>: Send + Sync {
    fn is_match(&self, path: &T) -> bool;
}

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

impl<T> Matcher<T> for Box<dyn Matcher<T>> {
    fn is_match(&self, path: &T) -> bool {
        (**self).is_match(path)
    }
}

impl<T, F> Matcher<T> for F
where
    F: Fn(&T) -> bool + Send + Sync,
{
    fn is_match(&self, path: &T) -> bool {
        (self)(path)
    }
}
