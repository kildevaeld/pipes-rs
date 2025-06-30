use bycat_error::Error;

use crate::modules::Backend;

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

use heather::{HBoxFuture, HSend, HSendSync};

pub trait DynInit<B: Backend>: HSendSync {
    fn init<'ctx, 'a>(
        &'a mut self,
        ctx: &'a mut B::InitContext<'ctx>,
    ) -> HBoxFuture<'a, Result<(), Error>>
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
        C: Backend + HSend,
        T: Init<C> + HSendSync + 'a,
        for<'b> T::Future<'b>: HSend,
    {
        Box::new(InitBox(module))
    }
}

impl<C, T> DynInit<C> for InitBox<T>
where
    C: Backend + HSend,
    T: Init<C> + HSendSync,
    for<'a> T::Future<'a>: HSend,
{
    fn init<'ctx, 'a>(
        &'a mut self,
        ctx: &'a mut <C as Backend>::InitContext<'ctx>,
    ) -> HBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a,
    {
        Box::pin(async move { self.0.init(ctx).await })
    }
}
