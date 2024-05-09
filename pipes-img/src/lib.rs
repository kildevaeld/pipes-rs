use std::{
    future::Future,
    io::{BufWriter, Cursor},
    sync::Arc,
};

use futures::future::BoxFuture;
use pipes::{work_fn, Body, Error, Package, Work};
use relative_path::RelativePathBuf;

#[derive(Debug, Clone)]
pub enum Operation {
    Resize { width: u32, height: u32 },
}

pub fn imageop(
    ops: Vec<Operation>,
) -> impl Work<Image, Output = Image, Future = impl Future + Send> + Sync + Send + Clone {
    let ops = Arc::new(ops);
    work_fn(move |_ctx, image: Image| {
        let ops = ops.clone();
        async move {
            tokio::task::spawn_blocking(move || {
                let mut img = image.image;

                for op in &*ops {
                    img = match op {
                        Operation::Resize { width, height } => {
                            img.resize(*width, *height, ::image::imageops::FilterType::Nearest)
                        }
                    };
                }

                Result::<_, Error>::Ok(Image {
                    path: image.path,
                    image: img,
                })
            })
            .await
            .map_err(Error::new)?
        }
    })
}

pub fn filter(
) -> impl Work<Package, Output = Option<Package>, Future = impl Future + Send> + Sync + Send + Copy
{
    work_fn(|_ctx, pkg: Package| async move {
        let mime = pkg.mime();
        if mime.type_() == mime::IMAGE {
            Result::<_, Error>::Ok(Some(pkg))
        } else {
            Ok(None)
        }
    })
}

pub fn save(
    format: image::ImageFormat,
) -> impl Work<Image, Output = Package, Future = impl Future + Send> + Sync + Send + Copy {
    work_fn(move |_ctx, img: Image| async move {
        tokio::task::spawn_blocking(move || {
            let mut bytes = Vec::default();
            let mut buf_writer = BufWriter::new(Cursor::new(&mut bytes));
            img.image
                .write_to(&mut buf_writer, format)
                .map_err(Error::new)?;

            drop(buf_writer);

            let path = img.path.with_extension("jpg");

            let pkg = Package::new(path, mime::IMAGE_JPEG, Body::Bytes(bytes.into()));

            Result::<_, Error>::Ok(pkg)
        })
        .await
        .map_err(Error::new)?
    })
}

#[derive(Debug, Clone)]
pub struct Image {
    path: RelativePathBuf,
    image: image::DynamicImage,
}

pub struct ImageWork;

impl Work<Package> for ImageWork {
    type Output = Image;

    type Future = BoxFuture<'static, Result<Self::Output, Error>>;

    fn call(&self, ctx: pipes::Context, mut pkg: Package) -> Self::Future {
        Box::pin(async move {
            let bytes = pkg.take_content().bytes().await?;

            let img = image::io::Reader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(Error::new)?
                .decode()
                .map_err(Error::new)?;

            Result::<_, Error>::Ok(Image {
                path: pkg.path().to_relative_path_buf(),
                image: img,
            })
        })
    }
}
