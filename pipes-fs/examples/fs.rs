use std::path::PathBuf;

use pipes::{SourceExt, Unit, dest_fn};
use pipes_fs::{FsSource, Package, Serde};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Test {
    name: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let fs = pipes::pipe(FsSource::new(PathBuf::from(".")).pattern("**/*.json"))
        .pipe(Serde::new())
        .dest(dest_fn(|pkg: Package<Test>| async move {
            //
            println!("{}", pkg.content().name);
            Result::<_, pipes::Error>::Ok(())
        }))
        .run(())
        .await;
}
