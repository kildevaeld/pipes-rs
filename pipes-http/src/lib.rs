use core::{pin::Pin, task::Poll};

use bytes::Bytes;
use futures::{Future, FutureExt, Stream, future::BoxFuture, stream::StreamExt};
use http_body::Body as _;
use mime::Mime;
use pipes::{Error, Work};
use pipes_package::{Content, IntoPackage, Package, StreamContent};
use reqwest::{Client, Method, Request, Response, Url};

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
    type Output = HttpResponse;

    type Future<'a> = BoxFuture<'a, Result<Self::Output, Error>>;

    fn call<'a>(&'a self, _ctx: C, package: Request) -> Self::Future<'a> {
        async move {
            self.client
                .execute(package)
                .await
                .map(HttpResponse)
                .map_err(Error::new)
        }
        .boxed()
    }
}

#[repr(transparent)]
pub struct HttpResponse(pub Response);

impl std::ops::Deref for HttpResponse {
    type Target = Response;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for HttpResponse {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<HttpResponse> for Response {
    fn from(value: HttpResponse) -> Self {
        value.0
    }
}
impl IntoPackage<StreamContent<BodyStream>> for HttpResponse {
    type Future = ResponseIntoPackageFuture;

    fn into_package(self) -> Self::Future {
        ResponseIntoPackageFuture {
            resp: self.0.into(),
        }
    }
}

pin_project_lite::pin_project! {
    pub struct ResponseIntoPackageFuture {
        resp: Option<Response>,
    }
}

impl Future for ResponseIntoPackageFuture {
    type Output = Result<Package<StreamContent<BodyStream>>, Error>;

    fn poll(self: Pin<&mut Self>, _cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let Some(mut resp) = this.resp.take() else {
            panic!("poll after done")
        };

        let request_path = relative_path::RelativePathBuf::from(resp.url().path());

        let _size = resp.content_length();
        let content_type = resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());

        let headers = std::mem::replace(resp.headers_mut(), Default::default());
        let status = resp.status();
        let body: reqwest::Body = resp.into();

        let file_name = request_path.file_name().unwrap_or("unknown");

        let mut pkg = Package::new(
            file_name.to_string(),
            content_type.unwrap_or(mime::APPLICATION_OCTET_STREAM),
            StreamContent::new(BodyStream(body)),
        );

        pkg.meta_mut().insert(headers);
        pkg.meta_mut().insert(status);

        Poll::Ready(Result::<_, Error>::Ok(pkg))
    }
}
