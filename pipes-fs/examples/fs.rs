use std::path::PathBuf;

use pipes::{SourceExt, Unit, work_fn};
use pipes_fs::{FsSource, Package, Serde};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Test {
    name: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let fs = pipes::pipe(FsSource::new(PathBuf::from(".")).pattern("**/*.json"))
        .pipe(Serde::new())
        .pipe(work_fn(|_, pkg: Package<Test>| async move {
            //
            println!("{}", pkg.content().name);
            Result::<_, pipes::Error>::Ok(())
        }))
        .unit()
        .run(())
        .await;
}
