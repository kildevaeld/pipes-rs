use futures::{future::BoxFuture, TryFuture};
use pin_project_lite::pin_project;

use crate::Error;

pub trait Func<T> {
    type Output<'a>
    where
        Self: 'a;
    fn call<'a>(&self, args: T) -> Self::Output<'a>;
}

impl<F, U, T> Func<T> for F
where
    F: Fn(T) -> U + 'static,
    U: TryFuture,
{
    type Output<'a> = U;
    fn call<'a>(&self, args: T) -> Self::Output<'a> {
        (self)(args)
    }
}
