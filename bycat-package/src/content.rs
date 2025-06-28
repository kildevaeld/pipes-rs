use alloc::boxed::Box;
use async_trait::async_trait;
use bytes::{BufMut, Bytes, BytesMut};
use core::convert::Infallible;
use futures::{TryStream, TryStreamExt};

#[async_trait]
pub trait Content {
    type Error;
    async fn bytes(&mut self) -> Result<Bytes, Self::Error>;
}

#[async_trait]
impl Content for Bytes {
    type Error = Infallible;
    async fn bytes(&mut self) -> Result<Bytes, Self::Error> {
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
{
    type Error = T::Error;
    async fn bytes(&mut self) -> Result<Bytes, Self::Error> {
        match &mut self.state {
            StreamContentState::Bytes(bs) => Ok(bs.clone()),
            StreamContentState::Stream(stream) => {
                let mut bytes = BytesMut::new();
                while let Some(next) = stream.try_next().await? {
                    bytes.put(next);
                }

                let bytes = bytes.freeze();

                self.state = StreamContentState::Bytes(bytes.clone());
                Ok(bytes)
            }
        }
    }
}
