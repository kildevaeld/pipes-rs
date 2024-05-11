use core::{marker::PhantomData, pin::Pin, task::Poll};
use std::sync::Mutex;

use alloc::{collections::VecDeque, sync::Arc};
use either::Either;
use futures::{ready, Future, Stream, TryFuture, TryStream};
use pin_project_lite::pin_project;

use crate::{Error, Source, Work};

pub trait Anyways {
    type Left;
    type Right;

    fn into_either(self) -> Either<Self::Left, Self::Right>;
}

impl<L, R> Anyways for Either<L, R> {
    type Left = L;
    type Right = R;
    fn into_either(self) -> Either<L, R> {
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Split<S, L, R> {
    splitter: S,
    left: L,
    right: R,
}

impl<S, L, R> Split<S, L, R> {
    pub fn new(splitter: S, left: L, right: R) -> Split<S, L, R> {
        Split {
            splitter,
            left,
            right,
        }
    }
}

impl<S, L, R, C, T> Work<C, T> for Split<S, L, R>
where
    S: Work<C, T>,
    S::Output: Anyways,
    L: Work<C, <S::Output as Anyways>::Left> + Clone,
    R: Work<C, <S::Output as Anyways>::Right, Output = L::Output> + Clone,
    C: Clone,
{
    type Output = L::Output;

    type Future = SplitFuture<S, L, R, C, T>;

    fn call(&self, ctx: C, package: T) -> Self::Future {
        SplitFuture::Init {
            future: self.splitter.call(ctx.clone(), package),
            left: self.left.clone(),
            right: self.right.clone(),
            ctx: Some(ctx),
        }
    }
}

pin_project! {
    #[project = SplitFutureProj]
    pub enum SplitFuture<S, L, R, C, T>
    where
    S: Work<C, T>,
    S::Output: Anyways,
    L: Work<C, <S::Output as Anyways>::Left>,
    R: Work<C, <S::Output as Anyways>::Right, Output = L::Output>,
    {
        Init {
            #[pin]
            future: S::Future,
            left: L,
            right: R,
            ctx: Option<C>
        },
        Next {
            #[pin]
            future: futures::future::Either<L::Future, R::Future>
        }
    }
}

impl<S, L, R, C, T> Future for SplitFuture<S, L, R, C, T>
where
    S: Work<C, T>,
    S::Output: Anyways,
    L: Work<C, <S::Output as Anyways>::Left>,
    R: Work<C, <S::Output as Anyways>::Right, Output = L::Output>,
{
    type Output = Result<L::Output, Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match this {
                SplitFutureProj::Init {
                    future,
                    left,
                    right,
                    ctx,
                } => match ready!(future.poll(cx)) {
                    Ok(ret) => {
                        let future = match ret.into_either() {
                            Either::Left(ret) => {
                                futures::future::Either::Left(left.call(ctx.take().unwrap(), ret))
                            }
                            Either::Right(ret) => {
                                futures::future::Either::Right(right.call(ctx.take().unwrap(), ret))
                            }
                        };

                        self.set(SplitFuture::Next { future });
                    }
                    Err(err) => return Poll::Ready(Err(err)),
                },
                SplitFutureProj::Next { future } => return future.poll(cx),
            }
        }
    }
}
