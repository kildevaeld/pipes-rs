use bycat_error::{BoxError, Error};
use bycat_package::{Content, async_trait};
use bytes::{BufMut, Bytes, BytesMut};
use futures::{TryStreamExt, stream::BoxStream};
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;

pub enum Body {
    Bytes(Bytes),
    Path(PathBuf),
    Stream(BoxStream<'static, Result<Bytes, BoxError>>),
    Empty,
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
}

impl Body {
    pub async fn bytes(&mut self) -> Result<Bytes, Error> {
        self.load().await?;
        match self {
            Self::Bytes(bs) => Ok(bs.clone()),
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
            let content = tokio::fs::read(&*path)
                .await
                .map_err(|err| Error::new(err).value("path", path.display().to_string()))?;
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

    pub async fn write_to(&mut self, file_path: &Path) -> Result<(), Error> {
        match self {
            Body::Bytes(bs) => {
                let mut file = tokio::fs::File::create(file_path).await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;
                file.write_all(&*bs).await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;
                file.flush().await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;
            }
            Body::Stream(stream) => {
                let mut file = tokio::fs::File::create(file_path).await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;

                let mut bytes = BytesMut::new();
                while let Some(next) = stream.try_next().await.map_err(Error::new)? {
                    file.write_all(&next).await.map_err(|err| {
                        Error::new(err).value("path", file_path.display().to_string())
                    })?;
                    bytes.put(next);
                }

                *self = Body::Bytes(bytes.freeze());

                file.flush().await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;
            }
            Body::Path(path) => {
                tokio::fs::copy(path, file_path).await.map_err(|err| {
                    Error::new(err).value("path", file_path.display().to_string())
                })?;
            }
            Body::Empty => {}
        }

        Ok(())
    }
}

#[async_trait]
impl Content for Body {
    type Error = Error;
    async fn bytes(&mut self) -> Result<Bytes, Error> {
        Body::bytes(self).await
    }
}

impl From<Bytes> for Body {
    fn from(value: Bytes) -> Self {
        Body::Bytes(value)
    }
}
