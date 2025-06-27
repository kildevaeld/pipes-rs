use core::{pin::Pin, task::Poll};

use either::Either;
use futures_core::{Future, ready};
use pin_project_lite::pin_project;

use crate::{Work, util::IntoEither};

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
    S::Output: IntoEither,
    L: Work<C, <S::Output as IntoEither>::Left, Error = S::Error> + Clone,
    R: Work<C, <S::Output as IntoEither>::Right, Output = L::Output, Error = S::Error> + Clone,
    C: Clone,
{
    type Output = L::Output;

    type Error = R::Error;

    type Future<'a>
        = SplitFuture<'a, S, L, R, C, T>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, ctx: &'a C, package: T) -> Self::Future<'a> {
        SplitFuture::Init {
            future: self.splitter.call(ctx, package),
            left: &self.left,
            right: &self.right,
            ctx: Some(ctx),
        }
    }
}

pin_project! {
    #[project = SplitFutureProj]
    pub enum SplitFuture<'a, S: 'a, L: 'a, R: 'a, C, T>
    where
    S: Work<C, T>,
    S::Output: IntoEither,
    L: Work<C, <S::Output as IntoEither>::Left, Error = S::Error>,
    R: Work<C, <S::Output as IntoEither>::Right, Output = L::Output, Error = S::Error>,
    {
        Init {
            #[pin]
            future: S::Future<'a>,
            left:  &'a L,
            right: &'a R,
            ctx: Option<&'a C>
        },
        Next {
            #[pin]
            future: Either<L::Future<'a>, R::Future<'a>>
        }
    }
}

impl<'a, S, L, R, C, T> Future for SplitFuture<'a, S, L, R, C, T>
where
    S: Work<C, T>,
    S::Output: IntoEither,
    L: Work<C, <S::Output as IntoEither>::Left, Error = S::Error>,
    R: Work<C, <S::Output as IntoEither>::Right, Output = L::Output, Error = L::Error>,
{
    type Output = Result<L::Output, L::Error>;

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
                            Either::Left(ret) => Either::Left(left.call(ctx.take().unwrap(), ret)),
                            Either::Right(ret) => {
                                Either::Right(right.call(ctx.take().unwrap(), ret))
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
