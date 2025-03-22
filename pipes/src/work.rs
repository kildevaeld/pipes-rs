use core::task::Poll;
use either::Either;
use futures::{ready, Future, TryFuture};
use pin_project_lite::pin_project;

use crate::{
    and::And,
    error::Error,
    split::{Anyways, Split},
    then::Then,
    wrap::Wrap,
};

pub trait Work<C, T> {
    type Output;
    type Future<'a>: Future<Output = Result<Self::Output, Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, ctx: C, package: T) -> Self::Future<'a>;
}

pub trait WorkExt<C, T>: Work<C, T> {
    fn pipe<W>(self, work: W) -> And<Self, W>
    where
        Self: Sized,
        W: Work<C, Self::Output>,
    {
        And::new(self, work)
    }

    fn then<W>(self, work: W) -> Then<Self, W>
    where
        Self: Sized,
        W: Work<C, Result<Self::Output, Error>>,
    {
        Then::new(self, work)
    }

    fn wrap<F, U>(self, func: F) -> Wrap<Self, F, C>
    where
        Self: Sized,
        F: Fn(C, T, Self) -> U + Clone,
        U: TryFuture,
        U::Error: Into<Error>,
    {
        Wrap::new(self, func)
    }

    fn split<L, R>(self, left: L, right: R) -> Split<Self, L, R>
    where
        Self: Sized,
        Self::Output: Anyways,
        L: Work<C, <Self::Output as Anyways>::Left> + Clone,
        R: Work<C, <Self::Output as Anyways>::Right, Output = L::Output> + Clone,
        C: Clone,
    {
        Split::new(self, left, right)
    }
}

impl<T, R, C> WorkExt<C, R> for T where T: Work<C, R> {}

#[derive(Debug, Clone, Copy)]
pub struct NoopWork;

impl<C, R: 'static> Work<C, R> for NoopWork {
    type Output = R;
    type Future<'a> = futures::future::Ready<Result<R, Error>>;
    fn call<'a>(&'a self, _ctx: C, package: R) -> Self::Future<'a> {
        futures::future::ready(Ok(package))
    }
}

pub fn work_fn<T, C, R, U>(func: T) -> WorkFn<T>
where
    T: Fn(C, R) -> U,
    U: TryFuture,
    U::Error: Into<Error>,
{
    WorkFn(func)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkFn<T>(pub(crate) T);

impl<T, U, C, R> Work<C, R> for WorkFn<T>
where
    T: Fn(C, R) -> U + 'static,
    U: TryFuture,
    U::Error: Into<Error>,
{
    type Output = U::Ok;
    type Future<'a>
        = WorkFnFuture<U>
    where
        Self: 'a;
    fn call<'a>(&'a self, ctx: C, package: R) -> Self::Future<'a> {
        WorkFnFuture {
            future: (self.0)(ctx, package),
        }
    }
}

pin_project! {
  pub struct WorkFnFuture<U> {
    #[pin]
    future: U
  }
}

impl<U> Future for WorkFnFuture<U>
where
    U: TryFuture,
    U::Error: Into<Error>,
{
    type Output = Result<U::Ok, Error>;
    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        match ready!(this.future.try_poll(cx)) {
            Ok(ret) => Poll::Ready(Ok(ret)),
            Err(err) => Poll::Ready(Err(err.into())),
        }
    }
}

impl<T1, T2, C, T> Work<C, T> for Either<T1, T2>
where
    T1: Work<C, T>,
    T2: Work<C, T>,
{
    type Output = Either<T1::Output, T2::Output>;

    type Future<'a>
        = EitherWorkFuture<'a, T1, T2, C, T>
    where
        Self: 'a;

    fn call<'a>(&'a self, ctx: C, package: T) -> Self::Future<'a> {
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
    pub enum EitherWorkFuture<'a, T1:'a, T2: 'a, C, T> where T1: Work<C, T>, T2: Work<C, T> {
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
    type Output = Result<Either<T1::Output, T2::Output>, Error>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        let this = self.project();

        match this {
            EitherFutureProj::T1 { future } => match ready!(future.try_poll(cx)) {
                Ok(ret) => Poll::Ready(Ok(Either::Left(ret))),
                Err(err) => Poll::Ready(Err(err)),
            },
            EitherFutureProj::T2 { future } => match ready!(future.try_poll(cx)) {
                Ok(ret) => Poll::Ready(Ok(Either::Right(ret))),
                Err(err) => Poll::Ready(Err(err)),
            },
        }
    }
}
