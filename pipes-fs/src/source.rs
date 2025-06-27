use std::path::PathBuf;

use crate::{Body, resolver::FileResolver};
use futures::{StreamExt, TryStreamExt, pin_mut, stream::BoxStream};
use pipes::{Matcher, Source};
use pipes_package::Package;
use relative_path::RelativePathBuf;

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

    pub fn pattern<T: Matcher<RelativePathBuf> + 'static>(self, pattern: T) -> Self {
        Self {
            root: self.root.pattern(pattern),
        }
    }
}

impl<C> Source<C> for FsSource {
    type Item = Package<Body>;

    type Stream<'a>
        = BoxStream<'a, Result<Self::Item, pipes::Error>>
    where
        Self: 'a;

    fn start<'a>(self, _ctx: C) -> Self::Stream<'a> {
        async_stream::try_stream! {
            let root = self.root.root().to_path_buf();

            let stream = self.root.find();
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
