use std::process::Output;

use bycat::Work;
use bycat_container::modules::BuildContext;

pub trait HostFactory<'a, C>
where
    C: BuildContext<'a>,
{
    type Error;
    type Input;
    type Host: Host<C::Context>;
    type Future<'b>: Future<Output = Result<Self::Host, Self::Error>>
    where
        Self: 'b,
        C: 'b;

    fn prepare_context(&mut self, ctx: &mut C) -> Result<(), Self::Error>;

    fn create<'b>(&'b self, ctx: &'b mut C) -> Self::Future<'b>;
}

pub trait Host<C> {
    type Options;
    type Future: Future<Output = Result<(), Self::Error>>;
    type Error;
    fn run<T>(self, ctx: C, options: Self::Options, shutdown: T) -> Self::Future
    where
        T: Future<Output = ()>;
}
