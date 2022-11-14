use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::fmt;
use std::io::Write;
use std::str::FromStr;
use std::time::Duration;

use base64::write::EncoderWriter as Base64Encoder;
use http::header::{CONTENT_ENCODING, CONTENT_LENGTH, LOCATION, REFERER, TRANSFER_ENCODING};
use http::StatusCode;
use serde::{Deserialize, Serialize};

use super::client::InnerClient;
#[cfg(feature = "multipart")]
use super::multipart;
use super::response::HttpResponse;
#[cfg(feature = "cookies")]
use crate::cookie::{self, CookieStore};
#[cfg(feature = "multipart")]
use crate::header::CONTENT_LENGTH;
use crate::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use crate::into_url::try_uri;
#[cfg(feature = "cookies")]
use crate::lunatic_impl::client::add_cookie_header;
use crate::redirect::remove_sensitive_headers;
use crate::{error, redirect, Body, Client, Method, Url, Version};
use http::{request::Parts, Request as HttpRequest};

/// A request which can be executed with `Client::execute()`.
#[derive(Clone)]
pub struct Request {
    pub(crate) method: Method,
    pub(crate) url: Url,
    pub(crate) headers: HeaderMap,
    pub(crate) body: Option<Body>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) version: Version,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct InnerRequest {
    pub(crate) method: String,
    pub(crate) url: Url,
    pub(crate) headers: HashMap<String, Vec<String>>,
    pub(crate) body: Option<Body>,
    pub(crate) timeout: Option<Duration>,
    pub(crate) version: Version,
}

/// A builder to construct the properties of a `Request`.
///
/// To construct a `RequestBuilder`, refer to the `Client` documentation.
#[must_use = "RequestBuilder does nothing until you 'send' it"]
#[derive(Clone)]
pub struct RequestBuilder {
    client: Client,
    request: crate::Result<Request>,
}

impl TryFrom<Request> for InnerRequest {
    type Error = crate::Error;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        Ok(InnerRequest {
            method: value.method.to_string(),
            url: value.url,
            headers: hashmap_from_header_map(value.headers),
            body: value.body,
            timeout: value.timeout,
            version: value.version,
        })
    }
}

pub(crate) fn hashmap_from_header_map(headers: HeaderMap) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    let mut curr_key = String::new();
    headers.clone().into_iter().for_each(|(k, v)| {
        let (k, v) = (k.map(|x| x.to_string()), v.to_str().unwrap().to_string());
        if let Some(key) = k {
            curr_key = key;
        }
        map.entry(curr_key.clone()).or_insert(vec![]).push(v);
    });
    lunatic_log::debug!(
        "Transformed headers to internal structures {:?} | NEW {:?}",
        headers,
        map
    );
    map
}

pub(crate) fn header_map_from_hashmap(headers: HashMap<String, Vec<String>>) -> HeaderMap {
    let mut map = HeaderMap::new();
    headers.iter().for_each(|(k, v)| {
        let key = HeaderName::from_str(k.as_str()).unwrap();
        v.iter().for_each(|v| {
            map.append(key.clone(), HeaderValue::from_str(v.as_str()).unwrap());
        })
    });
    map
}

impl InnerRequest {
    pub(super) fn pieces(
        self,
    ) -> (
        Method,
        Url,
        HeaderMap,
        Option<Body>,
        Option<Duration>,
        Version,
    ) {
        // convert back into request to change less http encoding/decoding code
        (
            Method::from_str(self.method.as_str()).unwrap(),
            self.url,
            header_map_from_hashmap(self.headers),
            self.body,
            self.timeout,
            self.version,
        )
    }
}

impl Request {
    /// Constructs a new request.
    #[inline]
    pub fn new(method: Method, url: Url) -> Self {
        Request {
            method,
            url,
            headers: HeaderMap::new(),
            body: None,
            timeout: None,
            version: Version::default(),
        }
    }

