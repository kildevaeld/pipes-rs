use kravl_core::Kravl;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let kravl = Kravl::new().await.unwrap();

    kravl.run(["./kravl-core/examples/example.js"])
}
