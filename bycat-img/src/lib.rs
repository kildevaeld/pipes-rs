use std::{
    future::Future,
    io::{BufWriter, Cursor},
    marker::PhantomData,
    sync::Arc,
    task::Poll,
};

use bycat::Work;
use bycat_error::Error;
use bycat_package::{Content, Package};
use bytes::Bytes;
use futures::ready;
use heather::{HBoxFuture, HSend};
use image::{ColorType, DynamicImage};
use pin_project_lite::pin_project;

pub type ImagePackage = Package<DynamicImage>;

#[derive(Debug, Clone)]
pub enum Operation {
    Resize { width: u32, height: u32 },
    Blur { sigma: f32 },
}

pub fn imageop<C>(ops: Vec<Operation>) -> ImageOp<C> {
    ImageOp(Arc::new(ops), PhantomData)
}

#[derive(Debug)]
pub struct ImageOp<C>(Arc<Vec<Operation>>, PhantomData<C>);

impl<C> Clone for ImageOp<C> {
    fn clone(&self) -> Self {
        ImageOp(self.0.clone(), PhantomData)
    }
}

impl<C> Work<C, ImagePackage> for ImageOp<C> {
    type Output = ImagePackage;
    type Error = Error;
    type Future<'a>
        = SpawnBlockFuture<ImagePackage>
    where
        C: 'a;
    fn call<'a>(&'a self, _ctx: &'a C, mut image: ImagePackage) -> Self::Future<'a> {
        let ops = self.0.clone();
        SpawnBlockFuture {
            future: tokio::task::spawn_blocking(move || {
                let mut img = image.replace_content(DynamicImage::new(1, 1, ColorType::Rgb8));

                for op in &*ops {
                    img = match op {
                        Operation::Resize { width, height } => {
                            img.resize(*width, *height, ::image::imageops::FilterType::Nearest)
                        }
                        Operation::Blur { sigma } => img.blur(*sigma),
                    };
                }

                Result::<_, Error>::Ok(image.map_content(img))
            }),
        }
    }
}

// pub fn filter<C>(
// ) -> impl Work<C, Package, Output = Option<Package>, Future = impl Future + Send> + Sync + Send + Copy
// {
//     work_fn(|_ctx: C, pkg: Package| async move {
//         let mime = pkg.mime();
//         if mime.type_() == mime::IMAGE {
//             Result::<_, Error>::Ok(Some(pkg))
//         } else {
//             Ok(None)
//         }
//     })
// }

pub fn save<C>(format: Format) -> Save<C> {
    Save {
        format,
        ctx: PhantomData,
    }
}

pub fn load<C>() -> ImageWork<C> {
    ImageWork(PhantomData)
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

#[derive(Debug)]
pub struct Save<C> {
    format: Format,
    ctx: PhantomData<C>,
}

impl<C> Clone for Save<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Copy for Save<C> {}

impl<C> Work<C, ImagePackage> for Save<C> {
    type Output = Package<Bytes>;
    type Error = Error;

    type Future<'a>
        = SpawnBlockFuture<Package<Bytes>>
    where
        C: 'a;
    fn call<'a>(&'a self, _ctx: &'a C, mut pkg: ImagePackage) -> Self::Future<'a> {
        let format = self.format;
        SpawnBlockFuture {
            future: tokio::task::spawn_blocking(move || {
                let bytes: Bytes = format.encode(pkg.content())?.into();

                pkg.path_mut().set_extension(format.ext());

                Result::<_, Error>::Ok(pkg.map_content(bytes))
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

#[derive(Debug)]
pub struct ImageWork<C>(PhantomData<C>);

impl<C> Copy for ImageWork<C> {}

impl<C> Clone for ImageWork<C> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<C> Default for ImageWork<C> {
    fn default() -> Self {
        ImageWork(PhantomData)
    }
}

impl<C, B> Work<C, Package<B>> for ImageWork<C>
where
    B: Content + HSend,
    for<'a> B: 'a,
    B::Error: Into<Error>,
{
    type Output = ImagePackage;
    type Error = Error;

    type Future<'a>
        = HBoxFuture<'a, Result<Self::Output, Error>>
    where
        Self: 'a,
        C: 'a;

    fn call<'a>(&'a self, _ctx: &'a C, mut pkg: Package<B>) -> Self::Future<'a> {
        Box::pin(async move {
            let bytes = pkg.content_mut().bytes().await.map_err(Into::into)?;

            let img = image::ImageReader::new(Cursor::new(bytes))
                .with_guessed_format()
                .map_err(Error::new)?
                .decode()
                .map_err(Error::new)?;

            Result::<_, Error>::Ok(pkg.map_content(img))
        })
    }
}
