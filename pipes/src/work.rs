use alloc::boxed::Box;
use core::task::Poll;
use either::Either;
use futures::{
    future::{BoxFuture, LocalBoxFuture},
    ready, Future, FutureExt, TryFuture,
};
use pin_project_lite::pin_project;

use crate::{and::And, context::Context, error::Error, then::Then};

#[cfg(feature = "std")]
use super::{IntoPackage, Package};

pub trait Work<T> {
    type Output;
    type Future: Future<Output = Result<Self::Output, Error>>;
    fn call(&self, ctx: Context, package: T) -> Self::Future;
}

pub trait WorkExt<T>: Work<T> {
    fn boxed(self) -> WorkBox<T, Self::Output>
    where
        Self: Sized + Send + 'static,
        Self::Future: Send + 'static,
        T: Send + 'static,
    {
        Box::new(BoxedWorker(self))
    }

    fn boxed_local(self) -> LocalWorkBox<T, Self::Output>
    where
        Self: Sized + Send + 'static,
        Self::Future: Send + 'static,
        T: Send + 'static,
    {
        Box::new(LocalBoxedWorker(self))
    }

    #[cfg(feature = "std")]
    fn into_package(self) -> IntoPackageWork<Self>
    where
        Self: Sized,
        Self::Output: IntoPackage,
    {
        IntoPackageWork { worker: self }
    }

    fn pipe<W>(self, work: W) -> And<Self, W>
    where
        Self: Sized,
        W: Work<Self::Output>,
    {
        And::new(self, work)
    }

    fn then<W>(self, work: W) -> Then<Self, W>
    where
        Self: Sized,
        W: Work<Result<Self::Output, Error>>,
    {
        Then::new(self, work)
    }
}

impl<T, R> WorkExt<R> for T where T: Work<R> {}

#[derive(Debug, Clone, Copy)]
pub struct NoopWork;

impl<R> Work<R> for NoopWork {
    type Output = R;
    type Future = futures::future::Ready<Result<R, Error>>;
    fn call(&self, _ctx: Context, package: R) -> Self::Future {
        futures::future::ready(Ok(package))
    }
}

pub fn work_fn<T, R, U>(func: T) -> WorkFn<T>
where
    T: Fn(Context, R) -> U,
    U: TryFuture,
    U::Error: Into<Error>,
{
    WorkFn(func)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkFn<T>(pub(crate) T);

impl<T, U, R> Work<R> for WorkFn<T>
where
    T: Fn(Context, R) -> U,
    U: TryFuture,
    U::Error: Into<Error>,
{
    type Output = U::Ok;
    type Future = WorkFnFuture<U>;
    fn call(&self, ctx: Context, package: R) -> Self::Future {
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

pub type WorkBox<R, O> =
    Box<dyn Work<R, Output = O, Future = BoxFuture<'static, Result<O, Error>>> + Send>;

pub type LocalWorkBox<R, O> =
    Box<dyn Work<R, Output = O, Future = LocalBoxFuture<'static, Result<O, Error>>>>;

pub struct BoxedWorker<T>(T);

impl<T, R> Work<R> for BoxedWorker<T>
where
    T: Work<R>,
    T::Future: Send + 'static,
    R: Send + 'static,
{
    type Output = T::Output;

    type Future = BoxFuture<'static, Result<T::Output, Error>>;

    fn call(&self, ctx: Context, package: R) -> Self::Future {
        self.0.call(ctx, package).boxed()
    }
}

pub struct LocalBoxedWorker<T>(T);

impl<T, R> Work<R> for LocalBoxedWorker<T>
where
    T: Work<R>,
    T::Future: 'static,
{
    type Output = T::Output;

    type Future = LocalBoxFuture<'static, Result<T::Output, Error>>;

    fn call(&self, ctx: Context, package: R) -> Self::Future {
        self.0.call(ctx, package).boxed_local()
    }
}

#[cfg(feature = "std")]
#[derive(Debug, Clone, Copy)]
pub struct IntoPackageWork<T> {
    worker: T,
}

#[cfg(feature = "std")]
impl<T, R> Work<R> for IntoPackageWork<T>
where
    T: Work<R>,
    T::Output: IntoPackage,
{
    type Output = Package;

    type Future = IntoPackageWorkFuture<T::Future>;

    fn call(&self, ctx: Context, package: R) -> Self::Future {
        IntoPackageWorkFuture::Work {
            future: self.worker.call(ctx, package),
        }
    }
}

#[cfg(feature = "std")]
pin_project! {
    #[project = Proj]
    pub enum IntoPackageWorkFuture<T> where T: TryFuture, T::Ok: IntoPackage {
       Work {
        #[pin]
        future: T
       },
       Convert {
        #[pin]
        future: <T::Ok as IntoPackage>::Future
       },
       Done
    }
}

#[cfg(feature = "std")]
impl<T> Future for IntoPackageWorkFuture<T>
where
    T: TryFuture,
    T::Ok: IntoPackage,
    T::Error: Into<Error>,
{
    type Output = Result<Package, Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> Poll<Self::Output> {
        loop {
            let this = self.as_mut().project();

            match this {
                Proj::Convert { future } => {
                    let ret = ready!(future.poll(cx));
                    self.set(Self::Done);
                    return Poll::Ready(ret);
                }
                Proj::Work { future } => {
                    let ret = ready!(future.try_poll(cx));
                    match ret {
                        Ok(ret) => self.set(Self::Convert {
                            future: ret.into_package(),
                        }),
                        Err(err) => {
                            self.set(Self::Done);
                            return Poll::Ready(Err(err.into()));
                        }
                    }
                }
                Proj::Done => {
                    panic!("poll after done")
                }
            }
        }
    }
}

impl<T1, T2, T> Work<T> for Either<T1, T2>
where
    T1: Work<T>,
    T2: Work<T>,
{
    type Output = Either<T1::Output, T2::Output>;

    type Future = EitherWorkFuture<T1, T2, T>;

    fn call(&self, ctx: Context, package: T) -> Self::Future {
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
    pub enum EitherWorkFuture<T1, T2, T> where T1: Work<T>, T2: Work<T> {
        T1 {
            #[pin]
            future: T1::Future
        },
        T2 {
            #[pin]
            future: T2::Future
        }
    }
}

impl<T1, T2, T> Future for EitherWorkFuture<T1, T2, T>
where
    T1: Work<T>,
    T2: Work<T>,
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
