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
    pipe(InifiniSource::new(task_fn(|ctx| async move {
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
    .and(InifiniSource::new(task_fn(|ctx| async move {
        ctx.register(task_fn(|ctx| async move {
            //
            Ok("Next hello 1")
        }))
        .await;

        ctx.register(task_fn(|ctx| async move {
            //
            Ok("Next hello 22")
        }))
        .await;
        Ok("Hello, Worldsource!")
    })))
    .dest(dest_fn(|ret| async move {
        println!("{}", ret);
        Result::<_, pipes::Error>::Ok(())
    }))
    .run(())
    .await;
}
