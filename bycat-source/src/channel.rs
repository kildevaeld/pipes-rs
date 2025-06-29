use super::Source;
use bycat_error::Error;
use core::task::Poll;
use flume::r#async::RecvStream;
use futures::{ready, Stream};
use pin_project_lite::pin_project;

pub struct Sender<T> {
    sx: flume::Sender<Result<T, Error>>,
}

impl<T: Send + 'static> Sender<T> {
    pub async fn send_async(&self, payload: Result<T, Error>) -> Result<(), Error> {
        self.sx
            .send_async(payload)
            .await
            .map_err(|_| Error::new("Channel closed"))
    }

    pub async fn send(&self, payload: Result<T, Error>) -> Result<(), Error> {
        self.sx
            .send(payload)
            .map_err(|_| Error::new("Channel closed"))
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender {
            sx: self.sx.clone(),
        }
    }
}

pub struct Receiver<T> {
    rx: flume::Receiver<Result<T, Error>>,
}

impl<T> Receiver<T> {
    pub async fn recv(&self) -> Result<T, Error> {
        match self.rx.recv_async().await.map_err(Error::new) {
            Ok(ret) => ret,
            Err(err) => Err(err),
        }
    }
}

pub fn channel<T>(buffer: usize) -> (Sender<T>, Receiver<T>) {
    let (sx, rx) = flume::bounded(buffer);
    (Sender { sx }, Receiver { rx })
}

pub fn unbound_channel<T>() -> (Sender<T>, Receiver<T>) {
    let (sx, rx) = flume::unbounded();
    (Sender { sx }, Receiver { rx })
}

impl<C, T: 'static> Source<C> for Receiver<T> {
    type Item = T;
    type Error = Error;

    type Stream<'a>
        = ReceiverStream<T>
    where
        C: 'a;

    fn create_stream<'a>(self, _ctx: &'a C) -> Self::Stream<'a> {
        ReceiverStream {
            rx: self.rx.into_stream(),
        }
    }
}

pin_project! {
    pub struct ReceiverStream<T: 'static> {
        #[pin]
        rx: RecvStream<'static,Result<T, Error>>
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        Poll::Ready(ready!(this.rx.poll_next(cx)))
    }
}
