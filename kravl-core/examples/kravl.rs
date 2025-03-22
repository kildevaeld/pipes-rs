use std::path::PathBuf;

use klaver::pool::VmPoolOptions;
use kravl_core::{KravlDestination, KravlSource};
use pipes::{Pipeline, Unit, pipe, prelude::*};
use pipes_fs::mime;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let modules = klaver::Options::default()
        .module::<klaver_dom::Module>()
        .module::<klaver_fs::Module>()
        .search_path(".")
        .build_environ();

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

    pipe(KravlSource::new_with(
        pool,
        [
            "./kravl-core/examples/example.js".into(),
            "./kravl-core/examples/example2.js".into(),
            "./kravl-core/examples/loppen.js".into(),
        ]
        .to_vec(),
    ))
    .dest(KravlDestination::new("output-yall").append_when(mime::APPLICATION_JSON))
    .run(())
    .await;
}
