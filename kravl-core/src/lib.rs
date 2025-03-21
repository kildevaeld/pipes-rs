use std::{path::PathBuf, sync::Arc};

use bindings::JsPackage;
use klaver::{RuntimeError, modules::Environ, pool::VmPoolOptions};
use pipes::{Package, Pipeline, Producer, SourceExt, dest_fn};
use pipes_infinity::{InifiniSource, task_fn};
use rquickjs::{CatchResultExt, Class, Function, Module, Object, Value};
use rquickjs_util::{Val, async_iterator::JsAsyncIterator};

mod bindings;

pub struct Kravl {
    pool: klaver::pool::Pool,
}

impl Kravl {
    pub async fn new() -> Result<Kravl, RuntimeError> {
        let modules = klaver::Options::default()
            .module::<klaver_dom::Module>()
            .module::<klaver_fs::Module>()
            .search_path(".")
            // .module::<klaver_image::Module>()
            .build_environ();

        let pool = klaver::pool::Pool::builder(
            klaver::pool::Manager::new(VmPoolOptions {
                max_stack_size: None,
                memory_limit: None,
                modules,
                worker_thread: false,
            })?
            .init(|vm| {
                Box::pin(async move {
                    vm.with(|ctx| Ok(ctx.eval::<(), _>(include_str!("./init.js")).catch(&ctx)?))
                        .await?;

                    Ok(())
                })
            }),
        )
        .build()
        .unwrap();

        Ok(Kravl { pool })
    }

    pub async fn run(&self, paths: impl Into<Vec<String>>) -> Result<(), RuntimeError> {
        let paths = paths.into();

        let (producer, rx) = Producer::<Package>::new();

        for path in paths {
            let pool = self.pool.clone();
            let producer = producer.clone();
            tokio::spawn(async move {
                let vm = pool.get().await.unwrap();

                klaver::async_with!(vm => |ctx| {

                        let runner = ctx.globals().get::<_, Function>("runTask").catch(&ctx)?;

                        let v = Class::instance(ctx.clone(), bindings::JsTaskCtx::default())?;

                        let ret = runner.call::<_, JsAsyncIterator<JsPackage>>((v, path))?;

                        while let Some(next) = ret.next().await? {
                            let pkg = next.into_package(&ctx)?;
                            producer.send(pkg);
                        }
                    Ok(())
                })
                .await
                .map_err(pipes::Error::new)
                .unwrap();
            });
        }

        drop(producer);

        let pipes = Pipeline::<_, _, ()>::new(rx).dest(dest_fn(|pkg| async {
            //
            Ok(())
        }));

        Ok(())
    }
}