    /// Get the method.
    #[inline]
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Get a mutable reference to the method.
    #[inline]
    pub fn method_mut(&mut self) -> &mut Method {
        &mut self.method
    }

    /// Get the url.
    #[inline]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get a mutable reference to the url.
    #[inline]
    pub fn url_mut(&mut self) -> &mut Url {
        &mut self.url
    }

    /// Get the headers.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to the headers.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get the body.
    #[inline]
    pub fn body(&self) -> Option<&Body> {
        self.body.as_ref()
    }

    /// Get a mutable reference to the body.
    #[inline]
    pub fn body_mut(&mut self) -> &mut Option<Body> {
        &mut self.body
    }

    /// Get the timeout.
    #[inline]
    pub fn timeout(&self) -> Option<&Duration> {
        self.timeout.as_ref()
    }

    /// Get a mutable reference to the timeout.
    #[inline]
    pub fn timeout_mut(&mut self) -> &mut Option<Duration> {
        &mut self.timeout
    }

    /// Get the http version.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Get a mutable reference to the http version.
    #[inline]
    pub fn version_mut(&mut self) -> &mut Version {
        &mut self.version
    }
}

impl RequestBuilder {
    pub(super) fn new(client: Client, request: crate::Result<Request>) -> RequestBuilder {
        let mut builder = RequestBuilder { client, request };

        let auth = builder
            .request
            .as_mut()
            .ok()
            .and_then(|req| extract_authority(&mut req.url));

        if let Some((username, password)) = auth {
            builder.basic_auth(username, password)
        } else {
            builder
        }
    }

