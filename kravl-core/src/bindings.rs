use std::sync::{Arc, Weak};

use klaver::{RuntimeError, pool::PooledVm};
use klaver_wintercg::streams::ReadableStream;
use pipes::{Body, Package};
use pipes_infinity::TaskCtx;
use rquickjs::{Ctx, FromJs, Function, JsLifetime, Object, class::Trace};
use rquickjs_util::{Buffer, StringRef, Val};

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

pub struct JsPackage<'js> {
    name: StringRef<'js>,
    mime: Option<StringRef<'js>>,
    content: JsPackageContent<'js>,
}

impl<'js> FromJs<'js> for JsPackage<'js> {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let obj = Object::from_js(ctx, value)?;

        Ok(JsPackage {
            name: obj.get("name")?,
            mime: obj.get("mime")?,
            content: obj.get("content")?,
        })
    }
}

pub enum JsPackageContent<'js> {
    String(StringRef<'js>),
    Buffer(Buffer<'js>),
    Stream(ReadableStream<'js>),
    Json(Val),
}

impl<'js> FromJs<'js> for JsPackageContent<'js> {
    fn from_js(ctx: &rquickjs::Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        if let Ok(buffer) = Buffer::from_js(ctx, value.clone()) {
            Ok(JsPackageContent::Buffer(buffer))
        } else if let Ok(string) = StringRef::from_js(ctx, value.clone()) {
            Ok(JsPackageContent::String(string))
        } else if let Ok(stream) = ReadableStream::from_js(ctx, value.clone()) {
            Ok(JsPackageContent::Stream(stream))
        } else if let Ok(val) = Val::from_js(ctx, value) {
            Ok(JsPackageContent::Json(val))
        } else {
            Err(rquickjs::Error::new_from_js("value", "package content"))
        }
    }
}

impl<'js> JsPackage<'js> {
    pub fn into_package(self, ctx: &Ctx<'js>) -> Result<Package, RuntimeError> {
        match self.content {
            JsPackageContent::Buffer(buffer) => match buffer.as_raw() {
                Some(raw) => Body::Bytes(raw.slice().to_vec().into()),
                None => Body::Empty,
            },
            JsPackageContent::Json(json) => {
                //
                match ctx.json_stringify(json)? {
                    Some(ret) => Body::Bytes(ret.to_string()?.into()),
                    None => Body::Empty,
                }
            }
            JsPackageContent::Stream(stream) => {
                todo!()
            }
            JsPackageContent::String(string) => {
                todo!()
            }
        };

        todo!()
    }
}
