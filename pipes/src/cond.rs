use core::task::Poll;

use either::Either;
use futures::{ready, Future};
use pin_project_lite::pin_project;

use crate::{Error, Work};

#[derive(Debug, Clone, Copy)]
pub struct Cond<T, W> {
    check: T,
    work: W,
}

impl<T, W, C, R> Work<C, R> for Cond<T, W>
where
    W: Work<C, R> + 'static,
    T: Fn(&R) -> bool + 'static,
{
    type Output = Either<R, W::Output>;

    type Future<'a> = CondFuture<'a, W, C, R>;

    fn call<'a>(&'a self, ctx: C, package: R) -> Self::Future<'a> {
        if (self.check)(&package) {
            CondFuture::Work {
                future: self.work.call(ctx, package),
            }
        } else {
            CondFuture::Ready { ret: Some(package) }
        }
    }
}

pin_project! {

  #[project = CondFutureProj]
  pub enum CondFuture<'a, W: 'static, C, R> where W: Work<C, R> {
    Ready {
      ret: Option<R>,
    },
    Work {
      #[pin]
      future: W::Future<'a>
    }
  }
}

impl<'a, W, C, R> Future for CondFuture<'a, W, C, R>
where
    W: Work<C, R>,
{
    type Output = Result<Either<R, W::Output>, Error>;

    fn poll(
        self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        let this = self.project();
        match this {
            CondFutureProj::Ready { ret } => {
                Poll::Ready(Ok(Either::Left(ret.take().expect("poll after done"))))
            }
            CondFutureProj::Work { future } => match ready!(future.poll(cx)) {
                Ok(ret) => Poll::Ready(Ok(Either::Right(ret))),
                Err(err) => Poll::Ready(Err(err)),
            },
        }
    }
}

pub fn cond<T, W>(check: T, work: W) -> Cond<T, W> {
    Cond { check, work }
}
