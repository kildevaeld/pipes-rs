use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use futures::{TryStream, TryStreamExt};
use pipes::{BoxError, Error};

#[async_trait]
pub trait Content {
    async fn bytes(&mut self) -> Result<Bytes, Error>;
}

#[async_trait]
impl Content for Bytes {
    async fn bytes(&mut self) -> Result<Bytes, Error> {
        Ok(self.clone())
    }
}

enum StreamContentState<T> {
    Stream(T),
    Bytes(Bytes),
}

pub struct StreamContent<T> {
    state: StreamContentState<T>,
}

impl<T> StreamContent<T> {
    pub fn new(stream: T) -> StreamContent<T> {
        StreamContent {
            state: StreamContentState::Stream(stream),
        }
    }
}

impl<T> From<Bytes> for StreamContent<T> {
    fn from(value: Bytes) -> Self {
        StreamContent {
            state: StreamContentState::Bytes(value),
        }
    }
}

#[async_trait]
impl<T> Content for StreamContent<T>
where
    T: TryStream<Ok = Bytes> + Send + Unpin,
    T::Error: Into<BoxError>,
{
    async fn bytes(&mut self) -> Result<Bytes, Error> {
        match &mut self.state {
            StreamContentState::Bytes(bs) => Ok(bs.clone()),
            StreamContentState::Stream(stream) => {
                let mut bytes = BytesMut::new();
                while let Some(next) = stream.try_next().await.map_err(Error::new)? {
                    bytes.put(next);
                }

                let bytes = bytes.freeze();

                self.state = StreamContentState::Bytes(bytes.clone());
                Ok(bytes)
            }
        }
    }
}
