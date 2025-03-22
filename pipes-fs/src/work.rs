use std::path::PathBuf;

use futures::future::BoxFuture;
use pipes::Work;
use relative_path::RelativePathBuf;

use crate::{Body, Package};

pub struct FsWork {
    root: PathBuf,
}

impl FsWork {
    pub fn new(root: impl Into<PathBuf>) -> FsWork {
        FsWork { root: root.into() }
    }
}

impl<C> Work<C, RelativePathBuf> for FsWork {
    type Output = Package;

    type Future<'a>
        = BoxFuture<'a, Result<Package, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, _ctx: C, path: RelativePathBuf) -> Self::Future<'a> {
        Box::pin(async move {
            let full_path = path.to_logical_path(&self.root);
            let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
            Ok(Package::new(path, mime, Body::Path(full_path)))
        })
    }
}