    /// Add a `Header` to this Request.
    pub fn header<K, V>(self, key: K, value: V) -> RequestBuilder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        self.header_sensitive(key, value, false)
    }

    /// Add a `Header` to this Request with ability to define if header_value is sensitive.
    fn header_sensitive<K, V>(mut self, key: K, value: V, sensitive: bool) -> RequestBuilder
    where
        HeaderName: TryFrom<K>,
        <HeaderName as TryFrom<K>>::Error: Into<http::Error>,
        HeaderValue: TryFrom<V>,
        <HeaderValue as TryFrom<V>>::Error: Into<http::Error>,
    {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match <HeaderName as TryFrom<K>>::try_from(key) {
                Ok(key) => match <HeaderValue as TryFrom<V>>::try_from(value) {
                    Ok(mut value) => {
                        // We want to potentially make an unsensitive header
                        // to be sensitive, not the reverse. So, don't turn off
                        // a previously sensitive header.
                        if sensitive {
                            value.set_sensitive(true);
                        }
                        req.headers_mut().append(key, value);
                    }
                    Err(e) => error = Some(crate::error::builder(e.into())),
                },
                Err(e) => error = Some(crate::error::builder(e.into())),
            };
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Add a set of Headers to the existing ones on this Request.
    ///
    /// The headers will be merged in to any already set.
    pub fn headers(mut self, headers: crate::header::HeaderMap) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            crate::util::replace_headers(req.headers_mut(), headers);
        }
        self
    }

    /// Enable HTTP basic authentication.
    ///
    /// ```rust
    /// # use nightfly::Error;
    ///
    /// # fn run() -> Result<(), Error> {
    /// let client = nightfly::Client::new();
    /// let resp = client.delete("http://httpbin.org/delete")
    ///     .basic_auth("admin", Some("good password"))
    ///     .send()
    ///     ;
    /// # Ok(())
    /// # }
    /// ```
    pub fn basic_auth<U, P>(self, username: U, password: Option<P>) -> RequestBuilder
    where
        U: fmt::Display,
        P: fmt::Display,
    {
        let mut header_value = b"Basic ".to_vec();
        {
            let mut encoder = Base64Encoder::new(&mut header_value, base64::STANDARD);
            // The unwraps here are fine because Vec::write* is infallible.
            write!(encoder, "{}:", username).unwrap();
            if let Some(password) = password {
                write!(encoder, "{}", password).unwrap();
            }
        }

        self.header_sensitive(crate::header::AUTHORIZATION, header_value, true)
    }

    /// Enable HTTP bearer authentication.
    pub fn bearer_auth<T>(self, token: T) -> RequestBuilder
    where
        T: fmt::Display,
    {
        let header_value = format!("Bearer {}", token);
        self.header_sensitive(crate::header::AUTHORIZATION, header_value, true)
    }

    /// Set a body that can be turned into a `Body`
    pub fn body<T: Into<Body>>(mut self, body: T) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            *req.body_mut() = Some(body.into());
        }
        self
    }

    /// Set the request body as json.
    pub fn json<T: Serialize>(mut self, body: T) -> RequestBuilder {
        let mut serialisation_err = None;
        if let Ok(ref mut req) = self.request {
            req.headers_mut().append(
                "content-type",
                HeaderValue::from_str("application/json").unwrap(),
            );
            *req.body_mut() = match Body::json(body) {
                Ok(d) => Some(d),
                Err(err) => {
                    serialisation_err = Some(crate::error::serialization(err));
                    None
                }
            };
        }
        if let Some(serialisation_err) = serialisation_err {
            self.request = Err(serialisation_err);
        }
        self
    }

    /// Set the request body as json.
    pub fn text<T: Into<Vec<u8>>>(mut self, body: T) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            *req.body_mut() = match Body::text(body) {
                Ok(d) => Some(d),
                Err(_) => None,
            };
        }
        self
    }

    /// Enables a request timeout.
    ///
    /// The timeout is applied from when the request starts connecting until the
    /// response body has finished. It affects only this request and overrides
    /// the timeout configured using `ClientBuilder::timeout()`.
    pub fn timeout(mut self, timeout: Duration) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            *req.timeout_mut() = Some(timeout);
        }
        self
    }

    /// Sends a multipart/form-data body.
    ///
    /// ```
    /// # use nightfly::Error;
    ///
    /// # fn run() -> Result<(), Error> {
    /// let client = nightfly::Client::new();
    /// let form = nightfly::multipart::Form::new()
    ///     .text("key3", "value3")
    ///     .text("key4", "value4");
    ///
    ///
    /// let response = client.post("your url")
    ///     .multipart(form)
    ///     .send()
    ///     ;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "multipart")]
    #[cfg_attr(docsrs, doc(cfg(feature = "multipart")))]
    pub fn multipart(self, mut multipart: multipart::Form) -> RequestBuilder {
        let mut builder = self.header(
            CONTENT_TYPE,
            format!("multipart/form-data; boundary={}", multipart.boundary()).as_str(),
        );

        builder = match multipart.compute_length() {
            Some(length) => builder.header(CONTENT_LENGTH, length),
            None => builder,
        };

        if let Ok(ref mut req) = builder.request {
            *req.body_mut() = Some(multipart.stream())
        }
        builder
    }

    /// Modify the query string of the URL.
    ///
    /// Modifies the URL of this request, adding the parameters provided.
    /// This method appends and does not overwrite. This means that it can
    /// be called multiple times and that existing query parameters are not
    /// overwritten if the same key is used. The key will simply show up
    /// twice in the query string.
    /// Calling `.query(&[("foo", "a"), ("foo", "b")])` gives `"foo=a&foo=b"`.
    ///
    /// # Note
    /// This method does not support serializing a single key-value
    /// pair. Instead of using `.query(("key", "val"))`, use a sequence, such
    /// as `.query(&[("key", "val")])`. It's also possible to serialize structs
    /// and maps into a key-value pair.
    ///
    /// # Errors
    /// This method will fail if the object you provide cannot be serialized
    /// into a query string.
    pub fn query<T: Serialize + ?Sized>(mut self, query: &T) -> RequestBuilder {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            let url = req.url_mut();
            let mut pairs = url.query_pairs_mut();
            let serializer = serde_urlencoded::Serializer::new(&mut pairs);

            if let Err(err) = query.serialize(serializer) {
                error = Some(crate::error::builder(err));
            }
        }
        if let Ok(ref mut req) = self.request {
            if let Some("") = req.url().query() {
                req.url_mut().set_query(None);
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Set HTTP version
    pub fn version(mut self, version: Version) -> RequestBuilder {
        if let Ok(ref mut req) = self.request {
            req.version = version;
        }
        self
    }

    /// Send a form body.
    ///
    /// Sets the body to the url encoded serialization of the passed value,
    /// and also sets the `Content-Type: application/x-www-form-urlencoded`
    /// header.
    ///
    /// ```rust
    /// # use nightfly::Error;
    /// # use std::collections::HashMap;
    /// #
    /// # fn run() -> Result<(), Error> {
    /// let mut params = HashMap::new();
    /// params.insert("lang", "rust");
    ///
    /// let client = nightfly::Client::new();
    /// let res = client.post("http://httpbin.org")
    ///     .form(&params)
    ///     .send()
    ///     ;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Errors
    ///
    /// This method fails if the passed value cannot be serialized into
    /// url encoded format
    pub fn form<T: Serialize + ?Sized>(mut self, form: &T) -> RequestBuilder {
        let mut error = None;
        if let Ok(ref mut req) = self.request {
            match serde_urlencoded::to_string(form) {
                Ok(body) => {
                    req.headers_mut().insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/x-www-form-urlencoded"),
                    );
                    *req.body_mut() = Some(body.into());
                }
                Err(err) => error = Some(crate::error::builder(err)),
            }
        }
        if let Some(err) = error {
            self.request = Err(err);
        }
        self
    }

    /// Disable CORS on fetching the request.
    ///
    /// # WASM
    ///
    /// This option is only effective with WebAssembly target.
    ///
    /// The [request mode][mdn] will be set to 'no-cors'.
    ///
    /// [mdn]: https://developer.mozilla.org/en-US/docs/Web/API/Request/mode
    pub fn fetch_mode_no_cors(self) -> RequestBuilder {
        self
    }

    /// Build a `Request`, which can be inspected, modified and executed with
    /// `Client::execute()`.
    pub fn build(self) -> crate::Result<Request> {
        self.request
    }

    /// Constructs the Request and sends it to the target URL, returning a
    /// future Response.
    ///
    /// # Errors
    ///
    /// This method fails if there was an error while sending request,
    /// redirect loop was detected or redirect limit was exhausted.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use nightfly::Error;
    /// #
    /// # fn run() -> Result<(), Error> {
    /// let response = nightfly::Client::new()
    ///     .get("https://hyper.rs")
    ///     .send()
    ///     ;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send(mut self) -> Result<HttpResponse, crate::Error> {
        match self.request {
            Ok(req) => self.client.execute(req),
            Err(err) => Err(err),
        }
    }

    // /// Attempt to clone the RequestBuilder.
    // ///
    // /// `None` is returned if the RequestBuilder can not be cloned,
    // /// i.e. if the request body is a stream.
    // ///
    // /// # Examples
    // ///
    // /// ```
    // /// # use nightfly::Error;
    // /// #
    // /// # fn run() -> Result<(), Error> {
    // /// let client = nightfly::Client::new();
    // /// let builder = client.post("http://httpbin.org/post")
    // ///     .body("from a &str!");
    // /// let clone = builder.try_clone();
    // /// assert!(clone.is_some());
    // /// # Ok(())
    // /// # }
    // /// ```
    // pub fn try_clone(&self) -> Option<RequestBuilder> {
    //     self.request
    //         .as_ref()
    //         .ok()
    //         .and_then(|req| req.try_clone())
    //         .map(|req| RequestBuilder {
    //             client: self.client.clone(),
    //             request: Ok(req),
    //         })
    // }
}

