use mime::Mime;
use relative_path::{RelativePath, RelativePathBuf};
use bycat::Matcher;

use crate::Package;

pub trait WithPath {
    fn path(&self) -> &RelativePath;
}

impl<T> WithPath for Package<T> {
    fn path(&self) -> &RelativePath {
        self.path()
    }
}

impl<'a> WithPath for &'a RelativePath {
    fn path(&self) -> &RelativePath {
        self
    }
}

impl WithPath for RelativePath {
    fn path(&self) -> &RelativePath {
        self
    }
}

impl WithPath for RelativePathBuf {
    fn path(&self) -> &RelativePath {
        self
    }
}

impl<'a> WithPath for &'a RelativePathBuf {
    fn path(&self) -> &RelativePath {
        self
    }
}

#[derive(Debug, Clone)]
pub struct Glob<S>(S);

impl<T, S> Matcher<T> for Glob<S>
where
    S: AsRef<str> + Send + Sync,
    T: WithPath,
{
    fn is_match(&self, path: &T) -> bool {
        fast_glob::glob_match(self.0.as_ref(), path.path().as_str())
    }
}

pub fn match_glob<S>(pattern: S) -> Glob<S> {
    Glob(pattern)
}

#[derive(Debug, Clone)]
pub struct MimeMatcher(mime::Mime);

impl<T> Matcher<Package<T>> for MimeMatcher {
    fn is_match(&self, path: &Package<T>) -> bool {
        path.mime() == &self.0
    }
}

pub fn match_mime(mime: Mime) -> MimeMatcher {
    MimeMatcher(mime)
}
