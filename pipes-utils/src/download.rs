use std::path::PathBuf;

use futures::future::BoxFuture;
use pipes::{http::HttpWork, IntoPackage, IntoPackageWork, Package, Work, WorkExt};
use relative_path::RelativePath;

#[derive(Debug, Clone)]
pub struct Download {
    cache: PathBuf,
    work: IntoPackageWork<HttpWork, ()>,
}

impl Download {
    pub fn new(path: impl Into<PathBuf>) -> Download {
        Self::new_with(reqwest::Client::new(), path)
    }

    pub fn new_with(client: reqwest::Client, path: impl Into<PathBuf>) -> Download {
        Download {
            cache: path.into(),
            work: HttpWork::new(client).into_package(),
        }
    }
}

impl<C> Work<C, reqwest::Request> for Download {
    type Output = Package;

    type Future<'a> = BoxFuture<'a, Result<Self::Output, pipes::Error>>;

    fn call<'a>(&'a self, _ctx: C, req: reqwest::Request) -> Self::Future<'a> {
        let cache = self.cache.clone();
        let work = self.work.clone();
        Box::pin(async move {
            let url = req.url();

            if !tokio::fs::try_exists(&cache)
                .await
                .map_err(pipes::Error::new)?
            {
                tokio::fs::create_dir_all(&cache).await.ok();
            }

            let request_path = RelativePath::new(url.path());
            let file_name = request_path.file_name().unwrap_or("unknown").to_string();

            let cache_name = format!(
                "{}-{}",
                url.domain().unwrap().replace(".", "_"),
                url.path().replace("/", "_").replace("-", "_")
            );

            let cache_path = RelativePath::new(&cache_name);

            let cache_file_name = cache_path.file_name().unwrap_or(&cache_name);

            let full_cache_path = cache_path.to_logical_path(&cache);

            let mut pkg = if tokio::fs::try_exists(&full_cache_path)
                .await
                .map_err(pipes::Error::new)?
            {
                let pkg = pipes::fs::File::from_path(&full_cache_path)
                    .await?
                    .into_package()
                    .await?;

                pkg
            } else {
                tracing::debug!("Downloading: {}", url);
                work.call((), req).await?
            };

            pkg.set_path(cache_file_name);

            pkg.write_to(cache).await?;

            pkg.set_path(file_name);

            Ok(pkg)
        })
    }
}
