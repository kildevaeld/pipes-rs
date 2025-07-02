use crate::modules::BuildContext;
use alloc::boxed::Box;
use bycat_error::Error;
use futures_core::future::LocalBoxFuture;
pub trait Module<'ctx, C: BuildContext<'ctx>> {
    fn build<'a>(self, ctx: &'a mut C) -> impl Future<Output = Result<(), Error>> + 'a
    where
        Self: 'a;
}

impl<'ctx, T, C> Module<'ctx, C> for T
where
    T: FnOnce(&mut C) -> Result<(), Error>,
    C: BuildContext<'ctx>,
{
    fn build<'a>(self, ctx: &'a mut C) -> impl Future<Output = Result<(), Error>> + 'a
    where
        Self: 'a,
    {
        async move { (self)(ctx) }
    }
}

pub trait DynModule<'ctx, C: BuildContext<'ctx>> {
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> LocalBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a;
}

pub type BoxModule<'a, C> = Box<dyn DynModule<'a, C> + 'a>;

pub struct ModuleBox<T>(T);

impl<T> ModuleBox<T> {
    pub fn new<'a, C>(module: T) -> BoxModule<'a, C>
    where
        C: BuildContext<'a>,
        T: Module<'a, C> + 'a,
    {
        Box::new(ModuleBox(module))
    }
}

impl<'ctx, C, T> DynModule<'ctx, C> for ModuleBox<T>
where
    C: BuildContext<'ctx>,
    T: Module<'ctx, C>,
{
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> LocalBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a,
    {
        Box::pin(async move { self.0.build(ctx).await.map_err(Into::into) })
    }
}
