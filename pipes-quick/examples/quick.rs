use klaver::pool::VmPoolOptions;
use pipes::{Pipeline, Unit, dest_fn, prelude::*};
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
        .map_err(pipes::Error::new)
        .unwrap(),
    )
    .build()
    .unwrap();

    Pipeline::<_, _, ()>::new(vec![Result::<_, pipes::Error>::Ok(RelativePathBuf::from(
        "./pipes-quick/examples/example.js",
    ))])
    .pipe(QuickWork::new(pool))
    .flatten()
    .dest(dest_fn(|_| async move {
        println!("HELLO");
        Result::<_, pipes::Error>::Ok(())
    }))
    .run(())
    .await;
}
