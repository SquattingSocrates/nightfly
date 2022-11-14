pub mod builder;

pub use builder::*;

use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::io::Write;
use std::time::Duration;

use http::header::{self, Entry, HeaderMap, HeaderValue, ACCEPT_ENCODING, RANGE};
use http::Version;
use lunatic::process::{
    AbstractProcess, ProcessRef, Request as LunaticRequest, RequestHandler, StartProcess,
};
use lunatic::Tag;
use serde::{Deserialize, Serialize};

#[cfg(feature = "cookies")]
use crate::cookie;
use crate::error;
use crate::lunatic_impl::request::{hashmap_from_header_map, InnerRequest};
use crate::lunatic_impl::response::SerializableResponse;
use crate::lunatic_impl::{
    decoder::{parse_response, Accepts},
    http_stream::HttpStream,
    request::{PendingRequest, Request, RequestBuilder},
    response::HttpResponse,
};
use crate::redirect;
pub use crate::{Body, ClientBuilder};
use crate::{IntoUrl, Method, Url};

#[derive(Clone)]
pub struct InnerClient {
    pub(crate) accepts: Accepts,
    #[cfg(feature = "cookies")]
    pub(crate) cookie_store: Option<Arc<cookie::Jar>>,
    pub(crate) headers: HeaderMap,
    pub(crate) redirect_policy: redirect::Policy,
    pub(crate) referer: bool,
    pub(crate) request_timeout: Option<Duration>,
    // pub(crate) proxies: Arc<Vec<Proxy>>,
    // pub(crate) proxies_maybe_http_auth: bool,
    pub(crate) https_only: bool,
    pub(crate) stream_map: HashMap<HostRef, HttpStream>,
}

/// encode request as http text
pub fn request_to_vec(
    method: Method,
    uri: Url,
    mut headers: HeaderMap,
    body: Option<Body>,
    version: Version,
) -> Vec<u8> {
    let mut request_buffer: Vec<u8> = Vec::new();
    if let Some(body) = &body {
        headers.append(header::CONTENT_LENGTH, HeaderValue::from(body.len()));
    }

    // writing status line
    let path = if let Some(query) = uri.query() {
        format!("{}?{}", uri.path(), query)
    } else {
        uri.path().to_string()
    };
    request_buffer.extend(format!("{} {} {:?}\r\n", method, path, version,).as_bytes());
    // writing headers
    for (key, value) in headers.iter() {
        if let Ok(value) = String::from_utf8(value.as_ref().to_vec()) {
            request_buffer.extend(format!("{}: {}\r\n", key, value).as_bytes());
        }
    }
    // separator between header and data
    request_buffer.extend("\r\n".as_bytes());
    if let Some(body) = body {
        request_buffer.extend(body.inner());
    }

    request_buffer
}

impl AbstractProcess for InnerClient {
    type Arg = ClientBuilder;
    type State = Self;

    fn init(_: ProcessRef<Self>, builder: ClientBuilder) -> Self {
        builder.build_inner().unwrap()
    }

    fn terminate(_state: Self::State) {
        println!("Shutdown process");
    }

    fn handle_link_trapped(_state: &mut Self::State, _: Tag) {
        println!("Link trapped");
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ExecuteRequest(InnerRequest);

impl RequestHandler<ExecuteRequest> for InnerClient {
    type Response = crate::Result<SerializableResponse>;
    fn handle(state: &mut Self::State, ExecuteRequest(request): ExecuteRequest) -> Self::Response {
        let res = state.execute_request(request, vec![])?;
        Ok(SerializableResponse {
            body: res.body,
            status: res.status.as_u16(),
            version: res.version,
            headers: hashmap_from_header_map(res.headers),
            url: res.url,
        })
    }
}

/// An http `Client` to make Requests with.
///
/// The Client is a wrapper for a process so
/// The Client has various configuration values to tweak, but the defaults
/// are set to what is usually the most commonly desired value. To configure a
/// `Client`, use `Client::builder()`.
///
/// The `Client` holds a connection pool internally, so it is advised that
/// you create one and **reuse** it.
///
/// You do **not** have to wrap the `Client` in an [`Rc`] or [`Arc`] to **reuse** it,
/// because it already wraps a ProcessRef and that ensures that any incoming messages
/// will be processed in order, even if called at the same time from different processes.
///
/// Of course, as any usual ProcessRef, the Client struct is cloneable and serialisable
/// so it's easy to pass around between processes
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Client(pub ProcessRef<InnerClient>);

impl Default for Client {
    fn default() -> Self {
        let builder = ClientBuilder::new();
        let proc = InnerClient::start_link(builder, None);
        Client(proc)
    }
}

impl Client {
    /// Constructs a new `Client`.
    ///
    /// # Panics
    ///
    /// This method panics if a TLS backend cannot be initialized, or the resolver
    /// cannot load the system configuration.
    ///
    /// Use `Client::builder()` if you wish to handle the failure as an `Error`
    /// instead of panicking.
    pub fn new() -> Client {
        Client::default()
    }

    /// Convenience method to make a `GET` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn get<U>(&self, url: U) -> RequestBuilder
    where
        U: IntoUrl,
    {
        self.request(Method::GET, url)
    }

    /// Convenience method to make a `POST` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::POST, url)
    }

    /// Convenience method to make a `PUT` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PUT, url)
    }

