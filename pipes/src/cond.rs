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

impl<T, W, R> Work<R> for Cond<T, W>
where
    W: Work<R> + Clone,
    T: Fn(&R) -> bool,
{
    type Output = Either<R, W::Output>;

    type Future = CondFuture<W, R>;

    fn call(&self, ctx: crate::Context, package: R) -> Self::Future {
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
  pub enum CondFuture< W, R> where W: Work<R> {
    Ready {
      ret: Option<R>,
    },
    Work {
      #[pin]
      future: W::Future
    }
  }
}

impl<W, R> Future for CondFuture<W, R>
where
    W: Work<R>,
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
