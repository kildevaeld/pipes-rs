use core::task::Poll;

use futures::{ready, Stream};
use pin_project_lite::pin_project;

use crate::{and::And, dest::Dest, error::Error, SourceUnit, Unit};

pub trait Source {
    type Item;
    type Stream: Stream<Item = Result<Self::Item, Error>>;
    fn call(self) -> Self::Stream;
}

impl<T> Source for alloc::vec::Vec<Result<T, Error>> {
    type Item = T;
    type Stream = futures::stream::Iter<alloc::vec::IntoIter<Result<T, Error>>>;
    fn call(self) -> Self::Stream {
        futures::stream::iter(self)
    }
}

impl<T> Source for Result<T, Error> {
    type Item = T;

    type Stream = futures::stream::Once<futures::future::Ready<Result<T, Error>>>;

    fn call(self) -> Self::Stream {
        futures::stream::once(futures::future::ready(self))
    }
}

pub trait SourceExt: Source {
    fn and<S>(self, source: S) -> And<Self, S>
    where
        Self: Sized,
        S: Source,
    {
        And::new(self, source)
    }

    fn dest<T>(self, dest: T) -> SourceUnit<Self, T>
    where
        Self: Sized,
        T: Dest<Self::Item>,
    {
        SourceUnit::new(self, dest)
    }

    #[cfg(feature = "tokio")]
    fn spawn(self) -> SpawnSource<Self>
    where
        Self: Sized + Send + 'static,
        Self::Item: Send + 'static,
        Self::Stream: Send,
    {
        SpawnSource::new(self)
    }
}

impl<T> SourceExt for T where T: Source {}

#[cfg(feature = "async-channel")]
impl<T> Source for async_channel::Receiver<T> {
    type Item = T;
    type Stream = AsyncChannelStream<T>;

    fn call(self) -> Self::Stream {
        AsyncChannelStream { rx: self }
    }
}

#[cfg(feature = "async-channel")]
pin_project! {
    pub struct AsyncChannelStream<T> {
        #[pin]
        rx: async_channel::Receiver<T>,
    }
}

#[cfg(feature = "async-channel")]
impl<T> Stream for AsyncChannelStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let this = self.project();
        Poll::Ready(ready!(this.rx.poll_next(cx)).map(Ok))
    }
}

#[cfg(feature = "tokio")]
impl<T> Source for tokio::sync::mpsc::Receiver<T> {
    type Item = T;
    type Stream = TokioChannelStream<T>;

    fn call(self) -> Self::Stream {
        TokioChannelStream { rx: self }
    }
}

#[cfg(feature = "tokio")]
pin_project! {
    pub struct TokioChannelStream<T> {
        #[pin]
        rx: tokio::sync::mpsc::Receiver<T>,
    }
}

#[cfg(feature = "tokio")]
impl<T> Stream for TokioChannelStream<T> {
    type Item = Result<T, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(ready!(this.rx.poll_recv(cx)).map(Ok))
    }
}

#[cfg(feature = "tokio")]
pub struct SpawnSource<S>
where
    S: Source,
{
    rx: tokio::sync::mpsc::Receiver<S::Item>,
}

#[cfg(feature = "tokio")]
impl<S> SpawnSource<S>
where
    S: Source + Send + 'static,
    S::Item: Send + 'static,
    S::Stream: Send,
{
    pub fn new(source: S) -> SpawnSource<S> {
        let (sx, rx) = tokio::sync::mpsc::channel(10);
        tokio::spawn(async move {
            source.dest(sx).run().await;
        });

        SpawnSource { rx }
    }
}

#[cfg(feature = "tokio")]
impl<S> Source for SpawnSource<S>
where
    S: Source,
{
    type Item = S::Item;

    type Stream = TokioChannelStream<S::Item>;

    fn call(self) -> Self::Stream {
        TokioChannelStream { rx: self.rx }
    }
}
