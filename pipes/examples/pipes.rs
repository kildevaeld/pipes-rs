use futures::TryStreamExt;
use mime::APPLICATION_JSON;
use pipes::{
    dest_fn,
    http::{get, HttpWork},
    work_fn, Body, Error, Package, Pipeline, Source, SourceExt, Unit, WorkExt,
};
use reqwest::{Client, Method, Request, Url};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let p2 = Pipeline::new(vec![
        get("https://lightningcss.dev/playground/index.html"),
        get("https://dr.dk"),
        get("https://docs.rs/reqwest/latest/reqwest/struct.Url.html"),
        get("https://distrowatch.com/dwres.php?resource=headlines"),
    ])
    .pipe(HttpWork::new(Client::new()).into_package())
    .concurrent()
    .spawn();

    let pipeline = Pipeline::new(p2)
        .pipe(work_fn(|ctx, package: Package| async move {
            println!("Worker!!");
            Result::<_, Error>::Ok(package)
        }))
        .pipe(work_fn(|ctx, package: Package| async move {
            println!("Worker 2!!");
            Result::<_, Error>::Ok(package)
        }))
        .concurrent()
        .spawn()
        .dest(dest_fn(|s: Package| async move {
            println!("Destination: {}", s.name());

            Result::<_, Error>::Ok(())
        }));

    let out = pipeline.run().await;

    println!("out: {:?}", out)
}