impl fmt::Debug for Request {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt_request_fields(&mut f.debug_struct("Request"), self).finish()
    }
}

impl fmt::Debug for RequestBuilder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut builder = f.debug_struct("RequestBuilder");
        match self.request {
            Ok(ref req) => fmt_request_fields(&mut builder, req).finish(),
            Err(ref err) => builder.field("error", err).finish(),
        }
    }
}

fn fmt_request_fields<'a, 'b>(
    f: &'a mut fmt::DebugStruct<'a, 'b>,
    req: &Request,
) -> &'a mut fmt::DebugStruct<'a, 'b> {
    f.field("method", &req.method)
        .field("url", &req.url)
        .field("headers", &req.headers)
}

/// Check the request URL for a "username:password" type authority, and if
/// found, remove it from the URL and return it.
pub(crate) fn extract_authority(url: &mut Url) -> Option<(String, Option<String>)> {
    use percent_encoding::percent_decode;

    if url.has_authority() {
        let username: String = percent_decode(url.username().as_bytes())
            .decode_utf8()
            .ok()?
            .into();
        let password = url.password().and_then(|pass| {
            percent_decode(pass.as_bytes())
                .decode_utf8()
                .ok()
                .map(String::from)
        });
        if !username.is_empty() || password.is_some() {
            url.set_username("")
                .expect("has_authority means set_username shouldn't fail");
            url.set_password(None)
                .expect("has_authority means set_password shouldn't fail");
            return Some((username, password));
        }
    }

    None
}

