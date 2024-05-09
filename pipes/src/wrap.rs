use core::marker::PhantomData;

use alloc::boxed::Box;
use futures::{future::BoxFuture, TryFuture, TryFutureExt};

use crate::{Error, Work};

pub struct Wrap<T, F, C> {
    task: T,
    func: F,
    ctx: PhantomData<C>,
}

impl<T, F, C> Wrap<T, F, C> {
    pub fn new(task: T, func: F) -> Wrap<T, F, C> {
        Wrap {
            task,
            func,
            ctx: PhantomData,
        }
    }
}

impl<T, F, U, C, R> Work<C, R> for Wrap<T, F, C>
where
    T: Work<C, R> + Clone + Send + 'static,
    F: Fn(C, R, T) -> U + Clone + Send + 'static,
    U: TryFuture + Send,
    U::Error: Into<Error>,
    C: Send + 'static,
    R: Send + 'static,
{
    type Output = U::Ok;
    type Future = BoxFuture<'static, Result<U::Ok, Error>>;

    fn call(&self, ctx: C, package: R) -> Self::Future {
        let work = self.task.clone();
        let func = self.func.clone();
        Box::pin(async move { (func)(ctx, package, work).map_err(Into::into).await })
    }
}
