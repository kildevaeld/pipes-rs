use core::{mem::transmute, task::Poll};

use either::Either;
use futures::{ready, stream::TryFlatten, Stream, TryFuture, TryStream, TryStreamExt};
use pin_project_lite::pin_project;

use crate::{and::And, cloned::AsyncCloned, error::Error, then::Then, Pipeline, SourceUnit, Work};

pub trait Source<C> {
    type Item;
    type Stream<'a>: Stream<Item = Result<Self::Item, Error>>
    where
        Self: 'a;
    fn create_stream<'a>(self, ctx: C) -> Self::Stream<'a>;
}

impl<T: 'static, C> Source<C> for alloc::vec::Vec<Result<T, Error>> {
    type Item = T;
    type Stream<'a> = futures::stream::Iter<alloc::vec::IntoIter<Result<T, Error>>>;
    fn create_stream<'a>(self, _ctx: C) -> Self::Stream<'a> {
        futures::stream::iter(self)
    }
}

impl<T: 'static, C> Source<C> for Result<T, Error> {
    type Item = T;

    type Stream<'a> = futures::stream::Once<futures::future::Ready<Result<T, Error>>>;

    fn create_stream<'a>(self, _ctx: C) -> Self::Stream<'a> {
        futures::stream::once(futures::future::ready(self))
    }
}

pub trait SourceExt<C>: Source<C> {
    fn and<S>(self, source: S) -> And<Self, S>
    where
        Self: Sized,
        S: Source<C>,
    {
        And::new(self, source)
    }

    fn filter<W>(self, work: W) -> Filter<Self, W>
    where
        Self: Sized,
        W: Work<C, Self::Item, Output = Option<Self::Item>>,
    {
        Filter::new(self, work)
    }

    fn pipe<W>(self, work: W) -> Pipeline<Self, W, C>
    where
        Self: Sized,
        W: Work<C, Self::Item>,
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

    fn then<W>(self, work: W) -> Then<Self, W>
    where
        Self: Sized,
        W: Work<C, Result<Self::Item, Error>>,
    {
        Then::new(self, work)
    }

    fn unit(self) -> SourceUnit<Self>
    where
        Self: Sized,
    {
        SourceUnit::new(self)
    }
}

impl<T, C> SourceExt<C> for T where T: Source<C> {}

impl<T1, T2, C> Source<C> for Either<T1, T2>
where
    T1: Source<C>,
    T2: Source<C>,
{
    type Item = Either<T1::Item, T2::Item>;

    type Stream<'a>
        = EitherSourceStream<'a, T1, T2, C>
    where
        T1: 'a,
        T2: 'a;

    fn create_stream<'a>(self, ctx: C) -> Self::Stream<'a> {
        match self {
            Self::Left(left) => EitherSourceStream::T1 {
                stream: left.create_stream(ctx),
            },
            Self::Right(left) => EitherSourceStream::T2 {
                stream: left.create_stream(ctx),
            },
        }
    }
}

pin_project! {
    #[project = EitherStreamProj]
    pub enum EitherSourceStream<'a, T1: 'a, T2: 'a, C> where T1: Source<C>, T2: Source<C> {
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

impl<'a, T1, T2, C> Stream for EitherSourceStream<'a, T1, T2, C>
where
    T1: Source<C>,
    T2: Source<C>,
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

#[derive(Debug, Clone, Copy)]
pub struct Filter<T, W> {
    source: T,
    work: W,
}

impl<T, W> Filter<T, W> {
    pub fn new(source: T, work: W) -> Filter<T, W> {
        Filter { source, work }
    }
}

impl<T, W: 'static, C> Source<C> for Filter<T, W>
where
    T: Source<C>,
    W: Work<C, T::Item, Output = Option<T::Item>> + Clone,
    C: Clone,
{
    type Item = T::Item;

    type Stream<'a>
        = FilterStream<'a, T, W, C>
    where
        T: 'a,
        W: 'a;

    fn create_stream<'a>(self, ctx: C) -> Self::Stream<'a> {
        FilterStream {
            stream: self.source.create_stream(ctx.clone()),
            work: self.work,
            future: None,
            ctx,
        }
    }
}

pin_project! {
    #[project(!Unpin)]
    pub struct FilterStream<'a, T: 'a, W: 'static, C> where T: Source<C>, W: Work<C,T::Item, Output = Option<T::Item>> {
        #[pin]
        stream: T::Stream<'a>,
        work: W,
        #[pin]
        future: Option<W::Future<'a>>,
        ctx: C
    }
}

impl<'a, T, W, C> Stream for FilterStream<'a, T, W, C>
where
    W: Work<C, T::Item, Output = Option<T::Item>> + Clone,
    T: Source<C>,
    C: Clone,
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
                this.future.set(Some(unsafe {
                    transmute(this.work.call(this.ctx.clone(), item))
                }));
            } else {
                break None;
            }
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Flatten<S> {
    source: S,
}

impl<S, C> Source<C> for Flatten<S>
where
    S: Source<C>,
    S::Item: TryStream<Error = Error>,
{
    type Item = <S::Item as TryStream>::Ok;

    type Stream<'a>
        = TryFlatten<S::Stream<'a>>
    where
        S: 'a;

    fn create_stream<'a>(self, ctx: C) -> Self::Stream<'a> {
        self.source.create_stream(ctx).try_flatten()
    }
}
