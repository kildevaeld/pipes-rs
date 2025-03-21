use std::sync::{Arc, Weak};

use klaver::pool::PooledVm;
use pipes::Package;
use pipes_infinity::TaskCtx;
use rquickjs::{FromJs, Function, JsLifetime, class::Trace};
use rquickjs_util::StringRef;

#[derive(Default)]
#[rquickjs::class]
pub struct JsTaskCtx<'js> {
    pub tasks: Vec<PathOrFunc<'js>>,
}

unsafe impl<'js> JsLifetime<'js> for JsTaskCtx<'js> {
    type Changed<'to> = JsTaskCtx<'to>;
}

impl<'js> Trace<'js> for JsTaskCtx<'js> {
    fn trace<'a>(&self, tracer: rquickjs::class::Tracer<'a, 'js>) {}
}

#[rquickjs::methods]
impl<'js> JsTaskCtx<'js> {
    pub fn push(&self, task: PathOrFunc<'js>) -> rquickjs::Result<()> {
        Ok(())
    }
}

pub enum PathOrFunc<'js> {
    Path(StringRef<'js>),
    Func(Function<'js>),
}

impl<'js> FromJs<'js> for PathOrFunc<'js> {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        if value.is_string() {
            Ok(PathOrFunc::Path(StringRef::from_js(ctx, value)?))
        } else {
            Ok(PathOrFunc::Func(Function::from_js(ctx, value)?))
        }
    }
}
