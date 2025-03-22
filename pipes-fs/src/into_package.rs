use futures::{TryFuture, ready};
use pin_project_lite::pin_project;
use pipes::Work;
use std::{marker::PhantomData, task::Poll};

use crate::{IntoPackage, Package};

#[derive(Debug)]
pub struct IntoPackageWork<T, C> {
    pub(crate) worker: T,
    pub(crate) ctx: PhantomData<C>,
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

unsafe impl<T: Send, C> Send for IntoPackageWork<T, C> {}

unsafe impl<T: Sync, C> Sync for IntoPackageWork<T, C> {}

impl<T, C, R> Work<C, R> for IntoPackageWork<T, C>
where
    T: Work<C, R>,
    T::Output: IntoPackage,
    C: 'static,
{
    type Output = Package;

    type Future<'a>
        = IntoPackageWorkFuture<T::Future<'a>>
    where
        Self: 'a;

    fn call<'a>(&'a self, ctx: C, package: R) -> Self::Future<'a> {
        IntoPackageWorkFuture::Work {
            future: self.worker.call(ctx, package),
        }
    }
}

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

impl<T> Future for IntoPackageWorkFuture<T>
where
    T: TryFuture,
    T::Ok: IntoPackage,
    T::Error: Into<pipes::Error>,
{
    type Output = Result<Package, pipes::Error>;

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
