use core::task::{Poll, ready};

use pin_project_lite::pin_project;

use crate::Work;

pub struct Tower<T>(T);

impl<T> Tower<T> {
    pub fn new(service: T) -> Tower<T> {
        Tower(service)
    }
}

impl<T, C, I> Work<C, I> for Tower<T>
where
    T: tower::Service<I> + Clone,
{
    type Output = T::Response;

    type Error = T::Error;

    type Future<'a>
        = TowerFuture<T, I>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _context: &'a C, req: I) -> Self::Future<'a> {
        TowerFuture {
            state: State::Init {
                input: Some(req),
                future: tower::util::ReadyOneshot::new(self.0.clone()),
            },
        }
    }
}

pin_project! {
#[project = StateProj]
enum State<T, I> where T: tower::Service<I> {
  Init {
    input: Option<I>,
    #[pin]
    future: tower::util::ReadyOneshot<T, I>,
  },
  Future {
    #[pin]
    future: T::Future
  }
}

}

pin_project! {
  pub struct TowerFuture< T, I>
where
    T: tower::Service<I>,
{
    #[pin]
    state: State<T, I>
}

}

impl<T, I> Future for TowerFuture<T, I>
where
    T: tower::Service<I>,
{
    type Output = Result<T::Response, T::Error>;

    fn poll(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Self::Output> {
        loop {
            let mut this = self.as_mut().project();
            match this.state.as_mut().project() {
                StateProj::Init { input, future } => {
                    let mut ret = match ready!(future.poll(cx)) {
                        Ok(ret) => ret,
                        Err(err) => return Poll::Ready(Err(err)),
                    };

                    let future = ret.call(input.take().unwrap());

                    this.state.set(State::Future { future });
                }
                StateProj::Future { future } => {
                    return future.poll(cx);
                }
            }
        }
    }
}
