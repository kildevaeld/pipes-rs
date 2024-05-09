use std::{
    future::Future,
    io::{BufWriter, Cursor},
    sync::Arc,
    task::Poll,
};

use futures::{future::BoxFuture, ready};
use image::DynamicImage;
use pin_project_lite::pin_project;
use pipes::{work_fn, Body, Error, Package, Work};
use relative_path::RelativePathBuf;

#[derive(Debug, Clone)]
pub enum Operation {
    Resize { width: u32, height: u32 },
    Blur { sigma: f32 },
}

pub fn imageop(ops: Vec<Operation>) -> ImageOp {
    ImageOp(Arc::new(ops))
}

#[derive(Debug, Clone)]
pub struct ImageOp(Arc<Vec<Operation>>);

impl Work<Image> for ImageOp {
    type Output = Image;
    type Future = SpawnBlockFuture<Image>;
    fn call(&self, ctx: pipes::Context, image: Image) -> Self::Future {
        let ops = self.0.clone();
        SpawnBlockFuture {
            future: tokio::task::spawn_blocking(move || {
                let mut img = image.image;

                for op in &*ops {
                    img = match op {
                        Operation::Resize { width, height } => {
                            img.resize(*width, *height, ::image::imageops::FilterType::Nearest)
                        }
                        Operation::Blur { sigma } => img.blur(*sigma),
                    };
                }

                Result::<_, Error>::Ok(Image {
                    path: image.path,
                    image: img,
                })
            }),
        }
    }
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

pub fn save(format: Format) -> Save {
    Save { format }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Format {
    Jpg(u8),
    Png,
    Webp { quality: f32, lossless: bool },
}

impl Format {
    pub fn encode(&self, img: &DynamicImage) -> Result<Vec<u8>, Error> {
        let mut bytes = Vec::default();
        let buf_writer = BufWriter::new(Cursor::new(&mut bytes));
        match self {
            Format::Jpg(q) => {
                let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(buf_writer, *q);
                img.write_with_encoder(encoder).map_err(Error::new)?
            }
            Format::Png => {
                let encoder = image::codecs::png::PngEncoder::new(buf_writer);
                img.write_with_encoder(encoder).map_err(Error::new)?;
            }
            Format::Webp { quality, lossless } => {
                let encoder = webp::Encoder::from_image(img).map_err(Error::new)?;
                let mem = encoder
                    .encode_simple(*lossless, *quality)
                    .map_err(|_| Error::new("could not encode webp"))?;

                return Ok(mem.to_vec());
            }
        }

        Ok(bytes)
    }

    pub fn ext(&self) -> &str {
        match self {
            Self::Jpg(_) => "jpeg",
            Self::Png => "png",
            Self::Webp { .. } => "webp",
        }
    }

    pub fn mime(&self) -> mime::Mime {
        match self {
            Self::Jpg(_) => mime::IMAGE_JPEG,
            Self::Png => mime::IMAGE_PNG,
            Self::Webp { .. } => {
                let mime: mime::Mime = "image/webp".parse().expect("webp");
                mime
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Save {
    format: Format,
}

impl Work<Image> for Save {
    type Output = Package;
    type Future = SpawnBlockFuture<Package>;
    fn call(&self, _ctx: pipes::Context, img: Image) -> Self::Future {
        let format = self.format;
        SpawnBlockFuture {
            future: tokio::task::spawn_blocking(move || {
                let bytes = format.encode(&img.image)?;
                let path = img.path.with_extension(format.ext());

                let pkg = Package::new(path, format.mime(), Body::Bytes(bytes.into()));

                Result::<_, Error>::Ok(pkg)
            }),
        }
    }
}

pin_project! {
    pub struct SpawnBlockFuture<T> {
        #[pin]
        future: tokio::task::JoinHandle<Result<T, Error>>
    }
}

impl<T> Future for SpawnBlockFuture<T> {
    type Output = Result<T, Error>;

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = self.project();

        match ready!(this.future.poll(cx)) {
            Ok(ret) => Poll::Ready(ret),
            Err(err) => Poll::Ready(Err(Error::new(err))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Image {
    path: RelativePathBuf,
    image: image::DynamicImage,
}

#[derive(Debug, Clone, Copy)]
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
