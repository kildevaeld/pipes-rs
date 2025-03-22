use std::sync::Arc;

use bindings::{JsPackage, Meta};
use futures::future::BoxFuture;
use pipes::{Work, Source};
use pipes_fs::Package;
use relative_path::RelativePathBuf;
use rquickjs::{CatchResultExt, Class, Function, Module, Object};
use rquickjs_util::async_iterator::JsAsyncIterator;

mod bindings;

#[derive(Clone)]
pub struct QuickWork {
    pool: Arc<klaver::pool::Pool>,
}

impl QuickWork {
    pub fn new(pool: klaver::pool::Pool) -> QuickWork {
        QuickWork {
            pool: Arc::new(pool)
        }
    }
}

impl<C: Send + Sync + 'static> Work<C, RelativePathBuf> for QuickWork {
    type Output = pipes_util::ReceiverStream<Package>;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, ctx: C, path: RelativePathBuf) -> Self::Future<'a> {
        Box::pin(async move {
            let vm = self.pool.get().await.map_err(pipes::Error::new)?;
            let (sx, rx) = pipes_util::channel(1);
            tokio::spawn(async move {
                let sx_clone = sx.clone();
                let ret= klaver::async_with!(vm => |ctx| {
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
                        if sx.send(Ok(pkg)).await.is_err() {
                            break;
                        }
                    }
    
                    Ok(())
                })
                .await.map_err(pipes::Error::new);

                if let Err(err) = ret {
                    sx_clone.send(Err(err)).await.ok();
                }
            });
            
            Ok(rx.call(ctx))
        })
    }
}


impl<C: Send + Sync + 'static> Work<C, Package> for QuickWork {
    type Output = pipes_util::ReceiverStream<Package>;

    type Future<'a>
        = BoxFuture<'a, Result<Self::Output, pipes::Error>>
    where
        Self: 'a;

    fn call<'a>(&'a self, ctx: C, mut pkg: Package) -> Self::Future<'a> {
        Box::pin(async move {
            let vm = self.pool.get().await.map_err(pipes::Error::new)?;
            let (sx, rx) = pipes_util::channel(1);

            if pkg.path().extension() != Some("ts") && pkg.path().extension() != Some("js") {
                return Ok(rx.call(ctx))
            }

            let content = pkg.take_content().bytes().await?;

            let path = if pkg.path().as_str().starts_with("./") {
                pkg.path().to_relative_path_buf()
            } else {
                RelativePathBuf::from(format!("./{}", pkg.path()))
            };


            tokio::spawn(async move {
                let sx_clone = sx.clone();
                let ret= klaver::async_with!(vm => |ctx| {
                    ctx.eval::<(), _>(include_str!("./init.js")).catch(&ctx)?;

                    let (module, promise) = Module::declare(ctx.clone(), path.as_str(), content).catch(&ctx)?.eval().catch(&ctx)?;

                    promise.into_future::<()>().await.catch(&ctx)?;
                        

                    let meta: Option<Meta> = module.get("meta").catch(&ctx)?;
    
                    let console = ctx.globals().get::<_, Class<klaver_wintercg::console::Console>>("console").catch(&ctx)?;
                    console.borrow_mut().set_writer(bindings::Logger {
                        task: meta.as_ref().and_then(|m| m.name.as_ref().map(|m| m.as_str().to_string())).unwrap_or_else(|| path.to_string())
                    }).catch(&ctx)?;
    
    
                    let runner = ctx.globals().get::<_, Function>("__runTask").catch(&ctx)?;
                   
                    let ret = runner.call::<_, JsAsyncIterator<JsPackage>>((path.as_str(),)).catch(&ctx)?;
                    while let Some(next) = ret.next().await.catch(&ctx)? {
                        let pkg = next.into_package(&ctx).await?;
                        if sx.send(Ok(pkg)).await.is_err() {
                            break;
                        }
                    }
    
                    Ok(())
                })
                .await.map_err(pipes::Error::new);
                
                if let Err(err) = ret {
                    sx_clone.send(Err(err)).await.ok();
                }
            });
            
            Ok(rx.call(ctx))
        })
    }
}
