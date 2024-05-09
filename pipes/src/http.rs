use core::{pin::Pin, task::Poll};

use alloc::string::ToString;
use bytes::Bytes;
use futures::{future::BoxFuture, stream::StreamExt, Future, FutureExt, Stream};
use http_body::Body as _;
use mime::Mime;
use pin_project_lite::pin_project;
use relative_path::RelativePathBuf;
use reqwest::{Client, Method, Request, Response, Url};

use crate::{Body, Error, IntoPackage, Package, Work};

pub fn get(url: &str) -> Result<Request, Error> {
    Ok(Request::new(
        Method::GET,
        Url::parse(url).map_err(Error::new)?,
    ))
}

pub struct BodyStream(reqwest::Body);

impl Stream for BodyStream {
    type Item = Result<Bytes, Error>;

    fn poll_next(
        mut self: core::pin::Pin<&mut Self>,
        cx: &mut core::task::Context<'_>,
    ) -> core::task::Poll<Option<Self::Item>> {
        loop {
            return match futures::ready!(Pin::new(&mut self.0).poll_frame(cx)) {
                Some(Ok(frame)) => {
                    // skip non-data frames
                    if let Ok(buf) = frame.into_data() {
                        Poll::Ready(Some(Ok(buf)))
                    } else {
                        continue;
                    }
                }
                Some(Err(err)) => Poll::Ready(Some(Err(Error::new(err)))),
                None => Poll::Ready(None),
            };
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpWork {
    client: Client,
}

impl Default for HttpWork {
    fn default() -> Self {
        HttpWork {
            client: Client::new(),
        }
    }
}

impl HttpWork {
    pub fn new(client: Client) -> HttpWork {
        HttpWork { client }
    }
}

impl<C> Work<C, Request> for HttpWork {
    type Output = Response;

    type Future = BoxFuture<'static, Result<Self::Output, Error>>;

    fn call(&self, _ctx: C, package: Request) -> Self::Future {
        let client = self.client.clone();
        async move { client.execute(package).await.map_err(Error::new) }.boxed()
    }
}

impl IntoPackage for Response {
    type Future = ResponseIntoPackageFuture;

    fn into_package(self) -> Self::Future {
        ResponseIntoPackageFuture { resp: self.into() }
    }
}

pin_project! {
    pub struct ResponseIntoPackageFuture {
        resp: Option<Response>,
    }
}

impl Future for ResponseIntoPackageFuture {
    type Output = Result<Package, Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let Some(resp) = this.resp.take() else {
            panic!("poll after done")
        };

        let request_path = RelativePathBuf::from(resp.url().path());

        let size = resp.content_length();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());

        let body = BodyStream(resp.into());

        let file_name = request_path.file_name().unwrap_or("unknown");

        Poll::Ready(Result::<_, Error>::Ok(Package::new(
            file_name.to_string(),
            content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM),
            Body::Stream(body.boxed()),
        )))
    }
}
