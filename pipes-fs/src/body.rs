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

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
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
