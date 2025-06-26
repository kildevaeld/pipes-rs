use std::{collections::{HashMap, HashSet}, path::PathBuf, pin::Pin, sync::{Arc, Mutex}};

use bindings::{JsPackage, Meta};
use klaver::{pool::VmPoolOptions};
use pipes::{dest_fn, Dest, Pipeline, Source, SourceExt,  Unit, Work};
use pipes_fs::{Body, Mime, Package};
use pipes_util::ReceiverStream;
use relative_path::RelativePathBuf;
use rquickjs::{CatchResultExt, Class, Function, Module, Object, Value};
use rquickjs_util::{Val, async_iterator::JsAsyncIterator};
use tokio::io::AsyncWriteExt;

mod bindings;




pub struct KravlSource {
    pool: klaver::pool::Pool,
    paths: Vec<RelativePathBuf>
}

impl KravlSource {
    pub async fn new(paths: Vec<RelativePathBuf>) -> Result<KravlSource, pipes::Error> {
        let modules = klaver::Options::default()
        .module::<bindings::Module>()
        .search_path(".")
        .global::<bindings::Global>()
        .build_environ();

        let pool = klaver::pool::Pool::builder(
            klaver::pool::Manager::new(VmPoolOptions {
                max_stack_size: None,
                memory_limit: None,
                modules,
                worker_thread: false,
            }).map_err(pipes::Error::new)?)
        .build()
        .unwrap();

        Ok(Self::new_with(pool, paths))
    }

    pub fn new_with(pool: klaver::pool::Pool, paths: Vec<RelativePathBuf>) -> KravlSource {
        KravlSource { pool, paths }
    }

   
}

impl<C> Source<C> for KravlSource {
    type Item = Package<Body>;

    type Stream<'a> = ReceiverStream<Self::Item>
    where
        Self: 'a;

    fn start<'a>(self, ctx: C) -> Self::Stream<'a> {
        let (producer, rx) = pipes_util::channel(5);

        for path in self.paths {
            let pool = self.pool.clone();
            let producer = producer.clone();
            tokio::spawn(async move {
                let vm = pool.get().await.unwrap();

                let cloned_path = path.clone();

                let ret = klaver::async_with!(vm => |ctx| {

                    ctx.eval::<(), _>(include_str!("./init.js")).catch(&ctx)?;


                    let module = Module::import(&ctx, path.as_str()).catch(&ctx)?.into_future::<Object>().await.catch(&ctx)?;

                    let meta: Option<Meta> = module.get("meta")?;

                    let console = ctx.globals().get::<_, Class<klaver_wintercg::console::Console>>("console")?;
                    console.borrow_mut().set_writer(bindings::Logger {
                        task: meta.as_ref().and_then(|m| m.name.as_ref().map(|m| m.as_str().to_string())).unwrap_or_else(|| path.to_string())
                    }).catch(&ctx)?;

                    let runner = ctx.globals().get::<_, Function>("__runTask").catch(&ctx)?;
                   
                    let ret = runner.call::<_, JsAsyncIterator<JsPackage>>((path.as_str(),)).catch(&ctx)?;

                    while let Some(next) = ret.next().await.catch(&ctx)? {
                        let pkg = next.into_package(&ctx).await?;
                        producer.send(Ok(pkg)).await.ok();
                    }
                    Ok(())
                })
                .await;

               if let Err(err) = ret {
                println!("{} failed: {}", cloned_path, err);
               }
            });
        }
        
        rx.start(ctx)
    }
}

pub trait Filter {
    fn append(&self, pkg: &Package<Body>) -> bool;
}


impl Filter for Mime {
    fn append(&self, pkg: &Package<Body>) -> bool {
        pkg.mime() == self
    }
}


pub struct KravlDestination {
    open_files: Mutex<HashSet<PathBuf>>,
    root: PathBuf,
    signal: tokio::sync::Notify,
    append: Vec<Box<dyn Filter>>
}

impl KravlDestination {
    pub fn new(path: impl Into<PathBuf>) -> KravlDestination {
        KravlDestination { open_files: Default::default(), root: path.into(), signal: Default::default(), append: Default::default() }
    }

    fn append(&self, pkg: &Package<Body>) -> bool {
        for filter in &self.append {
            if filter.append(pkg) {
                return true
            }
        }
        false
    }

    pub fn append_when<T>(mut self, filter: T) ->  Self  where T: Filter + 'static{
        self.append.push(Box::new(filter));
        self
    }
}

impl Dest<Package<Body>> for KravlDestination {
    type Future<'a> = Pin<Box<dyn Future<Output = Result<(), pipes::Error>> + 'a>>
    where
        Self: 'a;

    fn call<'a>(&'a self, mut req: Package<Body>) -> Self::Future<'a> {

        Box::pin(async move {
            let path = req.path().to_logical_path(&self.root);
            
            loop {
                let mut lock = self.open_files.lock().unwrap();
                if !lock.contains(&path) {
                    lock.insert(path.clone());
                    break;
                }
                drop(lock);
               
                self.signal.notified().await
            }

            if let Some(parent) = path.parent() {
                tokio::fs::create_dir_all(parent).await.ok();
            }


            if self.append(&req) {
                let mut file = tokio::fs::OpenOptions::default().append(true).create(true).open(&path).await.map_err(pipes::Error::new)?;
                let bytes = req.take_content().bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
                file.write_all(b"\n").await.map_err(pipes::Error::new)?;
            } else {
                let mut file = tokio::fs::OpenOptions::default().create(true).truncate(true).write(true).open(&path).await.map_err(pipes::Error::new)?;
                let bytes = req.take_content().bytes().await?;
                file.write_all(&bytes).await.map_err(pipes::Error::new)?;
            }


            self.open_files.lock().unwrap().remove(&path);
            self.signal.notify_waiters();

            Ok(())
        })
    }
}


// pub struct KravlWork {
//     pool: klaver::pool::Pool
// }

// impl KravlWork {
//     async fn exec(&self, path: &RelativePath) {
//         let vm = self.pool.get().await.unwrap();

//                 let cloned_path = path.clone();

//                 let ret = klaver::async_with!(vm => |ctx| {

//                     ctx.eval::<(), _>(include_str!("./init.js")).catch(&ctx)?;


//                     let module = Module::import(&ctx, path.as_str()).catch(&ctx)?.into_future::<Object>().await.catch(&ctx)?;

//                     let meta: Option<Meta> = module.get("meta")?;

//                     let console = ctx.globals().get::<_, Class<klaver_wintercg::console::Console>>("console")?;
//                     console.borrow_mut().set_writer(bindings::Logger {
//                         task: meta.as_ref().and_then(|m| m.name.as_ref().map(|m| m.as_str().to_string())).unwrap_or_else(|| path.to_string())
//                     }).catch(&ctx)?;

//                     let runner = ctx.globals().get::<_, Function>("__runTask").catch(&ctx)?;
                   
//                     let ret = runner.call::<_, JsAsyncIterator<JsPackage>>((path.as_str(),)).catch(&ctx)?;

//                     while let Some(next) = ret.next().await.catch(&ctx)? {
//                         let pkg = next.into_package(&ctx).await?;
//                         producer.send(pkg).ok();
//                     }
//                     Ok(())
//                 })
//                 .await;
//     }
// }


// impl<C, T> Work<T> for KravlWork {
//     type Output;

//     type Future<'a>
//     where
//         Self: 'a;

//     fn call<'a>(&'a self, ctx: T, package: T) -> Self::Future<'a> {
//         todo!()
//     }
// }