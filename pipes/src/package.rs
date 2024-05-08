use alloc::string::String;
use bytes::Bytes;
use futures::{stream::BoxStream, Future};
use mime::Mime;

use crate::Error;

pub enum Body {
    Bytes(Bytes),
    Stream(BoxStream<'static, Result<Bytes, Error>>),
    Empty,
}

impl Body {
    pub async fn bytes(self) -> Bytes {
        todo!()
    }
}

#[derive(Debug, Default)]
pub struct Meta {}

pub struct Package {
    name: String,
    mime: Mime,
    content: Body,
    meta: Meta,
}

impl Package {
    pub fn new(name: String, mime: Mime, body: impl Into<Body>) -> Package {
        Package {
            name,
            mime,
            content: body.into(),
            meta: Meta::default(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
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
