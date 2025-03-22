use std::path::{Path, PathBuf};

use async_walkdir::WalkDir;
use futures::{Stream, TryStreamExt};

pub use async_walkdir::Error as WalkDirError;
use relative_path::RelativePathBuf;

pub trait Matcher: Send + Sync {
    fn is_match(&self, path: &Path) -> bool;
}

impl Matcher for String {
    fn is_match(&self, path: &Path) -> bool {
        self.as_str().is_match(path)
    }
}

impl<'a> Matcher for &'a str {
    fn is_match(&self, path: &Path) -> bool {
        let as_str = match path.file_name() {
            Some(ret) => ret.to_string_lossy().to_string(),
            None => path.display().to_string(),
        };
        fast_glob::glob_match(self, &*as_str)
    }
}

impl Matcher for Box<dyn Matcher> {
    fn is_match(&self, path: &Path) -> bool {
        (**self).is_match(path)
    }
}

impl<F> Matcher for F
where
    F: Fn(&Path) -> bool + Send + Sync,
{
    fn is_match(&self, path: &Path) -> bool {
        (self)(path)
    }
}

pub struct FileResolver {
    patterns: Vec<Box<dyn Matcher>>,
    root: PathBuf,
}

impl FileResolver {
    pub fn new(path: PathBuf) -> FileResolver {
        FileResolver {
            patterns: Default::default(),
            root: path,
        }
    }
}

impl FileResolver {
    pub fn pattern<M: Matcher + 'static>(mut self, pattern: M) -> Self {
        self.patterns.push(Box::new(pattern));
        self
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn find(self) -> impl Stream<Item = Result<PathBuf, WalkDirError>> {
        async_stream::try_stream! {

            let mut stream = WalkDir::new(self.root);

            'main_loop:
            while let Some(next) = stream.try_next().await? {
                if self.patterns.is_empty() {
                    yield next.path();
                } else {


                    for pattern in &self.patterns {
                        if pattern.is_match(&next.path()) {
                            yield next.path();
                            continue 'main_loop;
                        }
                    }
                }


            }

        }
    }

    pub fn find_relative(self) -> impl Stream<Item = Result<RelativePathBuf, WalkDirError>> {
        let root = self.root.clone();
        self.find().try_filter_map(move |m| {
            let root = root.clone();
            async move {
                let path = match pathdiff::diff_paths(m, &root) {
                    Some(path) => path,
                    None => return Ok(None),
                };
                Ok(RelativePathBuf::from_path(path).ok())
            }
        })
    }
}
