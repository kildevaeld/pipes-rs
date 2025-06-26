use futures::{TryFuture, ready};
use pin_project_lite::pin_project;
use pipes::Work;
use std::{marker::PhantomData, task::Poll};

use crate::{IntoPackage, Package};

#[derive(Debug)]
pub struct IntoPackageWork<T, C, B> {
    pub(crate) worker: T,
    pub(crate) ctx: PhantomData<fn() -> (C, B)>,
}

impl<T: Copy, C, B> Copy for IntoPackageWork<T, C, B> {}

impl<T: Clone, C, B> Clone for IntoPackageWork<T, C, B> {
    fn clone(&self) -> Self {
        IntoPackageWork {
            worker: self.worker.clone(),
            ctx: PhantomData,
        }
    }
}

unsafe impl<T: Send, C, B> Send for IntoPackageWork<T, C, B> {}

unsafe impl<T: Sync, C, B> Sync for IntoPackageWork<T, C, B> {}

impl<T, C, B, R> Work<C, R> for IntoPackageWork<T, C, B>
where
    T: Work<C, R>,
    T::Output: IntoPackage<B>,
    C: 'static,
{
    type Output = Package<B>;

    type Future<'a>
        = IntoPackageWorkFuture<T::Future<'a>, B>
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
    pub enum IntoPackageWorkFuture<T, B> where T: TryFuture, T::Ok: IntoPackage<B> {
       Work {
        #[pin]
        future: T
       },
       Convert {
        #[pin]
        future: <T::Ok as IntoPackage<B>>::Future
       },
       Done
    }
}

impl<T, B> Future for IntoPackageWorkFuture<T, B>
where
    T: TryFuture,
    T::Ok: IntoPackage<B>,
    T::Error: Into<pipes::Error>,
{
    type Output = Result<Package<B>, pipes::Error>;

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
