use core::task::Poll;

use either::Either;
use futures_core::{TryFuture, ready};
use pin_project_lite::pin_project;

pub trait Work<C, I> {
    type Output;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Output, Self::Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, context: &'a C, req: I) -> Self::Future<'a>;
}

// Either

impl<L, R, I, C> Work<C, I> for Either<L, R>
where
    L: Work<C, I>,
    R: Work<C, I>,
{
    type Output = Either<L::Output, R::Output>;
    type Error = Either<L::Error, R::Error>;

    type Future<'a>
        = EitherWorkFuture<'a, L, R, C, I>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, ctx: &'a C, package: I) -> Self::Future<'a> {
        match self {
            Self::Left(left) => EitherWorkFuture::T1 {
                future: left.call(ctx, package),
            },
            Self::Right(left) => EitherWorkFuture::T2 {
                future: left.call(ctx, package),
            },
        }
    }
}

pin_project! {
    #[project = EitherFutureProj]
    pub enum EitherWorkFuture<'a, T1:'a, T2: 'a, C: 'a, T> where T1: Work<C, T>, T2: Work<C, T> {
        T1 {
            #[pin]
            future: T1::Future<'a>
        },
        T2 {
            #[pin]
            future: T2::Future<'a>
        }
    }
}

impl<'a, T1, T2, C, T> Future for EitherWorkFuture<'a, T1, T2, C, T>
where
    T1: Work<C, T> + 'a,
    T2: Work<C, T> + 'a,
{
    type Output = Result<Either<T1::Output, T2::Output>, Either<T1::Error, T2::Error>>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();

        match this {
            EitherFutureProj::T1 { future } => match ready!(future.try_poll(cx)) {
                Ok(ret) => Poll::Ready(Ok(Either::Left(ret))),
                Err(err) => Poll::Ready(Err(Either::Left(err))),
            },
            EitherFutureProj::T2 { future } => match ready!(future.try_poll(cx)) {
                Ok(ret) => Poll::Ready(Ok(Either::Right(ret))),
                Err(err) => Poll::Ready(Err(Either::Right(err))),
            },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoopWork;

impl<C, R: 'static> Work<C, R> for NoopWork {
    type Output = R;
    type Error = core::convert::Infallible;
    type Future<'a>
        = core::future::Ready<Result<R, Self::Error>>
    where
        C: 'a;
    fn call<'a>(&'a self, _ctx: &'a C, package: R) -> Self::Future<'a> {
        core::future::ready(Ok(package))
    }
}