impl<T> TryFrom<HttpRequest<T>> for Request
where
    T: Into<Body>,
{
    type Error = crate::Error;

    fn try_from(req: HttpRequest<T>) -> crate::Result<Self> {
        let (parts, body) = req.into_parts();
        let Parts {
            method,
            uri,
            headers,
            version,
            ..
        } = parts;
        let url = Url::parse(&uri.to_string()).map_err(crate::error::builder)?;
        Ok(Request {
            method,
            url,
            headers,
            body: Some(body.into()),
            timeout: None,
            version: Version::from(version),
        })
    }
}

// impl TryFrom<Request> for HttpRequest<Body> {
//     type Error = crate::Error;

//     fn try_from(req: Request) -> crate::Result<Self> {
//         let Request {
//             method,
//             url,
//             headers,
//             body,
//             version,
//             ..
//         } = req;

//         let mut req = HttpRequest::builder()
//             .version(version)
//             .method(method)
//             .uri(url.as_str())
//             .body(body.unwrap_or_else(Body::empty))
//             .map_err(crate::error::builder)?;

//         *req.headers_mut() = headers;
//         Ok(req)
//     }
// }

#[derive(Debug)]
pub(crate) struct PendingRequest<'a> {
    /// parsed response
    res: HttpResponse,
    client: &'a mut InnerClient,
    // client_process: ProcessRef<Client>,
    req: InnerRequest,
    urls: Vec<Url>,
}

impl<'a> PendingRequest<'a> {
    pub fn new(
        res: HttpResponse,
        client: &'a mut InnerClient,
        // client_process: ProcessRef<Client>,
        req: InnerRequest,
        urls: Vec<Url>,
    ) -> Self {
        Self {
            res,
            client,
            // client_process,
            req,
            urls,
        }
    }

