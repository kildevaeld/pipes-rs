use std::path::PathBuf;

use futures::{StreamExt, pin_mut};
use pipes::{
    Error, FsDest, NoopWork, Package, Pipeline, Source, SourceExt, Unit, UnitExt, WorkExt, cond,
    dest_fn,
    fs::FsWork,
    http::{HttpWork, get},
    work_fn,
};
use pipes_infinity::{InifiniSource, task_fn};

pub fn pipe<C, T>(source: T) -> Pipeline<T, NoopWork, C> {
    Pipeline::new(source)
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    pipe::<(), _>(InifiniSource::new(task_fn(|ctx| async move {
        ctx.register(task_fn(|ctx| async move {
            //
            Ok("Next hello")
        }))
        .await;

        ctx.register(task_fn(|ctx| async move {
            //
            Ok("Next hello 22")
        }))
        .await;
        Ok("Hello, World!")
    })))
    .and(
        InifiniSource::new(task_fn(|ctx| async move {
            ctx.register(task_fn(|ctx| async move {
                //
                Ok(futures::stream::once(async move { Ok("Next hello 1") }).boxed())
            }))
            .await;

            ctx.register(task_fn(|ctx| async move {
                //
                Ok(futures::stream::iter([Ok("Next hello 122"), Ok("Next hello 333")]).boxed())
            }))
            .await;
            Ok(futures::stream::once(async move { Ok("Hello, Worldsource!") }).boxed())
        }))
        .flatten(),
    )
    .pipe(work_fn(|ctx, pkg| async move {
        //
        Result::<_, pipes::Error>::Ok(pkg)
    }))
    .dest(dest_fn(|ret| async move {
        println!("{}", ret);
        Result::<_, pipes::Error>::Ok(())
    }))
    .run(())
    .await;
}
