use core::task::Poll;
use either::Either;
use futures_core::{Future, ready};
use pin_project_lite::pin_project;

use crate::{Work, matcher::Matcher};

#[derive(Debug, Clone, Copy)]
pub struct When<T, W> {
    check: T,
    work: W,
}

impl<T, W, C, R> Work<C, R> for When<T, W>
where
    W: Work<C, R>,
    T: Matcher<R>,
{
    type Output = Either<R, W::Output>;

    type Error = W::Error;

    type Future<'a>
        = CondFuture<'a, W, C, R>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, ctx: &'a C, package: R) -> Self::Future<'a> {
        if self.check.is_match(&package) {
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
  pub enum CondFuture<'a, W: 'a, C: 'a, R> where W: Work<C, R> {
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
    type Output = Result<Either<R, W::Output>, W::Error>;

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

pub fn when<T, W>(check: T, work: W) -> When<T, W> {
    When { check, work }
}
