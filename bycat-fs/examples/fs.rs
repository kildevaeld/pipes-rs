use std::path::PathBuf;

use bycat::work_fn;
use bycat_error::Error;
use bycat_fs::FsSource;
use bycat_package::{Decode, Package, match_glob};
use bycat_source::{Unit, pipe, prelude::*};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Test {
    name: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let fs = pipe(FsSource::new(PathBuf::from(".")).pattern(match_glob("**/*.json")))
        .pipe(Decode::new())
        .pipe(work_fn(|_, pkg: Package<Test>| async move {
            //
            println!("{}", pkg.content().name);
            Result::<_, Error>::Ok(())
        }))
        .unit()
        .run(&())
        .await;
}
