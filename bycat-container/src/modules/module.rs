use crate::modules::BuildContext;
use bycat_error::Error;
use heather::{HBoxFuture, HSend, HSendSync};
pub trait Module<'ctx, C: BuildContext<'ctx>> {
    fn build<'a>(self, ctx: &'a mut C) -> impl Future<Output = Result<(), Error>> + HSend + 'a
    where
        Self: 'a;
}

impl<'ctx, T, C> Module<'ctx, C> for T
where
    T: FnOnce(&mut C) -> Result<(), Error> + HSend,
    C: BuildContext<'ctx> + HSendSync,
{
    fn build<'a>(self, ctx: &'a mut C) -> impl Future<Output = Result<(), Error>> + HSend + 'a
    where
        Self: 'a,
    {
        async move { (self)(ctx) }
    }
}

pub trait DynModule<'ctx, C: BuildContext<'ctx>>: HSendSync {
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> HBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a;
}

pub type BoxModule<'a, C> = Box<dyn DynModule<'a, C> + 'a>;

pub struct ModuleBox<T>(T);

impl<T> ModuleBox<T> {
    pub fn new<'a, C>(module: T) -> BoxModule<'a, C>
    where
        C: BuildContext<'a> + HSendSync,
        T: Module<'a, C> + HSendSync + 'a,
    {
        Box::new(ModuleBox(module))
    }
}

impl<'ctx, C, T> DynModule<'ctx, C> for ModuleBox<T>
where
    C: BuildContext<'ctx> + HSendSync,
    T: Module<'ctx, C> + HSendSync,
{
    fn build<'a>(self: Box<Self>, ctx: &'a mut C) -> HBoxFuture<'a, Result<(), Error>>
    where
        Self: 'a,
    {
        Box::pin(async move { self.0.build(ctx).await.map_err(Into::into) })
    }
}
