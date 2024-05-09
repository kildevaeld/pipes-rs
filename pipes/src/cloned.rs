use core::task::Poll;

use alloc::boxed::Box;
use async_stream::try_stream;
use futures::{ready, stream::BoxStream, Future, Stream, TryStreamExt};
use pin_project_lite::pin_project;

use crate::{Context, Error, Source, Work};

pub trait AsyncClone: Sized {
    type Future<'a>: Future<Output = Result<Self, Error>>
    where
        Self: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a>;
}

impl<T> AsyncClone for T
where
    T: Clone,
{
    type Future<'a> = futures::future::Ready<Result<Self, Error>> where T: 'a;

    fn async_clone<'a>(&'a mut self) -> Self::Future<'a> {
        futures::future::ready(Ok(self.clone()))
    }
}

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
    S: Source<C> + 'static + Send,
    for<'a> S::Stream<'a>: Send,
    S::Item: AsyncClone + Send,
    for<'a> <S::Item as AsyncClone>::Future<'a>: Send,
    T1::Output: Send,
    T1: Work<C, S::Item> + 'static + Clone + Send,
    T1::Future: Send,
    T2: Work<C, S::Item, Output = T1::Output> + 'static + Clone + Send,
    T2::Future: Send,
    for<'a> C: Send + 'a,
    C: Clone,
{
    type Item = T1::Output;

    type Stream<'a> = BoxStream<'a, Result<T1::Output, Error>>;

    fn call<'a>(self, ctx: C) -> Self::Stream<'a> {
        Box::pin(try_stream! {
            let stream = self.source.call(ctx.clone());
            futures::pin_mut!(stream);

            while let Some(mut item) = stream.try_next().await? {
                let clone = item.async_clone().await?;

                yield self.work1.call(ctx.clone(), clone).await?;
                yield self.work2.call(ctx.clone(), item).await?;

            }
        })
    }
}

// pin_project! {

//   pub struct AsyncClonedStream<'a, S: 'a, T1, T2>
//   where
//     S: Source,
//     S::Item: AsyncClone,
//     T1: Work<S::Item>,
//     T2: Work<S::Item, Output = T1::Output>,
//   {
//     #[pin]
//     stream: S::Stream<'a>,
//     work1: T1,
//     work2: T2,
//     #[pin]
//     future: Option<AsyncClonedState<'a, S, T1, T2>>
//   }
// }

// pin_project! {
//     #[project = AsyncCloneProj]
//   pub enum AsyncClonedState<'a, S: 'a, T1, T2>
//   where
//     S: Source,
//     S::Item: AsyncClone,
//     T1: Work<S::Item>,
//     T2: Work<S::Item, Output = T1::Output>,
//   {
//     Clone {
//     #[pin]
//       future: <S::Item as AsyncClone>::Future<'a>,
//       value: S::Item,
//     },
//     Work {
//         #[pin]
//       future: futures::future::Join<T1::Future, T2::Future>
//     }
//   }
// }

// impl<'a, S, T1, T2> Future for AsyncClonedState<'a, S, T1, T2>
// where
//     S: Source,
//     S::Item: AsyncClone,
//     T1: Work<S::Item>,
//     T2: Work<S::Item, Output = T1::Output>,
// {
//     type Output = Result<T1::Output, Error>;

//     fn poll(
//         self: core::pin::Pin<&mut Self>,
//         cx: &mut core::task::Context<'_>,
//     ) -> core::task::Poll<Self::Output> {
//         loop {
//             let this = self.as_mut().project();

//             match this {
//                 AsyncCloneProj::Clone { future } => {
//                     match ready!(future.poll(cx)) {
//                         Ok(ret) => {
//                             self.set(AsyncClonedState::Work { future: () })
//                         }
//                     }
//                 },
//                 AsyncCloneProj::Work { future } => future.poll(cx),
//             }
//         }
//     }
// }

// impl<'a, S, T1, T2> Stream for AsyncClonedStream<'a, S, T1, T2>
// where
//     S: Source,
//     S::Item: AsyncClone,
//     T1: Work<S::Item>,
//     T2: Work<S::Item, Output = T1::Output>,
// {
//     type Item = Result<T1::Output, Error>;

//     fn poll_next(
//         mut self: core::pin::Pin<&mut Self>,
//         cx: &mut core::task::Context<'_>,
//     ) -> core::task::Poll<Option<Self::Item>> {
//         loop {
//             let mut this = self.as_mut().project();

//             if let Some(future) = this.future.as_mut().as_pin_mut() {
//                 match future.project() {
//                     AsyncCloneProj::Clone { future } => match ready!(future.poll(cx)) {
//                         Ok(ret) => this.future.set(Some(AsyncClonedState::Work {
//                             future: futures::future::join(
//                                 this.work1.call(Context {}, package),
//                                 future2,
//                             ),
//                         })),
//                         Err(err) => {
//                             this.future.set(None);
//                             return Poll::Ready(Some(Err(err)));
//                         }
//                     },
//                     AsyncCloneProj::Work { future } => {}
//                 };
//             }
//         }
//     }
// }
