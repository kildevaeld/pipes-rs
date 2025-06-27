use flume::r#async::RecvStream;
use futures::{Stream, ready};
use pin_project_lite::pin_project;
use pipes::Source;
use std::task::Poll;

pub struct Sender<T> {
    sx: flume::Sender<Result<T, pipes::Error>>,
}

impl<T: Send + 'static> Sender<T> {
    pub async fn send_async(&self, payload: Result<T, pipes::Error>) -> Result<(), pipes::Error> {
        self.sx
            .send_async(payload)
            .await
            .map_err(|_| pipes::Error::new("Channel closed"))
    }

    pub async fn send(&self, payload: Result<T, pipes::Error>) -> Result<(), pipes::Error> {
        self.sx
            .send(payload)
            .map_err(|_| pipes::Error::new("Channel closed"))
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
    rx: flume::Receiver<Result<T, pipes::Error>>,
}

impl<T> Receiver<T> {
    pub async fn recv(&self) -> Result<T, pipes::Error> {
        match self.rx.recv_async().await.map_err(pipes::Error::new) {
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

    type Stream<'a> = ReceiverStream<T>;

    fn start<'a>(self, _ctx: C) -> Self::Stream<'a> {
        ReceiverStream {
            rx: self.rx.into_stream(),
        }
    }
}

pin_project! {
    pub struct ReceiverStream<T: 'static> {
        #[pin]
        rx: RecvStream<'static,Result<T, pipes::Error>>
    }
}

impl<T> Stream for ReceiverStream<T> {
    type Item = Result<T, pipes::Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        Poll::Ready(ready!(this.rx.poll_next(cx)))
    }
}
