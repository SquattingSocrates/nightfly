use std::borrow::Cow;
use std::convert::TryFrom;
use std::fmt;
use std::net::SocketAddr;
use std::pin::Pin;
use std::time::Duration;

use bytes::Bytes;
use encoding_rs::{Encoding, UTF_8};
use http::{HeaderMap, HeaderValue, StatusCode, Version};
use mime::Mime;
use serde::de::DeserializeOwned;
#[cfg(feature = "json")]
use serde::de::DeserializeOwned;
#[cfg(feature = "json")]
use serde_json;
use url::Url;

use super::decoder::Accepts;
#[cfg(feature = "cookies")]
use crate::cookie;

/// Extra information about the transport when an HttpConnector is used.
///
/// # Example
///
/// ```
/// # async fn doc() -> hyper::Result<()> {
/// use hyper::Uri;
/// use hyper::client::{Client, connect::HttpInfo};
///
/// let client = Client::new();
/// let uri = Uri::from_static("http://example.com");
///
/// let res = client.get(uri).await?;
/// res
///     .extensions()
///     .get::<HttpInfo>()
///     .map(|info| {
///         println!("remote addr = {}", info.remote_addr());
///     });
/// # Ok(())
/// # }
/// ```
///
/// # Note
///
/// If a different connector is used besides [`HttpConnector`](HttpConnector),
/// this value will not exist in the extensions. Consult that specific
/// connector to see what "extra" information it might provide to responses.
#[derive(Clone, Debug)]
pub struct HttpInfo {
    remote_addr: SocketAddr,
    local_addr: SocketAddr,
}

/// A Response to a submitted `Request`.
pub struct HttpResponse {
    /// body of response
    pub body: Vec<u8>,
    /// The response's status
    pub status: StatusCode,

    /// The response's version
    pub version: Version,

    /// The response's headers
    pub headers: HeaderMap<HeaderValue>,

    pub(super) url: Url,
}

impl HttpResponse {
    pub(super) fn new(
        res: http::Response<Vec<u8>>,
        url: Url,
        accepts: Accepts,
        timeout: Option<Duration>,
    ) -> HttpResponse {
        let (mut parts, body) = res.into_parts();
        // let decoder = Decoder::detect(&mut parts.headers, body, accepts);
        // let body = decoder.decode();

        HttpResponse {
            body,
            url,
            version: parts.version,
            status: parts.status,
            headers: parts.headers,
        }
    }

    /// Get the `StatusCode` of this `Response`.
    #[inline]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Get the HTTP `Version` of this `Response`.
    #[inline]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Get the `Headers` of this `Response`.
    #[inline]
    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to the `Headers` of this `Response`.
    #[inline]
    pub fn headers_mut(&mut self) -> &mut HeaderMap {
        &mut self.headers
    }

    /// Get the content-length of this response, if known.
    ///
    /// Reasons it may not be known:
    ///
    /// - The server didn't send a `content-length` header.
    /// - The response is compressed and automatically decoded (thus changing
    ///   the actual decoded length).
    pub fn content_length(&self) -> Option<u64> {
        // add 12 because there are 3 pairs of \r\n which take up 4 bytes/octects each
        Some(self.body().len() as u64)
    }

