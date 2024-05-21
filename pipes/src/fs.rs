use std::path::{Path, PathBuf};

use alloc::boxed::Box;
use futures::{future::BoxFuture, TryStreamExt};
use relative_path::RelativePathBuf;

use crate::{Body, Error, IntoPackage, Package, Work};

#[derive(Debug, Clone, Copy)]
pub struct FsWork;

pub struct File {
    path: PathBuf,
    content: tokio::fs::File,
}

impl File {
    pub async fn from_path(path: impl AsRef<Path>) -> Result<File, Error> {
        let file = tokio::fs::File::open(&path).await.map_err(Error::new)?;

        Ok(File {
            path: path.as_ref().to_path_buf(),
            content: file,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl core::ops::Deref for File {
    type Target = tokio::fs::File;
    fn deref(&self) -> &Self::Target {
        &self.content
    }
}

impl<C> Work<C, PathBuf> for FsWork {
    type Output = File;

    type Future = BoxFuture<'static, Result<Self::Output, Error>>;

    fn call(&self, _ctx: C, package: PathBuf) -> Self::Future {
        Box::pin(async move {
            let file = tokio::fs::File::open(&package).await.map_err(Error::new)?;

            Ok(File {
                path: package,
                content: file,
            })
        })
    }
}

impl IntoPackage for File {
    type Future = BoxFuture<'static, Result<Package, Error>>;

    fn into_package(self) -> Self::Future {
        Box::pin(async move {
            let path = RelativePathBuf::from_path(&self.path).map_err(Error::new)?;
            let mime = mime_guess::from_path(&self.path).first_or_octet_stream();
            let stream = tokio_util::io::ReaderStream::new(self.content).map_err(Error::new);
            Ok(Package::new(path, mime, Body::Stream(Box::pin(stream))))
        })
    }
}
