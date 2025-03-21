use std::{path::PathBuf, sync::Arc};

use klaver::{RuntimeError, modules::Environ, pool::VmPoolOptions};
use pipes_infinity::{InifiniSource, task_fn};
use rquickjs::{CatchResultExt, Class, Function, Module, Object, Value};

mod bindings;

pub struct Kravl {
    pool: klaver::pool::Pool,
}

impl Kravl {
    pub async fn new() -> Result<Kravl, RuntimeError> {
        let modules = klaver::Options::default()
            .module::<klaver_dom::Module>()
            .module::<klaver_fs::Module>()
            // .module::<klaver_image::Module>()
            .build_environ();

        let pool = klaver::pool::Pool::builder(klaver::pool::Manager::new(VmPoolOptions {
            max_stack_size: None,
            memory_limit: None,
            modules,
            worker_thread: false,
        })?)
        .build()
        .unwrap();

        Ok(Kravl { pool })
    }

    pub async fn run(&self, paths: impl Into<Vec<String>>) -> Result<(), RuntimeError> {
        let vm = deadpool::managed::Object::take(self.pool.get().await.unwrap());

        let vm = Arc::new(vm);

        let paths = paths.into();

        InifiniSource::<_, ()>::new(task_fn(
            |task_ctx: pipes_infinity::TaskCtx<(), ()>| async move {
                for path in paths {
                    let ret = klaver::async_with!(vm => |ctx| {

                        let module = Module::import(&ctx, path).catch(&ctx)?.into_future::<Object>().await.catch(&ctx)?;

                        let function = module.get::<_, Function>("default").catch(&ctx)?;

                        let v = Class::instance(ctx.clone(), bindings::JsTaskCtx::default())?;

                        let ret = function.call::<_, Value>((v,))?;

                        ctx.globals().set("REE", v)?;

                        Ok(())
                    })
                    .await
                    .map_err(pipes::Error::new)?;
                }

                todo!()
            },
        ));

        Ok(())
    }
}
