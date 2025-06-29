use bycat::Work;
use bycat_error::{BoxError, Error};
use bycat_package::{IntoPackage, Package};
use futures::future::BoxFuture;
use heather::{HBoxFuture, HSend};
use mime::Mime;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

use crate::Body;

#[derive(Debug, Clone)]
pub struct FsDest {
    path: std::path::PathBuf,
}

impl FsDest {
    pub fn new(path: impl Into<PathBuf>) -> FsDest {
        FsDest { path: path.into() }
    }
}

impl<C, T> Work<C, Package<T>> for FsDest
where
    T: Into<Body> + HSend,
    for<'a> T: 'a,
{
    type Output = Package<Body>;

    type Error = Error;

    type Future<'a>
        = HBoxFuture<'a, Result<Package<Body>, Error>>
    where
        C: 'a;

    fn call<'a>(&'a self, _ctx: &'a C, req: Package<T>) -> Self::Future<'a> {
        let path = self.path.clone();
        Box::pin(async move {
            let mut package = req.map(|body| async move { body.into() }).await;

            if !tokio::fs::try_exists(&path).await.map_err(Error::new)? {
                tokio::fs::create_dir_all(&path).await.map_err(Error::new)?
            }

            let file_path = package.path().to_logical_path(&path);

            package.content_mut().write_to(&file_path).await?;

            Ok(package)
        })
    }
}

pub trait Filter: Send + Sync {
    fn append(&self, pkg: &Package<Body>) -> bool;
}

impl Filter for Mime {
    fn append(&self, pkg: &Package<Body>) -> bool {
        pkg.mime() == self
    }
}

pub struct KravlDestination {
    root: PathBuf,
    append: Vec<Box<dyn Filter>>,
}

impl KravlDestination {
    pub fn new(path: impl Into<PathBuf>) -> KravlDestination {
        KravlDestination {
            root: path.into(),
            append: Default::default(),
        }
    }

    pub fn append_when<T>(mut self, filter: T) -> Self
    where
        T: Filter + 'static,
    {
        self.append.push(Box::new(filter));
        self
    }
}

impl KravlDestination {
    fn append(&self, pkg: &Package<Body>) -> bool {
        for filter in &self.append {
            if filter.append(pkg) {
                return true;
            }
        }
        false
    }
}

impl<C> Work<C, Package<Body>> for KravlDestination {
    type Output = Package<Body>;
    type Error = Error;
    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _ctx: &'a C, mut req: Package<Body>) -> Self::Future<'a> {
        Box::pin(async move {
            let path = req.path().to_logical_path(&self.root);

            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            if self.append(&req) {
                let mut file = tokio::fs::OpenOptions::default()
                    .append(true)
                    .create(true)
                    .open(&path)
                    .await
                    .map_err(Error::new)?;
                let bytes = req.replace_content(Body::Empty).bytes().await?;
                file.write_all(&bytes).await.map_err(Error::new)?;
                file.write_all(b"\n").await.map_err(Error::new)?;
            } else {
                let mut file = tokio::fs::OpenOptions::default()
                    .create(true)
                    .truncate(true)
                    .write(true)
                    .open(&path)
                    .await
                    .map_err(Error::new)?;
                let bytes = req.replace_content(Body::Empty).bytes().await?;
                file.write_all(&bytes).await.map_err(Error::new)?;
            }

            Ok(req)
        })
    }
}
