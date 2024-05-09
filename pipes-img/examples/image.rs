use std::path::PathBuf;

use pipes::{
    cond, dest_fn,
    fs::FsWork,
    http::{get, HttpWork},
    prelude::*,
    work_fn, Error, FsDest, Package, Pipeline, SourceExt, Unit, WorkExt,
};
use pipes_img::Operation;
use reqwest::Client;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pipe = vec![get("https://loppen.dk/sites/default/files/styles/wide-image/public/Ki%21%20foto%20fra%20Frontcover%20copy%20%281%29.jpg?itok=FoSCWMbQ")].pipe(HttpWork::new(Client::new()).into_package());

    pipe.pipe(pipes_img::ImageWork)
        .cloned(
            pipes_img::imageop(vec![Operation::Resize {
                width: 100,
                height: 100,
            }])
            .pipe(pipes_img::save(image::ImageFormat::Jpeg)),
            pipes_img::save(image::ImageFormat::Jpeg),
        )
        .dest(FsDest::new("test"))
        .run()
        .await;
}
