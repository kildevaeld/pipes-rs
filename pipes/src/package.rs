use core::task::Poll;
use std::path::{Path, PathBuf};

use alloc::boxed::Box;
use bytes::{BufMut, Bytes, BytesMut};
use either::Either;
use futures::{future::BoxFuture, stream::BoxStream, Future, TryStreamExt};
use mime::Mime;
use pin_project_lite::pin_project;
use relative_path::{RelativePath, RelativePathBuf};
use tokio::io::AsyncWriteExt;

use crate::{cloned::AsyncClone, Error};

pub enum Body {
    Bytes(Bytes),
    // Path(PathBuf,)
    Stream(BoxStream<'static, Result<Bytes, Error>>),
    Empty,
}

impl Body {
    pub async fn bytes(mut self) -> Result<Bytes, Error> {
        self.load().await?;
        match self {
            Self::Bytes(bs) => Ok(bs),
            _ => Ok(Bytes::new()),
        }
    }

    pub async fn load(&mut self) -> Result<(), Error> {
        if let Body::Stream(stream) = self {
            let mut buf = BytesMut::new();

            while let Some(next) = stream.try_next().await.unwrap() {
                buf.put(next);
            }

            *self = Body::Bytes(buf.freeze());
        }

        Ok(())
    }

    pub async fn clone(&mut self) -> Result<Body, Error> {
        self.load().await?;

        match self {
            Self::Bytes(bs) => Ok(Body::Bytes(bs.clone())),
            Self::Empty => Ok(Body::Empty),
            _ => panic!("loaded"),
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Meta {}

pub struct Package {
    name: RelativePathBuf,
    mime: Mime,
    content: Body,
    meta: Meta,
}

impl Package {
    pub fn new(name: impl Into<RelativePathBuf>, mime: Mime, body: impl Into<Body>) -> Package {
        Package {
            name: name.into(),
            mime,
            content: body.into(),
            meta: Meta::default(),
        }
    }

    pub fn path(&self) -> &RelativePath {
        &self.name
    }

    pub fn name(&self) -> &str {
        self.name.file_name().unwrap()
    }

    pub fn mime(&self) -> &Mime {
        &self.mime
    }

    pub fn content(&self) -> &Body {
        &self.content
    }

    pub fn set_content(&mut self, content: impl Into<Body>) {
        self.content = content.into();
    }

    pub fn take_content(&mut self) -> Body {
        core::mem::replace(&mut self.content, Body::Empty)
    }

    pub async fn clone(&mut self) -> Result<Package, Error> {
        Ok(Package {
            name: self.name.clone(),
            mime: self.mime.clone(),
            content: self.content.clone().await?,
            meta: self.meta.clone(),
        })
    }

    pub async fn write_to(self, path: impl AsRef<Path>) -> Result<(), Error> {
        let mut file = tokio::fs::File::create(self.name.to_logical_path(path))
            .await
            .map_err(Error::new)?;

        match self.content {
            Body::Bytes(bs) => {
                file.write_all(&*bs).await.map_err(Error::new)?;
            }
            Body::Stream(mut stream) => {
                while let Some(next) = stream.try_next().await? {
                    file.write_all(&next).await.map_err(Error::new)?;
                }
            }
            Body::Empty => {}
        }

        file.flush().await.map_err(Error::new)?;

        Ok(())
    }
}

pub trait IntoPackage {
    type Future: Future<Output = Result<Package, Error>>;

    fn into_package(self) -> Self::Future;
}

impl IntoPackage for Package {
    type Future = futures::future::Ready<Result<Package, Error>>;
    fn into_package(self) -> Self::Future {
        futures::future::ready(Ok(self))
    }
}

impl<T1, T2> IntoPackage for Either<T1, T2>
where
    T1: IntoPackage,
    T2: IntoPackage,
{
    type Future = EitherIntoPackageFuture<T1, T2>;

    fn into_package(self) -> Self::Future {
        match self {
            Self::Left(left) => EitherIntoPackageFuture::T1 {
                future: left.into_package(),
            },
            Self::Right(left) => EitherIntoPackageFuture::T2 {
                future: left.into_package(),
            },
        }
    }
}

pin_project! {
    #[project = EitherFutureProj]
    pub enum EitherIntoPackageFuture<T1, T2> where T1: IntoPackage, T2: IntoPackage {
        T1 {
            #[pin]
            future: T1::Future
        },
        T2 {
            #[pin]
            future: T2::Future
        }
    }
}

impl<T1, T2> Future for EitherIntoPackageFuture<T1, T2>
where
    T1: IntoPackage,
    T2: IntoPackage,
{
    type Output = Result<Package, Error>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();

        match this {
            EitherFutureProj::T1 { future } => future.poll(cx),
            EitherFutureProj::T2 { future } => future.poll(cx),
        }
    }
}

impl AsyncClone for Package {
    type Future<'a> = BoxFuture<'a, Result<Package, Error>>;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a> {
        Box::pin(async move { self.clone().await })
    }
}
