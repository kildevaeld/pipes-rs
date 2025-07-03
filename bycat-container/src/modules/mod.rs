mod context;
mod init;
mod module;

use alloc::vec::Vec;
use bycat_error::Error;
use heather::HSend;

pub use self::{context::*, init::*, module::*};

pub trait Backend {
    type InitContext<'ctx>;
    type BuildContext<'ctx>: BuildContext<'ctx>;
}

pub struct Builder<'a, T: Backend> {
    init: Vec<BoxInit<'a, T>>,
}

impl<'a, T> Default for Builder<'a, T>
where
    T: Backend,
{
    fn default() -> Self {
        Builder {
            init: Default::default(),
        }
    }
}

impl<'a, T: Backend> Builder<'a, T> {
    pub fn with<I>(mut self, init: I) -> Self
    where
        I: Init<T> + Send + Sync + 'a,
    {
        self.init.push(InitBox::new(init));
        self
    }

    pub fn add<I>(&mut self, init: I) -> &mut Self
    where
        I: Init<T> + Send + Sync + 'a,
        for<'b> I::Future<'b>: HSend,
    {
        self.init.push(InitBox::new(init));
        self
    }

    pub async fn build<'ctx>(&mut self, ctx: &mut T::InitContext<'ctx>) -> Result<(), Error> {
        for init in &mut self.init {
            init.init(ctx).await?;
        }

        Ok(())
    }
}
