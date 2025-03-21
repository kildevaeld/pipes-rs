use std::path::PathBuf;

use kravl_core::Kravl;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let kravl = Kravl::new(PathBuf::from("./output-yall")).await.unwrap();

    kravl
        .run([
            "./kravl-core/examples/example.js".to_string(),
            "./kravl-core/examples/example2.js".to_string(),
            "./kravl-core/examples/loppen.js".to_string(),
        ])
        .await
        .unwrap();
}
