use bycat::work_fn;
use bycat_error::Error;
use bycat_source::{Pipeline, SourceExt, Unit, pipe};
use klaver::pool::VmPoolOptions;
use pipes_quick::QuickWork;
use relative_path::RelativePathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let modules = klaver::Options::default().search_path(".").build_environ();

    let pool = klaver::pool::Pool::builder(
        klaver::pool::Manager::new(VmPoolOptions {
            max_stack_size: None,
            memory_limit: None,
            modules,
            worker_thread: false,
        })
        .map_err(Error::new)
        .unwrap(),
    )
    .build()
    .unwrap();

    pipe(vec![
        Result::<_, Error>::Ok(RelativePathBuf::from("./pipes-quick/examples/example.js")),
        Result::<_, Error>::Ok(RelativePathBuf::from("./pipes-quick/examples/example2.js")),
    ])
    .pipe(QuickWork::new(pool.clone()))
    .flatten_unordered(4)
    // .and(
    //     pipes_fs::FsSource::new(".".into())
    //         .pattern("./pipes-quick/examples/*.js")
    //         .pipe(QuickWork::new(pool))
    //         .flatten(),
    // )
    // .concurrent(work_fn(
    //     |ctx, ret| async move { bycat_error::Result::Ok(()) },
    // ))
    .unit()
    .run(&())
    .await;
}