    /// Convenience method to make a `PATCH` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn patch<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::PATCH, url)
    }

    /// Convenience method to make a `DELETE` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::DELETE, url)
    }

    /// Convenience method to make a `HEAD` request to a URL.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn head<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.request(Method::HEAD, url)
    }

    /// Start building a `Request` with the `Method` and `Url`.
    ///
    /// Returns a `RequestBuilder`, which will allow setting headers and
    /// the request body before sending.
    ///
    /// # Errors
    ///
    /// This method fails whenever the supplied `Url` cannot be parsed.
    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        let req = url.into_url().map(move |url| Request::new(method, url));
        RequestBuilder::new(self.clone(), req)
    }

    /// Executes a `Request`.
    ///
    /// A `Request` can be built manually with `Request::new()` or obtained
    /// from a RequestBuilder with `RequestBuilder::build()`.
    ///
    /// You should prefer to use the `RequestBuilder` and
    /// `RequestBuilder::send()`.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    pub fn execute(&mut self, request: Request) -> Result<HttpResponse, crate::Error> {
        let inner: InnerRequest = request.try_into()?;
        let res = self.0.request(ExecuteRequest(inner))?;
        res.try_into()
    }

    /// Creates a `ClientBuilder` to configure a `Client`.
    ///
    /// This is the same as `ClientBuilder::new()`.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }
}

#[derive(Debug, Serialize, Clone, Deserialize, Hash, PartialEq, Eq)]
pub(crate) enum HostRef {
    Http(String),
    Https(String),
}

impl HostRef {
    pub(crate) fn new(url: &Url) -> Self {
        let protocol = url.scheme();
        if protocol == "https" {
            return HostRef::Https(format!("{}", url.host().unwrap()));
        }
        let conn_str = format!("{}:{}", url.host().unwrap(), url.port().unwrap_or(80));
        HostRef::Http(conn_str)
    }
}

impl InnerClient {
    pub(crate) fn accepts(&self) -> Accepts {
        self.accepts
    }

    /// ensures connection
    pub fn ensure_connection(&mut self, url: Url) -> crate::Result<HttpStream> {
        let host_ref = HostRef::new(&url);
        if let Some(stream) = self.stream_map.get(&host_ref) {
            return Ok(stream.to_owned());
        }
        HttpStream::connect(url)
    }

    fn fmt_fields(&self, f: &mut fmt::DebugStruct<'_, '_>) {
        // Instead of deriving Debug, only print fields when their output
        // would provide relevant or interesting data.

        #[cfg(feature = "cookies")]
        {
            if let Some(_) = self.cookie_store {
                f.field("cookie_store", &true);
            }
        }

        f.field("accepts", &self.accepts);

        // if !self.proxies.is_empty() {
        //     f.field("proxies", &self.proxies);
        // }

        // if !self.redirect_policy.is_default() {
        //     f.field("redirect_policy", &self.redirect_policy);
        // }

        if self.referer {
            f.field("referer", &true);
        }

        f.field("default_headers", &self.headers);

        if let Some(ref d) = self.request_timeout {
            f.field("timeout", d);
        }
    }

    pub(crate) fn execute_request(
        &mut self,
        req: InnerRequest,
        urls: Vec<Url>,
    ) -> crate::Result<HttpResponse> {
        let (method, url, mut headers, body, _timeout, version) = req.clone().pieces();
        if url.scheme() != "http" && url.scheme() != "https" {
            return Err(error::url_bad_scheme(url));
        }

        // check if we're in https_only mode and check the scheme of the current URL
        if self.https_only && url.scheme() != "https" {
            return Err(error::url_bad_scheme(url));
        }

        if let Some(host) = url.host() {
            headers.append("Host", HeaderValue::from_str(&host.to_string()).unwrap());
        }

        // insert default headers in the request headers
        // without overwriting already appended headers.
        for (key, value) in &self.headers {
            if let Entry::Vacant(entry) = headers.entry(key) {
                entry.insert(value.clone());
            }
        }

        // Add cookies from the cookie store.
        #[cfg(feature = "cookies")]
        {
            if let Some(cookie_store) = self.cookie_store.as_ref() {
                if headers.get(crate::header::COOKIE).is_none() {
                    add_cookie_header(&mut headers, cookie_store.clone(), &url);
                }
            }
        }

        let accept_encoding = self.accepts.as_str();

        if let Some(accept_encoding) = accept_encoding {
            if !headers.contains_key(ACCEPT_ENCODING) && !headers.contains_key(RANGE) {
                headers.insert(ACCEPT_ENCODING, HeaderValue::from_static(accept_encoding));
            }
        }

        // let uri = expect_uri(&url);

        // self.proxy_auth(&uri, &mut headers);

        let encoded = request_to_vec(
            method,
            url.clone(),
            headers.clone(),
            body,
            version.try_into().unwrap(),
        );
        lunatic_log::debug!(
            "Encoded headers {:?} | Encoded request {:?}",
            headers,
            String::from_utf8(encoded.clone())
        );

        let mut stream = self.ensure_connection(url)?;
        // if let Some(timeout) = self.request_timeout {
        //     stream.set
        // }

        stream.write_all(&encoded).unwrap();

        let response_buffer = Vec::new();

        match parse_response(response_buffer, stream.clone(), req.clone(), self) {
            Ok(res) => PendingRequest::new(res, self, req, urls).resolve(),
            Err(_e) => unimplemented!(),
        }
    }

