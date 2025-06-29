use std::path::PathBuf;

use crate::{Body, resolver::FileResolver};
use bycat::Matcher;
use bycat_error::Error;
use bycat_package::Package;
use bycat_source::Source;
use futures::{StreamExt, TryStreamExt, pin_mut, stream::BoxStream};
use heather::{HBoxStream, HSend};
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

impl<C> Source<C> for FsSource
where
    C: HSend,
{
    type Item = Package<Body>;

    type Error = Error;

    type Stream<'a>
        = HBoxStream<'a, Result<Self::Item, Error>>
    where
        Self: 'a,
        C: 'a;

    fn create_stream<'a>(self, _ctx: &'a C) -> Self::Stream<'a> {
        Box::pin(async_stream::try_stream! {
            let root = self.root.root().to_path_buf();

            let stream = self.root.find();
            pin_mut!(stream);

            while let Some(next) = stream.try_next().await.map_err(Error::new)? {
                let full_path = next.to_logical_path(&root);
                let mime = mime_guess::from_path(&full_path).first_or_octet_stream();

                yield Package::new(next, mime, Body::Path(full_path));

            }
        })
    }
}
