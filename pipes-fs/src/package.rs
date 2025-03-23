use core::{
    any::{Any, TypeId},
    task::Poll,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use bytes::{BufMut, Bytes, BytesMut};
use either::Either;
use futures::{Future, TryStreamExt, future::BoxFuture, stream::BoxStream};
use pin_project_lite::pin_project;
use relative_path::{RelativePath, RelativePathBuf};
use tokio::io::AsyncWriteExt;

use pipes::{AsyncClone, Error};

pub use mime::{self, Mime};

pub enum Body {
    Bytes(Bytes),
    Path(PathBuf),
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

            while let Some(next) = stream.try_next().await.map_err(Error::new)? {
                buf.put(next);
            }

            *self = Body::Bytes(buf.freeze());
        } else if let Body::Path(path) = self {
            let content = tokio::fs::read(path).await.map_err(Error::new)?;
            *self = Body::Bytes(content.into());
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

trait ToAny {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clone_box(&self) -> Box<dyn ToAny + Send>;
    fn any_box(self: Box<Self>) -> Box<dyn Any>;
}

impl<T> ToAny for T
where
    T: Any + Clone + Send,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clone_box(&self) -> Box<dyn ToAny + Send> {
        Box::new(self.clone())
    }

    fn any_box(self: Box<Self>) -> Box<dyn Any> {
        let this = *self;
        Box::new(this)
    }
}

#[derive(Default)]
pub struct Meta {
    values: HashMap<TypeId, Box<dyn ToAny + Send>>,
}

impl Meta {
    pub fn insert<T: Clone + Send + 'static>(&mut self, value: T) -> Option<T> {
        let old = self.values.insert(TypeId::of::<T>(), Box::new(value));
        old.and_then(|m| m.any_box().downcast().ok().map(|m| *m))
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.values
            .get(&TypeId::of::<T>())
            .and_then(|m| m.as_any().downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.values
            .get_mut(&TypeId::of::<T>())
            .and_then(|m| m.as_any_mut().downcast_mut::<T>())
    }
}

impl Clone for Meta {
    fn clone(&self) -> Self {
        let values = self
            .values
            .iter()
            .map(|(k, v)| (k.clone(), v.clone_box()))
            .collect::<HashMap<_, _>>();

        Meta { values }
    }
}

pub struct Package<B> {
    pub(crate) name: RelativePathBuf,
    pub(crate) mime: Mime,
    pub(crate) content: B,
    pub(crate) meta: Meta,
}

impl<B> Package<B> {
    pub fn new(name: impl Into<RelativePathBuf>, mime: Mime, body: B) -> Package<B> {
        Package {
            name: name.into(),
            mime,
            content: body.into(),
            meta: Default::default(),
        }
    }

    pub fn path(&self) -> &RelativePath {
        &self.name
    }

    pub fn path_mut(&mut self) -> &mut RelativePathBuf {
        &mut self.name
    }

    pub fn set_path(&mut self, name: impl Into<RelativePathBuf>) {
        self.name = name.into();
    }

    pub fn name(&self) -> &str {
        self.name.file_name().unwrap()
    }

    pub fn mime(&self) -> &Mime {
        &self.mime
    }

    pub fn content(&self) -> &B {
        &self.content
    }

    pub fn content_mut(&mut self) -> &mut B {
        &mut self.content
    }

    pub fn take_content(&mut self) -> B
    where
        B: Default,
    {
        core::mem::replace(&mut self.content, B::default())
    }

    pub fn replace_content(&mut self, content: B) -> B {
        core::mem::replace(&mut self.content, content)
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }

    pub fn meta_mut(&mut self) -> &mut Meta {
        &mut self.meta
    }
}

impl Package<Body> {
    pub async fn write_to(&mut self, path: impl AsRef<Path>) -> Result<(), Error> {
        let file_path = self.name.to_logical_path(path);

        match &mut self.content {
            Body::Bytes(bs) => {
                let mut file = tokio::fs::File::create(file_path)
                    .await
                    .map_err(Error::new)?;
                file.write_all(&*bs).await.map_err(Error::new)?;
                file.flush().await.map_err(Error::new)?;
            }
            Body::Stream(stream) => {
                let mut file = tokio::fs::File::create(file_path)
                    .await
                    .map_err(Error::new)?;

                let mut bytes = BytesMut::new();
                while let Some(next) = stream.try_next().await? {
                    file.write_all(&next).await.map_err(Error::new)?;
                    bytes.put(next);
                }

                self.content = Body::Bytes(bytes.freeze());

                file.flush().await.map_err(Error::new)?;
            }
            Body::Path(path) => {
                tokio::fs::copy(path, file_path).await.map_err(Error::new)?;
            }
            Body::Empty => {}
        }

        Ok(())
    }
}

impl<B: AsyncClone + Send> AsyncClone for Package<B>
where
    for<'a> B::Future<'a>: Send,
{
    type Future<'a>
        = BoxFuture<'a, Result<Package<B>, pipes::Error>>
    where
        Self: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a> {
        Box::pin(async move {
            Ok(Package {
                name: self.name.clone(),
                mime: self.mime.clone(),
                content: self.content.async_clone().await?,
                meta: self.meta.clone(),
            })
        })
    }
}

pub trait IntoPackage<B> {
    type Future: Future<Output = Result<Package<B>, Error>>;

    fn into_package(self) -> Self::Future;
}

impl<B> IntoPackage<B> for Package<B> {
    type Future = futures::future::Ready<Result<Package<B>, Error>>;
    fn into_package(self) -> Self::Future {
        futures::future::ready(Ok(self))
    }
}

impl<T1, T2, B> IntoPackage<B> for Either<T1, T2>
where
    T1: IntoPackage<B>,
    T2: IntoPackage<B>,
{
    type Future = EitherIntoPackageFuture<T1, T2, B>;

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
    pub enum EitherIntoPackageFuture<T1, T2, B> where T1: IntoPackage<B>, T2: IntoPackage<B> {
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

impl<T1, T2, B> Future for EitherIntoPackageFuture<T1, T2, B>
where
    T1: IntoPackage<B>,
    T2: IntoPackage<B>,
{
    type Output = Result<Package<B>, Error>;

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