    // fn proxy_auth(&self, dst: &Uri, headers: &mut HeaderMap) {
    //     if !self.proxies_maybe_http_auth {
    //         return;
    //     }

    //     // Only set the header here if the destination scheme is 'http',
    //     // since otherwise, the header will be included in the CONNECT tunnel
    //     // request instead.
    //     if dst.scheme() != Some(&Scheme::HTTP) {
    //         return;
    //     }

    //     // if headers.contains_key(PROXY_AUTHORIZATION) {
    //     //     return;
    //     // }

    //     // for proxy in self.proxies.iter() {
    //     //     if proxy.is_match(dst) {
    //     //         if let Some(header) = proxy.http_basic_auth(dst) {
    //     //             headers.insert(PROXY_AUTHORIZATION, header);
    //     //         }

    //     //         break;
    //     //     }
    //     // }
    // }
}

impl fmt::Debug for InnerClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("Client");
        self.fmt_fields(&mut builder);
        builder.finish()
    }
}

impl fmt::Debug for ClientBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("ClientBuilder");
        self.config.fmt_fields(&mut builder);
        builder.finish()
    }
}

// impl PendingRequest {
//     fn in_flight(self: Pin<&mut Self>) -> Pin<&mut ResponseFuture> {
//         self.project().in_flight
//     }

//     fn timeout(self: Pin<&mut Self>) -> Pin<&mut Option<Pin<Box<Sleep>>>> {
//         self.project().timeout
//     }

//     fn urls(self: Pin<&mut Self>) -> &mut Vec<Url> {
//         self.project().urls
//     }

//     fn headers(self: Pin<&mut Self>) -> &mut HeaderMap {
//         self.project().headers
//     }

//     fn retry_error(mut self: Pin<&mut Self>, err: &(dyn std::error::Error + 'static)) -> bool {
//         if !is_retryable_error(err) {
//             return false;
//         }

//         trace!("can retry {:?}", err);

//         let body = match self.body {
//             Some(Some(ref body)) => Body::reusable(body.clone()),
//             Some(None) => {
//                 debug!("error was retryable, but body not reusable");
//                 return false;
//             }
//             None => Body::empty(),
//         };

//         if self.retry_count >= 2 {
//             trace!("retry count too high");
//             return false;
//         }
//         self.retry_count += 1;

//         let uri = expect_uri(&self.url);
//         let mut req = Request::builder()
//             .method(self.method.clone())
//             .uri(uri)
//             .body(body.into_stream())
//             .expect("valid request parts");

//         *req.headers_mut() = self.headers.clone();

//         *self.as_mut().in_flight().get_mut() = self.client.hyper.request(req);

//         true
//     }
// }

// fn is_retryable_error(err: &(dyn std::error::Error + 'static)) -> bool {
//     if let Some(cause) = err.source() {
//         if let Some(err) = cause.downcast_ref::<h2::Error>() {
//             // They sent us a graceful shutdown, try with a new connection!
//             return err.is_go_away()
//                 && err.is_remote()
//                 && err.reason() == Some(h2::Reason::NO_ERROR);
//         }
//     }
//     false
// }

#[cfg(feature = "cookies")]
pub(crate) fn add_cookie_header(
    headers: &mut HeaderMap,
    cookie_store: Arc<cookie::Jar>,
    url: &Url,
) {
    use crate::cookie::CookieStore;

    if let Some(header) = cookie_store.cookies(url) {
        headers.insert(crate::header::COOKIE, header);
    }
}

#[cfg(test)]
mod tests {
    #[lunatic::test]
    fn execute_request_rejects_invald_urls() {
        let url_str = "hxxps://www.rust-lang.org/";
        let url = url::Url::parse(url_str).unwrap();
        let result = crate::get(url.clone());

        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.is_builder());
        assert_eq!(url_str, err.url().unwrap().as_str());
    }
}
