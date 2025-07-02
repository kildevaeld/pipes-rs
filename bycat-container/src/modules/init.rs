use crate::modules::Backend;
use alloc::boxed::Box;
use bycat_error::Error;
use futures_core::future::LocalBoxFuture;

pub trait Init<B: Backend> {
    type Future<'a>: Future<Output = Result<(), Error>>
    where
        Self: 'a;
    fn init<'ctx, 'a>(&'a mut self, ctx: &'a mut B::InitContext<'ctx>) -> Self::Future<'a>;
}

impl<B: Backend, T> Init<B> for T
where
    T: Fn(&mut B::InitContext<'_>) -> Result<(), Error>,
{
    type Future<'a>
        = core::future::Ready<Result<(), Error>>
    where
        Self: 'a;

    fn init<'ctx, 'a>(
        &'a mut self,
        ctx: &'a mut <B as Backend>::InitContext<'ctx>,
    ) -> Self::Future<'a> {
        let ret = (self)(ctx);
        core::future::ready(ret)
    }
}

pub trait DynInit<B: Backend>: Send + Sync {
    fn init<'ctx, 'a>(
        &'a mut self,
        ctx: &'a mut B::InitContext<'ctx>,
    ) -> LocalBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a;
}

pub type BoxInit<'a, C> = Box<dyn DynInit<C> + 'a>;

// impl<'module, C> Init<C> for BoxInit<'module, C>
// where
//     C: Backend,
// {
//     type Future<'a>
//         = HBoxFuture<'a, Result<(), Error>>
//     where
//         Self: 'a;

//     fn init<'ctx, 'a>(
//         &'a self,
//         ctx: &'a mut <C as Backend>::InitContext<'ctx>,
//     ) -> Self::Future<'a> {
//         Box::new(async move {
//             <Self as DynInit<C>>::build(self, ctx).await?;
//             Ok(())
//         })
//     }
// }

pub struct InitBox<T>(T);

impl<T> InitBox<T> {
    pub fn new<'a, C>(module: T) -> Box<dyn DynInit<C> + 'a>
    where
        C: Backend,
        T: Init<C> + Send + Sync + 'a,
    {
        Box::new(InitBox(module))
    }
}

impl<C, T> DynInit<C> for InitBox<T>
where
    C: Backend,
    T: Init<C> + Send + Sync,
{
    fn init<'ctx, 'a>(
        &'a mut self,
        ctx: &'a mut <C as Backend>::InitContext<'ctx>,
    ) -> LocalBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a,
    {
        Box::pin(async move { self.0.init(ctx).await })
    }
}
