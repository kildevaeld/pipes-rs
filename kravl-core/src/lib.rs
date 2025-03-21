use std::{collections::{HashMap, HashSet}, path::PathBuf, pin::Pin, sync::{Arc, Mutex}};

use bindings::JsPackage;
use klaver::{RuntimeError, modules::Environ, pool::VmPoolOptions};
use pipes::{dest_fn, Dest, Package, Pipeline, Producer, SourceExt, Unit};
use pipes_infinity::{InifiniSource, task_fn};
use rquickjs::{CatchResultExt, Class, Function, Module, Object, Value};
use rquickjs_util::{Val, async_iterator::JsAsyncIterator};
use tokio::io::AsyncWriteExt;

mod bindings;

pub struct Kravl {
    pool: klaver::pool::Pool,
    destination: PathBuf
}

impl Kravl {
    pub async fn new(destination: PathBuf) -> Result<Kravl, RuntimeError> {
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

        Ok(Kravl { pool, destination })
    }

    pub async fn run(&self, paths: impl Into<Vec<String>>) -> Result<(), pipes::Error> {
        let paths = paths.into();

        let (producer, rx) = Producer::<Package>::new();

        for path in paths {
            let pool = self.pool.clone();
            let producer = producer.clone();
            tokio::spawn(async move {
                let vm = pool.get().await.unwrap();

                klaver::async_with!(vm => |ctx| {

                        let runner = ctx.globals().get::<_, Function>("runTask").catch(&ctx)?;

                        let v = Class::instance(ctx.clone(), bindings::JsTaskCtx::default()).catch(&ctx)?;
                   
                        let ret = runner.call::<_, JsAsyncIterator<JsPackage>>((v, path)).catch(&ctx)?;

                        while let Some(next) = ret.next().await.catch(&ctx)? {
                            let pkg = next.into_package(&ctx).await?;
                            producer.send(pkg).unwrap();
                        }
                    Ok(())
                })
                .await
                .map_err(pipes::Error::new)
                .unwrap();
            });
        }

        drop(producer);

        Pipeline::<_, _, ()>::new(rx)
            .dest(KravlDestination {
                open_files: Default::default(),
                root: self.destination.clone(),
                signal: tokio::sync::Notify::new()
            })
            .run(())
            .await;

        Ok(())
    }
}


pub struct KravlDestination {
    open_files: Mutex<HashSet<PathBuf>>,
    root: PathBuf,
    signal: tokio::sync::Notify
}

impl Dest<Package> for KravlDestination {
    type Future<'a> = Pin<Box<dyn Future<Output = Result<(), pipes::Error>> + 'a>>
    where
        Self: 'a;

    fn call<'a>(&'a self, mut req: Package) -> Self::Future<'a> {

        Box::pin(async move {
            let path = req.path().to_logical_path(&self.root);
            
            loop {
                let mut lock = self.open_files.lock().unwrap();
                if !lock.contains(&path) {
                    lock.insert(path.clone());
                    break;
                }
                drop(lock);
                println!("WAIT");
                self.signal.notified().await
            }

            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }

            if req.mime() == &pipes::mime::APPLICATION_JSON {
                let mut file = tokio::fs::OpenOptions::default().append(true).create(true).open(&path).await.map_err(pipes::Error::new)?;
                let bytes = req.take_content().bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
                file.write_all(b"\n").await.map_err(pipes::Error::new)?;
            } else {
                let mut file = tokio::fs::OpenOptions::default().create_new(true).write(true).open(&path).await.map_err(pipes::Error::new)?;
                let bytes = req.take_content().bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
            }


            self.open_files.lock().unwrap().remove(&path);
            self.signal.notify_waiters();

            Ok(())
        })
    }
}