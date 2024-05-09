use std::path::PathBuf;

use pipes::{
    cond, dest_fn,
    fs::FsWork,
    http::{get, HttpWork},
    work_fn, Error, FsDest, Package, Pipeline, SourceExt, Unit, WorkExt,
};
use reqwest::Client;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let p2 = vec![
        get("https://lightningcss.dev/playground/index.html"),
        get("https://dr.dk"),
        get("https://docs.rs/reqwest/latest/reqwest/struct.Url.html"),
        get("https://distrowatch.com/dwres.php?resource=headlines"),
    ]
    .pipe(HttpWork::new(Client::new()).into_package())
    .concurrent()
    .spawn()
    .and(
        Pipeline::new(vec![Result::<_, Error>::Ok(PathBuf::from("./Cargo.toml"))])
            .pipe(FsWork.into_package()),
    );

    let pipeline = p2
        .pipe(
            cond(
                |pkg: &Package| pkg.mime() == &mime::TEXT_HTML_UTF_8,
                work_fn(|ctx, mut pkg: Package| {
                    async move {
                        //
                        // let bytes = pkg.take_content().bytes().await?;
                        // let str = core::str::from_utf8(&bytes).map_err(Error::new)?;

                        // println!("HTML: {}", str);
                        Result::<_, Error>::Ok(pkg)
                    }
                }),
            )
            .into_package(),
        )
        // .filter(work_fn(|ctx, pkg: Package| async move {
        //     if pkg.mime() == &mime::TEXT_HTML_UTF_8 {
        //         Result::<_, Error>::Ok(None)
        //     } else {
        //         Ok(Some(pkg))
        //     }
        // }))
        .cloned(
            work_fn(|ctx, package: Package| async move {
                println!("Clone 1");
                Result::<_, Error>::Ok(package)
            }),
            work_fn(|ctx, package: Package| async move {
                println!("Clone 2");
                Result::<_, Error>::Ok(package)
            }),
        )
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
        .dest(FsDest::new("test"));

    let out = pipeline.run().await;

    println!("out: {:?}", out)
}
