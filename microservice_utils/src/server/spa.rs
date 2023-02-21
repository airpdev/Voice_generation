use axum::{
    body::{Body, HttpBody},
    error_handling::HandleError,
    response::Response,
    routing::{get_service, Route},
    Router,
};
use http::{Request, StatusCode};
use std::{
    any::type_name,
    convert::Infallible,
    fmt,
    future::{ready, Ready},
    io,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc,
};
use tower_http::services::{ServeDir, ServeFile};
use tower_service::Service;

/// Router for single page applications.
///
/// `SpaRouter` gives a routing setup commonly used for single page applications.
///
/// # Example
///
/// ```
/// use axum_extra::routing::SpaRouter;
/// use axum::{Router, routing::get};
///
/// let spa = SpaRouter::new("/assets", "dist");
///
/// let app = Router::new()
///     // `SpaRouter` implements `Into<Router>` so it works with `merge`
///     .merge(spa)
///     // we can still add other routes
///     .route("/api/foo", get(api_foo));
/// # let _: Router<axum::body::Body> = app;
///
/// async fn api_foo() {}
/// ```
///
/// With this setup we get this behavior:
///
/// - `GET /` will serve `index.html`
/// - `GET /assets/app.js` will serve `dist/app.js` assuming that file exists
/// - `GET /assets/doesnt_exist` will respond with `404 Not Found` assuming no
///   such file exists
/// - `GET /some/other/path` will serve `index.html` since there isn't another
///   route for it
/// - `GET /api/foo` will serve the `api_foo` handler function
pub struct SpaRouter<B = Body, T = (), F = fn(io::Error) -> Ready<StatusCode>> {
    paths: Arc<Paths>,
    handle_error: F,
    _marker: PhantomData<fn() -> (B, T)>,
}

#[derive(Debug)]
struct Paths {
    assets_path: Vec<String>,
    assets_dir: Vec<PathBuf>,
    index_file: PathBuf,
}

impl<B> SpaRouter<B, (), fn(io::Error) -> Ready<StatusCode>> {
    /// Create a new `SpaRouter`.
    ///
    /// Assets will be served at `GET /{serve_assets_at}` from the directory at `assets_dir`.
    ///
    /// The index file defaults to `assets_dir.join("index.html")`.
    pub fn new<P>(serve_assets_at: Vec<&str>, assets_dir: Vec<P>) -> Self
    where
        P: AsRef<Path>,
    {
        let assets = serve_assets_at
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>();
        let path = assets_dir
            .iter()
            .map(|x| x.as_ref().into())
            .collect::<Vec<PathBuf>>();
        Self {
            paths: Arc::new(Paths {
                assets_path: assets,
                assets_dir: path.clone(),
                index_file: path
                    .first()
                    .expect("failed to get assets")
                    .join("index.html"),
            }),
            handle_error: |_| ready(StatusCode::INTERNAL_SERVER_ERROR),
            _marker: PhantomData,
        }
    }
}

impl<B, T, F> SpaRouter<B, T, F> {
    /// Set the path to the index file.
    ///
    /// `path` must be relative to `assets_dir` passed to [`SpaRouter::new`].
    ///
    /// # Example
    ///
    /// ```
    /// use axum_extra::routing::SpaRouter;
    /// use axum::Router;
    ///
    /// let spa = SpaRouter::new("/assets", "dist")
    ///     .index_file("another_file.html");
    ///
    /// let app = Router::new().merge(spa);
    /// # let _: Router<axum::body::Body> = app;
    /// ```
    pub fn index_file<P>(mut self, path: P) -> Self
    where
        P: AsRef<Path>,
    {
        self.paths = Arc::new(Paths {
            assets_path: self.paths.assets_path.clone(),
            assets_dir: self.paths.assets_dir.clone(),
            index_file: self.paths.assets_dir[0].join(path),
        });
        self
    }

    /// Change the function used to handle unknown IO errors.
    ///
    /// `SpaRouter` automatically maps missing files and permission denied to
    /// `404 Not Found`. The callback given here will be used for other IO errors.
    ///
    /// See [`axum::error_handling::HandleErrorLayer`] for more details.
    ///
    /// # Example
    ///
    /// ```
    /// use std::io;
    /// use axum_extra::routing::SpaRouter;
    /// use axum::{Router, http::{Method, Uri}};
    ///
    /// let spa = SpaRouter::new("/assets", "dist").handle_error(handle_error);
    ///
    /// async fn handle_error(method: Method, uri: Uri, err: io::Error) -> String {
    ///     format!("{} {} failed with {}", method, uri, err)
    /// }
    ///
    /// let app = Router::new().merge(spa);
    /// # let _: Router<axum::body::Body> = app;
    /// ```
    pub fn handle_error<T2, F2>(self, f: F2) -> SpaRouter<B, T2, F2> {
        SpaRouter {
            paths: self.paths,
            handle_error: f,
            _marker: PhantomData,
        }
    }
}

impl<B, F, T> From<SpaRouter<B, T, F>> for Router<B>
where
    F: Clone + Send + 'static,
    HandleError<Route<B, io::Error>, F, T>:
        Service<Request<B>, Response = Response, Error = Infallible>,
    <HandleError<Route<B, io::Error>, F, T> as Service<Request<B>>>::Future: Send,
    B: HttpBody + Send + 'static,
    T: 'static,
{
    fn from(spa: SpaRouter<B, T, F>) -> Self {
        let assets_path = &spa.paths.assets_path;

        let mut router = Router::new();

        for (i, asset_path) in assets_path.iter().enumerate() {
            let service = get_service(ServeDir::new(&spa.paths.assets_dir[i]))
                .handle_error(spa.handle_error.clone());

            router = router.nest(asset_path, service);
        }

        router.fallback(
            get_service(ServeFile::new(&spa.paths.index_file)).handle_error(spa.handle_error),
        )
    }
}

impl<B, T, F> fmt::Debug for SpaRouter<B, T, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            paths,
            handle_error: _,
            _marker,
        } = self;

        f.debug_struct("SpaRouter")
            .field("paths", &paths)
            .field("handle_error", &format_args!("{}", type_name::<F>()))
            .field("request_body_type", &format_args!("{}", type_name::<B>()))
            .field(
                "extractor_input_type",
                &format_args!("{}", type_name::<T>()),
            )
            .finish()
    }
}

impl<B, T, F> Clone for SpaRouter<B, T, F>
where
    F: Clone,
{
    fn clone(&self) -> Self {
        Self {
            paths: self.paths.clone(),
            handle_error: self.handle_error.clone(),
            _marker: self._marker,
        }
    }
}
