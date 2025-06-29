use core::{
    error::Error,
    task::{ready, Poll},
};

use alloc::boxed::Box;
use async_stream::try_stream;
use bycat::Work;
use futures::{stream::BoxStream, Future, Stream, TryFutureExt, TryStreamExt};
use pin_project_lite::pin_project;

use crate::Source;

pub trait AsyncClone: Sized {
    type Error;
    type Future<'a>: Future<Output = Result<Self, Self::Error>>
    where
        Self: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a>;
}

// impl<T> AsyncClone for T
// where
//     T: Clone,
// {
//     type Future<'a>
//         = futures::future::Ready<Result<Self, Error>>
//     where
//         T: 'a;

//     fn async_clone<'a>(&'a mut self) -> Self::Future<'a> {
//         futures::future::ready(Ok(self.clone()))
//     }
// }

#[derive(Debug, Clone, Copy)]
pub struct AsyncCloned<S, T1, T2> {
    source: S,
    work1: T1,
    work2: T2,
}

impl<S, T1, T2> AsyncCloned<S, T1, T2> {
    pub fn new(source: S, work1: T1, work2: T2) -> AsyncCloned<S, T1, T2> {
        AsyncCloned {
            source,
            work1,
            work2,
        }
    }
}

impl<S, T1, T2, C> Source<C> for AsyncCloned<S, T1, T2>
where
    S: Source<C> + Send + 'static,
    S::Error: Send,
    for<'a> S::Stream<'a>: Send,
    S::Item: Clone + Send,
    T1::Output: Send,
    T1: Work<C, S::Item> + Clone + Send,
    T1::Error: Into<S::Error> + Send,
    for<'a> T1::Future<'a>: Send,
    T2: Work<C, S::Item, Output = T1::Output> + 'static + Send,
    T2::Error: Into<S::Error> + Send,
    for<'a> T2::Future<'a>: Send,
    C: Send + Sync,
    for<'a> T1: 'a,
{
    type Item = T1::Output;

    type Error = S::Error;

    type Stream<'a>
        = ClonedStream<'a, S, T1, T2, C>
    where
        C: 'a,
        S: 'a,
        T1: 'a,
        T2: 'a;

    fn create_stream<'a>(self, ctx: &'a C) -> Self::Stream<'a> {
        ClonedStream {
            ctx,
            stream: self.source.create_stream(ctx),
            work1: self.work1,
            work2: self.work2,
            state: State::Stream,
        }
    }
}

pin_project! {
    #[project = StateProj]
    enum State<'a, S:'a, T1: 'a, T2: 'a, C: 'a>
    where
        S: Source<C>,
        T1: Work<C, S::Item>,
        T2: Work<C, S::Item>
    {
        Stream,
        Work1 {
            #[pin]
            future: T1::Future<'a>,
            item: Option<S::Item>
        },
        Work2 {
            #[pin]
            future: T2::Future<'a>
        }
    }
}

pin_project! {
    pub struct ClonedStream<'a, S:'a, T1:'a, T2:'a, C:'a>
    where
        S: Source<C>,
        T1: Work<C, S::Item>,
        T2: Work<C, S::Item>
    {
        ctx: &'a C,
        work1: T1,
        work2: T2,
        #[pin]
        state: State<'a, S, T1, T2, C>,
        #[pin]
        stream: S::Stream<'a>
    }
}

impl<'a, S: 'a, T1: 'a, T2: 'a, C: 'a> Stream for ClonedStream<'a, S, T1, T2, C>
where
    S: Source<C>,
    S::Item: Clone,
    T1: Work<C, S::Item>,
    T1::Error: Into<S::Error>,
    T2: Work<C, S::Item, Output = T1::Output>,
    T2::Error: Into<S::Error>,
{
    type Item = Result<T1::Output, S::Error>;

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        loop {
            let mut this = self.as_mut().project();

            let state = this.state.as_mut().project();

            match state {
                StateProj::Stream => {
                    //
                    match ready!(this.stream.poll_next(cx)) {
                        Some(Ok(ret)) => {
                            let future = this.work1.call(this.ctx, ret.clone());
                            this.state.set(State::Work1 {
                                future,
                                item: Some(ret),
                            });
                            continue;
                        }
                        Some(Err(err)) => {
                            return Poll::Ready(Some(Err(err)));
                        }
                        None => return Poll::Ready(None),
                    }
                }
                StateProj::Work1 { future, item } => {
                    let ret = ready!(future.poll(cx));
                    let item = item.take().unwrap();
                    let future = this.work2.call(this.ctx, item);
                    this.state.set(State::Work2 { future });
                    return Poll::Ready(Some(ret.map_err(Into::into)));
                }
                StateProj::Work2 { future } => {
                    let ret = ready!(future.poll(cx));
                    this.state.set(State::Stream);
                    return Poll::Ready(Some(ret.map_err(Into::into)));
                }
            }
        }
    }
}
