use relative_path::RelativePath;

use crate::Package;

pub trait WithPath {
    fn path(&self) -> &RelativePath;
}

impl<T> WithPath for Package<T> {
    fn path(&self) -> &RelativePath {
        self.path()
    }
}

pub trait Matcher<T>: Send + Sync {
    fn is_match(&self, path: &T) -> bool;
}

impl<T: WithPath> Matcher<T> for String {
    fn is_match(&self, path: &T) -> bool {
        self.as_str().is_match(path)
    }
}

impl<'a, T: WithPath> Matcher<T> for &'a str {
    fn is_match(&self, path: &T) -> bool {
        fast_glob::glob_match(self, path.path().as_str())
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
