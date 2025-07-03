use bycat_container::modules::{BoxModule, BuildContext, InitContext, ModuleBox};
use rquickjs::Ctx;

use crate::backend::Quick;

pub struct JsBuilderContext<'js> {
    pub ctx: Ctx<'js>,
}

impl<'js> BuildContext<'js> for JsBuilderContext<'js> {
    type Context = JsContext<'js>;
}

pub struct JsContext<'js> {
    ctx: Ctx<'js>,
}

pub struct JsInitContext<'js> {
    pub inits: Vec<BoxModule<'js, JsBuilderContext<'js>>>,
}

impl<'js> InitContext<'js> for JsInitContext<'js> {
    type Backend = Quick;

    fn add_module<T>(&mut self, module: T)
    where
        T: bycat_container::modules::Module<
                'js,
                bycat_container::modules::BuildContextType<'js, Self::Backend>,
            > + 'js,
    {
        self.inits.push(ModuleBox::new(module));
    }
}