    /// Retrieve the cookies contained in the response.
    ///
    /// Note that invalid 'Set-Cookie' headers will be ignored.
    ///
    /// # Optional
    ///
    /// This requires the optional `cookies` feature to be enabled.
    #[cfg(feature = "cookies")]
    #[cfg_attr(docsrs, doc(cfg(feature = "cookies")))]
    pub fn cookies<'a>(&'a self) -> impl Iterator<Item = cookie::Cookie<'a>> + 'a {
        cookie::extract_response_cookies(self.res.headers()).filter_map(Result::ok)
    }

    /// Get the final `Url` of this `Response`.
    #[inline]
    pub fn url(&self) -> &Url {
        &self.url
    }

    /// Get the remote address used to get this `Response`.
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        None
        // self.res
        //     .extensions()
        //     .get::<HttpInfo>()
        //     .map(|info| info.remote_addr())
    }

    // /// Returns a reference to the associated extensions.
    // pub fn extensions(&self) -> &http::Extensions {
    //     self.res.extensions()
    // }

    // /// Returns a mutable reference to the associated extensions.
    // pub fn extensions_mut(&mut self) -> &mut http::Extensions {
    //     self.res.extensions_mut()
    // }

    // body methods

    /// Get the full response text.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the REPLACEMENT CHARACTER.
    /// Encoding is determinated from the `charset` parameter of `Content-Type` header,
    /// and defaults to `utf-8` if not presented.
    ///
    /// # Example
    ///
    /// ```
    /// # fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = nightfly::get("http://httpbin.org/range/26")
    ///     .text();
    ///
    /// println!("text: {:?}", content);
    /// # Ok(())
    /// # }
    /// ```
    pub fn text(self) -> crate::Result<String> {
        self.text_with_charset("utf-8")
    }

    /// Get the full response text given a specific encoding.
    ///
    /// This method decodes the response body with BOM sniffing
    /// and with malformed sequences replaced with the REPLACEMENT CHARACTER.
    /// You can provide a default encoding for decoding the raw message, while the
    /// `charset` parameter of `Content-Type` header is still prioritized. For more information
    /// about the possible encoding name, please go to [`encoding_rs`] docs.
    ///
    /// [`encoding_rs`]: https://docs.rs/encoding_rs/0.8/encoding_rs/#relationship-with-windows-code-pages
    ///
    /// # Example
    ///
    /// ```
    /// # fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let content = nightfly::get("http://httpbin.org/range/26")
    ///     
    ///     .text_with_charset("utf-8")
    ///     ;
    ///
    /// println!("text: {:?}", content);
    /// # Ok(())
    /// # }
    /// ```
    pub fn text_with_charset(self, default_encoding: &str) -> crate::Result<String> {
        let content_type = self
            .headers()
            .get(crate::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<Mime>().ok());
        let encoding_name = content_type
            .as_ref()
            .and_then(|mime| mime.get_param("charset").map(|charset| charset.as_str()))
            .unwrap_or(default_encoding);
        let encoding = Encoding::for_label(encoding_name.as_bytes()).unwrap_or(UTF_8);

        let full = self.body();

        let (text, _, _) = encoding.decode(&full);
        if let Cow::Owned(s) = text {
            return Ok(s);
        }
        unsafe {
            // decoding returned Cow::Borrowed, meaning these bytes
            // are already valid utf8
            Ok(String::from_utf8_unchecked(full.to_vec()))
        }
    }

    /// Try to deserialize the response body as JSON.
    ///
    /// # Optional
    ///
    /// This requires the optional `json` feature enabled.
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate nightfly;
    /// # extern crate serde;
    /// #
    /// # use nightfly::Error;
    /// # use serde::Deserialize;
    /// #
    /// // This `derive` requires the `serde` dependency.
    /// #[derive(Deserialize)]
    /// struct Ip {
    ///     origin: String,
    /// }
    ///
    /// # fn run() -> Result<(), Error> {
    /// let ip = nightfly::get("http://httpbin.org/ip")
    ///     
    ///     .json::<Ip>()
    ///     ;
    ///
    /// println!("ip: {}", ip.origin);
    /// # Ok(())
    /// # }
    /// #
    /// # fn main() { }
    /// ```
    ///
    /// # Errors
    ///
    /// This method fails whenever the response body is not in JSON format
    /// or it cannot be properly deserialized to target type `T`. For more
    /// details please see [`serde_json::from_reader`].
    ///
    /// [`serde_json::from_reader`]: https://docs.serde.rs/serde_json/fn.from_reader.html
    // #[cfg(feature = "json")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    pub fn json<T: DeserializeOwned>(self) -> crate::Result<T> {
        let full = self.body();

        serde_json::from_slice(&full).map_err(crate::error::decode)
    }

    // /// Get the full response body as `Bytes`.
    // ///
    // /// # Example
    // ///
    // /// ```
    // /// # fn run() -> Result<(), Box<dyn std::error::Error>> {
    // /// let bytes = nightfly::get("http://httpbin.org/ip")
    // ///
    // ///     .bytes()
    // ///     ;
    // ///
    // /// println!("bytes: {:?}", bytes);
    // /// # Ok(())
    // /// # }
    // /// ```
    // pub fn bytes(self) -> crate::Result<Bytes> {
    //     Bytes::try_from(&self.res.body()[..])
    // }

    /// return vec
    pub fn body(&self) -> Vec<u8> {
        self.body.clone()
    }

    /// Stream a chunk of the response body.
    ///
    /// When the response body has been exhausted, this will return `None`.
    ///
    /// # Example
    ///
    /// ```
    /// # fn run() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut res = nightfly::get("https://hyper.rs");
    ///
    /// while let Some(chunk) = res.chunk() {
    ///     println!("Chunk: {:?}", chunk);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn chunk(&mut self) -> crate::Result<Option<Bytes>> {
        // if let Some(item) = self.res.body_mut().next() {
        //     Ok(Some(item?))
        // } else {
        Ok(None)
        // }
    }

    // util methods

    /// Turn a response into an error if the server returned an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use nightfly::Response;
    /// fn on_response(res: Response) {
    ///     match res.error_for_status() {
    ///         Ok(_res) => (),
    ///         Err(err) => {
    ///             // asserting a 400 as an example
    ///             // it could be any status between 400...599
    ///             assert_eq!(
    ///                 err.status(),
    ///                 Some(nightfly::StatusCode::BAD_REQUEST)
    ///             );
    ///         }
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn error_for_status(self) -> crate::Result<Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(crate::error::status_code(self.url, status))
        } else {
            Ok(self)
        }
    }

    /// Turn a reference to a response into an error if the server returned an error.
    ///
    /// # Example
    ///
    /// ```
    /// # use nightfly::Response;
    /// fn on_response(res: &Response) {
    ///     match res.error_for_status_ref() {
    ///         Ok(_res) => (),
    ///         Err(err) => {
    ///             // asserting a 400 as an example
    ///             // it could be any status between 400...599
    ///             assert_eq!(
    ///                 err.status(),
    ///                 Some(nightfly::StatusCode::BAD_REQUEST)
    ///             );
    ///         }
    ///     }
    /// }
    /// # fn main() {}
    /// ```
    pub fn error_for_status_ref(&self) -> crate::Result<&Self> {
        let status = self.status();
        if status.is_client_error() || status.is_server_error() {
            Err(crate::error::status_code(self.url.clone(), status))
        } else {
            Ok(self)
        }
    }

    // private

    // The Response's body is an implementation detail.
    // You no longer need to get a reference to it, there are async methods
    // on the `Response` itself.
    //
    // This method is just used by the blocking API.
    #[cfg(feature = "blocking")]
    pub(crate) fn body_mut(&mut self) -> &mut Decoder {
        self.res.body_mut()
    }
}

impl fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Response")
            .field("url", self.url())
            .field("status", &self.status())
            .field("headers", self.headers())
            .finish()
    }
}

// /// A `Response` can be piped as the `Body` of another request.
// impl From<Response> for Body {
//     fn from(r: Response) -> Body {
//         Body::stream(r.res.into_body())
//     }
// }

// #[cfg(test)]
// mod tests {
//     use super::Response;
//     use crate::ResponseBuilderExt;
//     use http::response::Builder;
//     use url::Url;

//     #[test]
//     fn test_from_http_response() {
//         let url = Url::parse("http://example.com").unwrap();
//         let response = Builder::new()
//             .status(200)
//             .url(url.clone())
//             .body("foo")
//             .unwrap();
//         let response = Response::from(response);

//         assert_eq!(response.status(), 200);
//         assert_eq!(*response.url(), url);
//     }
// }