    /// return either a parsed response or an error if there's a redirect loop
    /// or if maximum redirects were reached
    pub fn resolve(mut self) -> Result<HttpResponse, crate::Error> {
        #[cfg(feature = "cookies")]
        {
            if let Some(ref cookie_store) = self.client.cookie_store {
                let mut cookies =
                    cookie::extract_response_cookie_headers(&self.res.headers()).peekable();
                if cookies.peek().is_some() {
                    cookie_store.set_cookies(&mut cookies, &self.req.url);
                }
            }
        }
        let should_redirect = match self.res.status() {
            StatusCode::MOVED_PERMANENTLY | StatusCode::FOUND | StatusCode::SEE_OTHER => {
                // self.body = None;
                for header in &[
                    TRANSFER_ENCODING,
                    CONTENT_ENCODING,
                    CONTENT_TYPE,
                    CONTENT_LENGTH,
                ] {
                    self.res.headers.remove(header);
                }

                match self.req.method.as_str() {
                    "GET" | "HEAD" => {}
                    _ => {
                        self.req.method = "GET".to_string();
                    }
                }
                true
            }
            StatusCode::TEMPORARY_REDIRECT | StatusCode::PERMANENT_REDIRECT => true,
            _ => false,
        };
        if should_redirect {
            let loc = self.res.headers().get(LOCATION).and_then(|val| {
                let loc = (|| -> Option<Url> {
                    // Some sites may send a utf-8 Location header,
                    // even though we're supposed to treat those bytes
                    // as opaque, we'll check specifically for utf8.
                    self.req
                        .url
                        .join(std::str::from_utf8(val.as_bytes()).ok()?)
                        .ok()
                })();

                // Check that the `url` is also a valid `http::Uri`.
                //
                // If not, just log it and skip the redirect.
                let loc = loc.and_then(|url| {
                    if try_uri(&url).is_some() {
                        Some(url)
                    } else {
                        None
                    }
                });

                if loc.is_none() {
                    lunatic_log::debug!("Location header had invalid URI: {:?}", val);
                }
                loc
            });

            // map headers back to http type because it can handle multiple headers
            let mut headers = header_map_from_hashmap(self.req.headers.clone());
            if let Some(loc) = loc {
                if self.client.referer {
                    if let Some(referer) = make_referer(&loc, &self.req.url) {
                        headers.insert(REFERER, referer);
                    }
                }
                let url = self.req.url.clone();
                self.urls.push(url);
                let action = self
                    .client
                    .redirect_policy
                    .check(self.res.status(), &loc, &self.urls);

                match action {
                    redirect::ActionKind::Follow => {
                        lunatic_log::debug!(
                            "redirecting '{}' to '{}' with headers {:?}",
                            self.req.url,
                            loc,
                            headers
                        );

                        if self.client.https_only && loc.scheme() != "https" {
                            return Err(error::redirect(error::url_bad_scheme(loc.clone()), loc));
                        }

                        self.req.url = loc;

                        remove_sensitive_headers(&mut headers, &self.req.url, &self.urls);
                        let mut req = Request::new(
                            // it's fine to unwrap here because the method was constructed with a valid builder
                            Method::from_str(self.req.method.as_str()).unwrap(),
                            self.req.url.clone(),
                        );
                        req.headers = headers.clone();

                        // Add cookies from the cookie store.
                        #[cfg(feature = "cookies")]
                        {
                            if let Some(ref cookie_store) = self.client.cookie_store {
                                add_cookie_header(
                                    &mut headers,
                                    cookie_store.clone(),
                                    &self.req.url,
                                );
                            }
                        }

                        return self.client.execute_request(req.try_into()?, self.urls);
                    }
                    redirect::ActionKind::Stop => {
                        lunatic_log::debug!("redirect policy disallowed redirection to '{}'", loc);
                    }
                    redirect::ActionKind::Error(err) => {
                        return Err(crate::error::redirect(err, self.req.url.clone()));
                    }
                }
            }
        }

        Ok(self.res)
    }
}

fn make_referer(next: &Url, previous: &Url) -> Option<HeaderValue> {
    if next.scheme() == "http" && previous.scheme() == "https" {
        return None;
    }

    let mut referer = previous.clone();
    let _ = referer.set_username("");
    let _ = referer.set_password(None);
    referer.set_fragment(None);
    referer.as_str().parse().ok()
}

#[cfg(test)]
mod tests {
    use crate::Client;

    use http::{HeaderValue, Method};
    use serde::Serialize;
    use std::collections::BTreeMap;

    #[lunatic::test]
    fn add_query_append() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.get(some_url);

        let r = r.query(&[("foo", "bar")]);
        let r = r.query(&[("qux", 3)]);

