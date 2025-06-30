use crate::modules::{Backend, Module};

pub trait BuildContext<'js> {
    type Context;
}

pub type RunContextType<'js, T> = <<T as Backend>::BuildContext<'js> as BuildContext<'js>>::Context;

pub type BuildContextType<'js, T> = <T as Backend>::BuildContext<'js>;

pub trait InitContext<'ctx> {
    type Backend: Backend;
    fn add_module<T>(&mut self, module: T)
    where
        T: Module<'ctx, BuildContextType<'ctx, Self::Backend>> + 'ctx;
}
