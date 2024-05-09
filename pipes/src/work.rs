use alloc::boxed::Box;
use core::{marker::PhantomData, task::Poll};
use either::Either;
use futures::{
    future::{BoxFuture, LocalBoxFuture},
    ready, Future, FutureExt, TryFuture,
};
use pin_project_lite::pin_project;

use crate::{and::And, context::Context, error::Error, then::Then, wrap::Wrap};

#[cfg(feature = "std")]
use super::{IntoPackage, Package};

pub trait Work<C, T> {
    type Output;
    type Future: Future<Output = Result<Self::Output, Error>>;
    fn call(&self, ctx: C, package: T) -> Self::Future;
}

pub trait WorkExt<C, T>: Work<C, T> {
    fn boxed(self) -> WorkBox<C, T, Self::Output>
    where
        Self: Sized + Send + 'static,
        Self::Future: Send + 'static,
        T: Send + 'static,
    {
        Box::new(BoxedWorker(self))
    }

    fn boxed_local(self) -> LocalWorkBox<C, T, Self::Output>
    where
        Self: Sized + Send + 'static,
        Self::Future: Send + 'static,
        T: Send + 'static,
    {
        Box::new(LocalBoxedWorker(self))
    }

    #[cfg(feature = "std")]
    fn into_package(self) -> IntoPackageWork<Self, C>
    where
        Self: Sized,
        Self::Output: IntoPackage,
    {
        IntoPackageWork {
            worker: self,
            ctx: PhantomData,
        }
    }

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
}

impl<T, R, C> WorkExt<C, R> for T where T: Work<C, R> {}

#[derive(Debug, Clone, Copy)]
pub struct NoopWork;

impl<C, R> Work<C, R> for NoopWork {
    type Output = R;
    type Future = futures::future::Ready<Result<R, Error>>;
    fn call(&self, _ctx: C, package: R) -> Self::Future {
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
    T: Fn(C, R) -> U,
    U: TryFuture,
    U::Error: Into<Error>,
{
    type Output = U::Ok;
    type Future = WorkFnFuture<U>;
    fn call(&self, ctx: C, package: R) -> Self::Future {
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

pub type WorkBox<C, R, O> =
    Box<dyn Work<C, R, Output = O, Future = BoxFuture<'static, Result<O, Error>>> + Send>;

pub type LocalWorkBox<C, R, O> =
    Box<dyn Work<C, R, Output = O, Future = LocalBoxFuture<'static, Result<O, Error>>>>;

pub struct BoxedWorker<T>(T);

impl<T, C, R> Work<C, R> for BoxedWorker<T>
where
    T: Work<C, R>,
    T::Future: Send + 'static,
    R: Send + 'static,
{
    type Output = T::Output;

    type Future = BoxFuture<'static, Result<T::Output, Error>>;

    fn call(&self, ctx: C, package: R) -> Self::Future {
        self.0.call(ctx, package).boxed()
    }
}

pub struct LocalBoxedWorker<T>(T);

impl<T, C, R> Work<C, R> for LocalBoxedWorker<T>
where
    T: Work<C, R>,
    T::Future: 'static,
{
    type Output = T::Output;

    type Future = LocalBoxFuture<'static, Result<T::Output, Error>>;

    fn call(&self, ctx: C, package: R) -> Self::Future {
        self.0.call(ctx, package).boxed_local()
    }
}

#[cfg(feature = "std")]
#[derive(Debug)]
pub struct IntoPackageWork<T, C> {
    worker: T,
    ctx: PhantomData<C>,
}

impl<T: Copy, C> Copy for IntoPackageWork<T, C> {}

impl<T: Clone, C> Clone for IntoPackageWork<T, C> {
    fn clone(&self) -> Self {
        IntoPackageWork {
            worker: self.worker.clone(),
            ctx: PhantomData,
        }
    }
}

#[cfg(feature = "std")]
impl<T, C, R> Work<C, R> for IntoPackageWork<T, C>
where
    T: Work<C, R>,
    T::Output: IntoPackage,
{
    type Output = Package;

    type Future = IntoPackageWorkFuture<T::Future>;

    fn call(&self, ctx: C, package: R) -> Self::Future {
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

impl<T1, T2, C, T> Work<C, T> for Either<T1, T2>
where
    T1: Work<C, T>,
    T2: Work<C, T>,
{
    type Output = Either<T1::Output, T2::Output>;

    type Future = EitherWorkFuture<T1, T2, C, T>;

    fn call(&self, ctx: C, package: T) -> Self::Future {
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
    pub enum EitherWorkFuture<T1, T2, C, T> where T1: Work<C, T>, T2: Work<C, T> {
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

impl<T1, T2, C, T> Future for EitherWorkFuture<T1, T2, C, T>
where
    T1: Work<C, T>,
    T2: Work<C, T>,
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