        let req = r.build().expect("request is valid");
        assert_eq!(req.url().query(), Some("foo=bar&qux=3"));
    }

    #[lunatic::test]
    fn add_query_append_same() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.get(some_url);

        let r = r.query(&[("foo", "a"), ("foo", "b")]);

        let req = r.build().expect("request is valid");
        assert_eq!(req.url().query(), Some("foo=a&foo=b"));
    }

    #[lunatic::test]
    fn add_query_struct() {
        #[derive(Serialize)]
        struct Params {
            foo: String,
            qux: i32,
        }

        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.get(some_url);

        let params = Params {
            foo: "bar".into(),
            qux: 3,
        };

        let r = r.query(&params);

        let req = r.build().expect("request is valid");
        assert_eq!(req.url().query(), Some("foo=bar&qux=3"));
    }

    #[lunatic::test]
    fn add_query_map() {
        let mut params = BTreeMap::new();
        params.insert("foo", "bar");
        params.insert("qux", "three");

        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.get(some_url);

        let r = r.query(&params);

        let req = r.build().expect("request is valid");
        assert_eq!(req.url().query(), Some("foo=bar&qux=three"));
    }

    #[lunatic::test]
    fn test_replace_headers() {
        use http::HeaderMap;

        let mut headers = HeaderMap::new();
        headers.insert("foo", "bar".parse().unwrap());
        headers.append("foo", "baz".parse().unwrap());

        let client = Client::new();
        let req = client
            .get("https://hyper.rs")
            .header("im-a", "keeper")
            .header("foo", "pop me")
            .headers(headers)
            .build()
            .expect("request build");

        assert_eq!(req.headers()["im-a"], "keeper");

        let foo = req.headers().get_all("foo").iter().collect::<Vec<_>>();
        assert_eq!(foo.len(), 2);
        assert_eq!(foo[0], "bar");
        assert_eq!(foo[1], "baz");
    }

    #[lunatic::test]
    fn normalize_empty_query() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let empty_query: &[(&str, &str)] = &[];

        let req = client
            .get(some_url)
            .query(empty_query)
            .build()
            .expect("request build");

        assert_eq!(req.url().query(), None);
        assert_eq!(req.url().as_str(), "https://google.com/");
    }

    #[lunatic::test]
    fn try_clone_reusable() {
        let client = Client::new();
        let builder = client
            .post("http://httpbin.org/post")
            .header("foo", "bar")
            .text("from a &str!");
        let req = builder.clone().build().expect("request is valid");
        assert_eq!(req.url().as_str(), "http://httpbin.org/post");
        assert_eq!(req.method(), Method::POST);
        assert_eq!(req.headers()["foo"], "bar");
    }

    #[lunatic::test]
    fn try_clone_no_body() {
        let client = Client::new();
        let builder = client.get("http://httpbin.org/get");
        let req = builder.clone().build().expect("request is valid");
        assert_eq!(req.url().as_str(), "http://httpbin.org/get");
        assert_eq!(req.method(), Method::GET);
        assert!(req.body().is_none());
    }

    #[lunatic::test]
    fn convert_url_authority_into_basic_auth() {
        let client = Client::new();
        let some_url = "https://Aladdin:open sesame@localhost/";

        let req = client.get(some_url).build().expect("request build");

        assert_eq!(req.url().as_str(), "https://localhost/");
        assert_eq!(
            req.headers()["authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
    }

    #[lunatic::test]
    fn test_basic_auth_sensitive_header() {
        let client = Client::new();
        let some_url = "https://localhost/";

        let req = client
            .get(some_url)
            .basic_auth("Aladdin", Some("open sesame"))
            .build()
            .expect("request build");

        assert_eq!(req.url().as_str(), "https://localhost/");
        assert_eq!(
            req.headers()["authorization"],
            "Basic QWxhZGRpbjpvcGVuIHNlc2FtZQ=="
        );
        assert!(req.headers()["authorization"].is_sensitive());
    }

    #[lunatic::test]
    fn test_bearer_auth_sensitive_header() {
        let client = Client::new();
        let some_url = "https://localhost/";

        let req = client
            .get(some_url)
            .bearer_auth("Hold my bear")
            .build()
            .expect("request build");

        assert_eq!(req.url().as_str(), "https://localhost/");
        assert_eq!(req.headers()["authorization"], "Bearer Hold my bear");
        assert!(req.headers()["authorization"].is_sensitive());
    }

    #[lunatic::test]
    fn test_explicit_sensitive_header() {
        let client = Client::new();
        let some_url = "https://localhost/";

        let mut header = http::HeaderValue::from_static("in plain sight");
        header.set_sensitive(true);

        let req = client
            .get(some_url)
            .header("hiding", header)
            .build()
            .expect("request build");

        assert_eq!(req.url().as_str(), "https://localhost/");
        assert_eq!(req.headers()["hiding"], "in plain sight");
        assert!(req.headers()["hiding"].is_sensitive());
    }

    use serde_json;
    use std::collections::HashMap;

    #[lunatic::test]
    fn basic_get_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.get(some_url).build().unwrap();

        assert_eq!(r.method, Method::GET);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[test]
    fn basic_head_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.head(some_url).build().unwrap();

        assert_eq!(r.method, Method::HEAD);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[lunatic::test]
    fn basic_post_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.post(some_url).build().unwrap();

        assert_eq!(r.method, Method::POST);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[lunatic::test]
    fn basic_put_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.put(some_url).build().unwrap();

        assert_eq!(r.method, Method::PUT);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[lunatic::test]
    fn basic_patch_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.patch(some_url).build().unwrap();

        assert_eq!(r.method, Method::PATCH);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[lunatic::test]
    fn basic_delete_request() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.delete(some_url).build().unwrap();

        assert_eq!(r.method, Method::DELETE);
        assert_eq!(r.url.as_str(), some_url);
    }

    #[test]
    fn add_body() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.post(some_url);

        let body = "Some interesting content";

        let r = r.body(body).build().unwrap();
        let buf = r.body.unwrap().inner();

        assert_eq!(buf.iter().as_slice(), body.as_bytes());
    }

    // #[test]
    // fn add_form() {
    //     let client = Client::new();
    //     let some_url = "https://google.com/";
    //     let mut r = client.post(some_url).unwrap();

    //     let mut form_data = HashMap::new();
    //     form_data.insert("foo", "bar");

    //     let r = r.form(&form_data).unwrap().build();

    //     // Make sure the content type was set
    //     assert_eq!(
    //         r.headers.get::<ContentType>(),
    //         Some(&ContentType::form_url_encoded())
    //     );

    //     let buf = body::read_to_string(r.body.unwrap()).unwrap();

    //     let body_should_be = serde_urlencoded::to_string(&form_data).unwrap();
    //     assert_eq!(buf, body_should_be);
    // }

    #[test]
    fn add_json() {
        let client = Client::new();
        let some_url = "https://google.com/";
        let r = client.post(some_url);

        let mut json_data = HashMap::new();
        json_data.insert("foo", "bar");

        let r = r.json(&json_data).build().unwrap();

        // Make sure the content type was set
        assert_eq!(
            r.headers.get("content-type"),
            Some(&HeaderValue::from_str("application/json").unwrap())
        );

        let buf = String::from_utf8(r.body.unwrap().inner()).unwrap();

        let body_should_be = serde_json::to_string(&json_data).unwrap();
        assert_eq!(buf, body_should_be);
    }

    #[lunatic::test]
    fn add_json_fail() {
        use serde::ser::Error;
        use serde::{Serialize, Serializer};
        struct MyStruct;
        impl Serialize for MyStruct {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                Err(S::Error::custom("nope"))
            }
        }

        let client = Client::new();
        let some_url = "https://google.com/";
        let json_data = MyStruct {};
        let r = client.post(some_url);
        let res = r.json(&json_data).build();
        println!("BUILDER ERR {:?}", res);
        assert!(res.unwrap_err().is_serialization());
    }
}
