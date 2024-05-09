use pipes::{
    http::{get, HttpWork},
    prelude::*,
    FsDest, Unit, WorkExt,
};
use pipes_img::{Format, ImageWork, Operation};
use reqwest::Client;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let pipe = vec![get("https://loppen.dk/sites/default/files/styles/wide-image/public/Ki%21%20foto%20fra%20Frontcover%20copy%20%281%29.jpg?itok=FoSCWMbQ")].pipe(HttpWork::new(Client::new()).into_package());

    pipe.pipe(ImageWork)
        .cloned(
            pipes_img::imageop(vec![
                Operation::Resize {
                    width: 200,
                    height: 1200,
                },
                Operation::Blur { sigma: 2. },
            ])
            .pipe(pipes_img::save(Format::Webp {
                quality: 75.,
                lossless: false,
            })),
            pipes_img::save(Format::Jpg(80)),
        )
        .dest(FsDest::new("test"))
        .run()
        .await;
}
