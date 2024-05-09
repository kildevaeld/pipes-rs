use core::{future::ready, task::Poll};

use alloc::sync::Arc;
use either::Either;
use futures::{ready, stream::TryFlatten, Stream, StreamExt, TryFuture, TryStream, TryStreamExt};
use pin_project_lite::pin_project;

use crate::{
    and::And, cloned::AsyncCloned, dest::Dest, error::Error, Context, Pipeline, SourceUnit, Unit,
    Work,
};

pub trait Source {
    type Item;
    type Stream<'a>: Stream<Item = Result<Self::Item, Error>>
    where
        Self: 'a;
    fn call<'a>(self) -> Self::Stream<'a>;
}

impl<T: 'static> Source for alloc::vec::Vec<Result<T, Error>> {
    type Item = T;
    type Stream<'a> = futures::stream::Iter<alloc::vec::IntoIter<Result<T, Error>>>;
    fn call<'a>(self) -> Self::Stream<'a> {
        futures::stream::iter(self)
    }
}

impl<T: 'static> Source for Result<T, Error> {
    type Item = T;

    type Stream<'a> = futures::stream::Once<futures::future::Ready<Result<T, Error>>>;

    fn call<'a>(self) -> Self::Stream<'a> {
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

    fn filter<W>(self, work: W) -> Filter<Self, W>
    where
        Self: Sized,
        W: Work<Self::Item, Output = Option<Self::Item>>,
    {
        Filter::new(self, work)
    }

    fn pipe<W>(self, work: W) -> Pipeline<Self, W>
    where
        Self: Sized,
        W: Work<Self::Item>,
    {
        Pipeline::new_with(self, work)
    }

    fn flatten(self) -> Flatten<Self>
    where
        Self: Sized,
        Self::Item: TryStream<Error = Error>,
    {
        Flatten { source: self }
    }

    fn cloned<T1, T2>(self, work1: T1, work2: T2) -> AsyncCloned<Self, T1, T2>
    where
        Self: Sized,
    {
        AsyncCloned::new(self, work1, work2)
    }

    #[cfg(feature = "tokio")]
    fn spawn(self) -> SpawnSource<Self>
    where
        Self: Sized + Send + 'static,
        Self::Item: Send + 'static,
        for<'a> Self::Stream<'a>: Send,
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
impl<T: 'static> Source for tokio::sync::mpsc::Receiver<T> {
    type Item = T;
    type Stream<'a> = TokioChannelStream<T>;

    fn call<'a>(self) -> Self::Stream<'a> {
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
    for<'a> S::Stream<'a>: Send,
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
impl<S: 'static> Source for SpawnSource<S>
where
    S: Source,
{
    type Item = S::Item;

    type Stream<'a> = TokioChannelStream<S::Item>;

    fn call<'a>(self) -> Self::Stream<'a> {
        TokioChannelStream { rx: self.rx }
    }
}

impl<T1, T2> Source for Either<T1, T2>
where
    T1: Source,
    T2: Source,
{
    type Item = Either<T1::Item, T2::Item>;

    type Stream<'a> = EitherSourceStream<'a, T1, T2> where T1: 'a, T2: 'a;

    fn call<'a>(self) -> Self::Stream<'a> {
        match self {
            Self::Left(left) => EitherSourceStream::T1 {
                stream: left.call(),
            },
            Self::Right(left) => EitherSourceStream::T2 {
                stream: left.call(),
            },
        }
    }
}

pin_project! {
    #[project = EitherStreamProj]
    pub enum EitherSourceStream<'a, T1: 'a, T2: 'a> where T1: Source, T2: Source {
        T1 {
            #[pin]
            stream: T1::Stream<'a>
        },
        T2 {
            #[pin]
            stream: T2::Stream<'a>
        }
    }
}

impl<'a, T1, T2> Stream for EitherSourceStream<'a, T1, T2>
where
    T1: Source,
    T2: Source,
{
    type Item = Result<Either<T1::Item, T2::Item>, Error>;

    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();

        match this {
            EitherStreamProj::T1 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Left(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(err))),
                None => Poll::Ready(None),
            },
            EitherStreamProj::T2 { stream } => match ready!(stream.try_poll_next(cx)) {
                Some(Ok(ret)) => Poll::Ready(Some(Ok(Either::Right(ret)))),
                Some(Err(err)) => Poll::Ready(Some(Err(err))),
                None => Poll::Ready(None),
            },
        }
    }
}

pub struct Filter<T, W> {
    source: T,
    work: W,
}

impl<T, W> Filter<T, W> {
    pub fn new(source: T, work: W) -> Filter<T, W> {
        Filter { source, work }
    }
}

impl<T, W: 'static> Source for Filter<T, W>
where
    T: Source,
    W: Work<T::Item, Output = Option<T::Item>>,
{
    type Item = T::Item;

    type Stream<'a> = FilterStream<'a, T, W> where T:'a, W: 'a;

    fn call<'a>(self) -> Self::Stream<'a> {
        FilterStream {
            stream: self.source.call(),
            work: self.work,
            future: None,
        }
    }
}

pin_project! {
    pub struct FilterStream<'a, T: 'a, W> where T: Source, W: Work<T::Item, Output = Option<T::Item>> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future>,
    }
}

impl<'a, T, W> Stream for FilterStream<'a, T, W>
where
    W: Work<T::Item, Output = Option<T::Item>>,
    T: Source,
{
    type Item = Result<T::Item, Error>;
    fn poll_next(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.future.as_mut().as_pin_mut() {
                let item = ready!(fut.try_poll(cx));
                this.future.set(None);
                match item {
                    Ok(Some(ret)) => break Some(Ok(ret)),
                    Err(err) => break Some(Err(err)),
                    _ => {}
                }
            } else if let Some(item) = ready!(this.stream.as_mut().try_poll_next(cx)?) {
                this.future.set(Some(this.work.call(Context {}, item)));
            } else {
                break None;
            }
        })
    }
}

pub struct Flatten<S> {
    source: S,
}

impl<S> Source for Flatten<S>
where
    S: Source,
    S::Item: TryStream<Error = Error>,
{
    type Item = <S::Item as TryStream>::Ok;

    type Stream<'a> = TryFlatten<S::Stream<'a>> where S: 'a;

    fn call<'a>(self) -> Self::Stream<'a> {
        self.source.call().try_flatten()
    }
}

// pin_project! {

//     pub struct FlattenStream<T> where T: Source {
//         #[pin]
//         stream: TryFlatten<T::Stream>
//     }
// }

// impl<T> Stream for FlattenStream<T>
// where
//     T: Source,
//     T::Item: TryStream<Error = Error,
// {
//     type Item = Result<<T::Item as TryStream>::Ok, Error>;

//     fn poll_next(
//         self: core::pin::Pin<&mut Self>,
//         cx: &mut core::task::Context<'_>,
//     ) -> Poll<Option<Self::Item>> {
//         let this = self.project();

//         match ready!(this.stream.try_poll_next(cx)) {
//             Some(Ok(ret)) => Poll::Ready(Some(Ok(ret))),
//             Some(Err(err)) => Poll::Ready(Some(Err(err.into()))),
//             None => Poll::Ready(None),
//         }
//     }
// }
