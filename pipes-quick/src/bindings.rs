use klaver::{RuntimeError, modules::GlobalInfo};
use klaver_wintercg::{console::ConsoleWriter, streams::ReadableStream};
use pipes_fs::{Body, Mime, Package};
use rquickjs::{Ctx, FromJs, Object};
use rquickjs_util::{Buffer, StringRef, Val};
use std::{borrow::Cow, str::FromStr};

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
    pub async fn into_package(self, ctx: &Ctx<'js>) -> Result<Package<Body>, RuntimeError> {
        let body = match self.content {
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
                Body::Bytes(stream.to_bytes(ctx.clone()).await?.into())
            }
            JsPackageContent::String(string) => Body::Bytes(string.as_bytes().to_vec().into()),
        };

        let mime = if let Some(mime) = self.mime {
            Mime::from_str(mime.as_str()).unwrap()
        } else {
            pipes_fs::mime::APPLICATION_OCTET_STREAM
        };

        Ok(Package::new(self.name.as_str(), mime, body))
    }
}

pub struct Meta<'js> {
    pub name: Option<StringRef<'js>>,
}

impl<'js> FromJs<'js> for Meta<'js> {
    fn from_js(ctx: &Ctx<'js>, value: rquickjs::Value<'js>) -> rquickjs::Result<Self> {
        let obj = Object::from_js(ctx, value)?;

        Ok(Meta {
            name: obj.get("name")?,
        })
    }
}

pub struct Global;

impl GlobalInfo for Global {
    fn typings() -> Option<std::borrow::Cow<'static, str>> {
        Some(Cow::Borrowed(include_str!("../module.d.ts")))
    }
    fn register(_builder: &mut klaver::modules::GlobalBuilder<'_, Self>) {}
}

pub struct Logger {
    pub task: String,
}

impl ConsoleWriter for Logger {
    fn write(&self, level: klaver_wintercg::console::Level, message: String) {
        println!("[{}:{}]: {}", self.task, level, message)
    }
}

// pub struct Module;

// impl ModuleInfo for Module {
//     const NAME: &'static str = "kravl";

//     fn typings() -> Option<Cow<'static, str>> {
//         Some(Cow::Borrowed(include_str!("./pipes.d.ts")))
//     }

//     fn register(modules: &mut klaver::modules::ModuleBuilder<'_, Self>) {
//         modules.register_source(include_bytes!("./pipes.js").to_vec());
//     }
// }
