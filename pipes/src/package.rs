use bytes::{BufMut, Bytes, BytesMut};
use futures::{stream::BoxStream, Future, TryStreamExt};
use mime::Mime;
use relative_path::RelativePathBuf;

use crate::Error;

pub enum Body {
    Bytes(Bytes),
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
}

#[derive(Debug, Default)]
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

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn mime(&self) -> &Mime {
        &self.mime
    }

    pub fn content(&self) -> &Body {
        &self.content
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
