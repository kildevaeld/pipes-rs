use std::path::PathBuf;

use crate::{
    Body, Package,
    resolver::{FileResolver, Matcher},
};
use futures::{StreamExt, TryStreamExt, pin_mut, stream::BoxStream};
use pipes::Source;

pub struct FsSource {
    root: FileResolver,
}

impl Default for FsSource {
    fn default() -> Self {
        FsSource::new(std::env::current_dir().unwrap())
    }
}

impl FsSource {
    pub fn new(root: PathBuf) -> FsSource {
        FsSource {
            root: FileResolver::new(root),
        }
    }

    pub fn pattern<T: Matcher + 'static>(self, pattern: T) -> Self {
        Self {
            root: self.root.pattern(pattern),
        }
    }
}

impl<C> Source<C> for FsSource {
    type Item = Package;

    type Stream<'a>
        = BoxStream<'a, Result<Self::Item, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(self, _ctx: C) -> Self::Stream<'a> {
        async_stream::try_stream! {
            let root = self.root.root().to_path_buf();

            let stream = self.root.find_relative();
            pin_mut!(stream);

            while let Some(next) = stream.try_next().await.map_err(pipes::Error::new)? {
                let full_path = next.to_logical_path(&root);
                let mime = mime_guess::from_path(&full_path).first_or_octet_stream();

                yield Package::new(next, mime, Body::Path(full_path));

            }
        }
        .boxed()
    }
}
