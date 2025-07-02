use bycat_container::modules::{Backend, Builder, Init};
use bycat_error::Error;
use klaver::{RuntimeError, Vm};

use crate::{
    JsContext,
    context::{JsBuilderContext, JsInitContext},
};

#[derive(Default)]
pub struct Quick {
    builder: Builder<'static, Self>,
}

impl Backend for Quick {
    type InitContext<'ctx> = JsInitContext<'ctx>;

    type BuildContext<'ctx> = JsBuilderContext<'ctx>;
}

impl Quick {
    pub fn add<I>(&mut self, init: I) -> &mut Self
    where
        I: Init<Self> + Send + Sync + 'static,
        // for<'b> I::Future<'b>: ,
    {
        self.builder.add(init);
        self
    }

    pub async fn build(mut self) -> Result<Vm, Error> {
        let vm = Vm::new().build().await.map_err(Error::new)?;

        klaver::async_with!(vm => |ctx| {
            let mut init_ctx = JsInitContext {
                inits: Default::default(),
            };
            self.builder.build(&mut init_ctx).await.map_err(|err| RuntimeError::Custom(Box::new(err)))?;

            let mut ctx = JsBuilderContext {
                ctx
            };

            for module in init_ctx.inits {
                module.build(&mut ctx).await.map_err(|err| RuntimeError::Custom(Box::new(err)))?;
            }

            Ok(())
        }).await.map_err(Error::new)?;

        Ok(vm)
    }
}
