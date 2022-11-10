//! HTTP Cookies

use std::collections::HashMap;
use std::fmt;
use std::time::SystemTime;
use std::{collections::HashSet, convert::TryInto};

use crate::header::{HeaderValue, SET_COOKIE};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Serialize, Deserialize)]
/// A single HTTP cookie.
pub struct Cookie<'a>(cookie_crate::Cookie<'a>);

// ===== impl Cookie =====

impl<'a> Cookie<'a> {
    fn parse(value: &'a HeaderValue) -> Result<Cookie<'a>, CookieParseError> {
        std::str::from_utf8(value.as_bytes())
            .map_err(cookie_crate::ParseError::from)
            .and_then(cookie_crate::Cookie::parse)
            .map_err(CookieParseError)
            .map(Cookie)
    }

    /// The name of the cookie.
    pub fn name(&self) -> &str {
        self.0.name()
    }

    /// The value of the cookie.
    pub fn value(&self) -> &str {
        self.0.value()
    }

    /// Returns true if the 'HttpOnly' directive is enabled.
    pub fn http_only(&self) -> bool {
        self.0.http_only().unwrap_or(false)
    }

    /// Returns true if the 'Secure' directive is enabled.
    pub fn secure(&self) -> bool {
        self.0.secure().unwrap_or(false)
    }

    /// Returns true if  'SameSite' directive is 'Lax'.
    pub fn same_site_lax(&self) -> bool {
        self.0.same_site() == Some(cookie_crate::SameSite::Lax)
    }

    /// Returns true if  'SameSite' directive is 'Strict'.
    pub fn same_site_strict(&self) -> bool {
        self.0.same_site() == Some(cookie_crate::SameSite::Strict)
    }

    /// Returns the path directive of the cookie, if set.
    pub fn path(&self) -> Option<&str> {
        self.0.path()
    }

    /// Returns the domain directive of the cookie, if set.
    pub fn domain(&self) -> Option<&str> {
        self.0.domain()
    }

    /// Get the Max-Age information.
    pub fn max_age(&self) -> Option<std::time::Duration> {
        self.0.max_age().map(|d| {
            d.try_into()
                .expect("time::Duration into std::time::Duration")
        })
    }

    /// The cookie expiration time.
    pub fn expires(&self) -> Option<SystemTime> {
        match self.0.expires() {
            Some(cookie_crate::Expiration::DateTime(offset)) => Some(SystemTime::from(offset)),
            None | Some(cookie_crate::Expiration::Session) => None,
        }
    }
}

impl<'a> fmt::Debug for Cookie<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

pub(crate) fn extract_response_cookie_headers<'a>(
    headers: &'a http::HeaderMap,
) -> impl Iterator<Item = &'a HeaderValue> + 'a {
    headers.get_all(SET_COOKIE).iter()
}

pub(crate) fn extract_response_cookies<'a>(
    headers: &'a http::HeaderMap,
) -> impl Iterator<Item = Result<Cookie<'a>, CookieParseError>> + 'a {
    headers
        .get_all(SET_COOKIE)
        .iter()
        .map(|value| Cookie::parse(value))
}

/// Error representing a parse failure of a 'Set-Cookie' header.
pub(crate) struct CookieParseError(cookie_crate::ParseError);

impl<'a> fmt::Debug for CookieParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'a> fmt::Display for CookieParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for CookieParseError {}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CookieJar(HashMap<Url, cookie_crate::CookieJar>);

// ===== impl CookieJar =====

// impl CookieJar {
//     /// Add a cookie to this jar.
//     ///
//     /// # Example
//     ///
//     /// ```
//     /// use nightfly::{cookie::CookieJar, Url};
//     ///
//     /// let cookie = "foo=bar; Domain=yolo.local";
//     /// let url = "https://yolo.local".parse::<Url>().unwrap();
//     ///
//     /// let jar = CookieJar::default();
//     /// jar.add_cookie_str(cookie, &url);
//     ///
//     /// // and now add to a `ClientBuilder`?
//     /// ```
//     pub fn add_cookie_str(&self, cookie: &str, url: &url::Url) {
//         let cookies = cookie_crate::Cookie::parse(cookie)
//             .ok()
//             .map(|c| c.into_owned())
//             .into_iter();
//         self.store_response_cookies(cookies, url);
//     }

//     fn get_store(&mut self, url: &url::Url) -> &mut cookie_crate::CookieJar {
//         self.0.entry(url.clone()).or_default()
//     }

//     pub fn set_cookies(
//         &self,
//         cookie_headers: &mut dyn Iterator<Item = &HeaderValue>,
//         url: &url::Url,
//     ) {
//         let iter =
//             cookie_headers.filter_map(|val| Cookie::parse(val).map(|c| c.0.into_owned()).ok());

//         self.0.store_response_cookies(iter, url);
//     }

//     pub fn cookies(&self, url: &url::Url) -> Option<HeaderValue> {
//         let s = self
//             .0
//             .get_request_values(url)
//             .map(|(name, value)| format!("{}={}", name, value))
//             .collect::<Vec<_>>()
//             .join("; ");

//         if s.is_empty() {
//             return None;
//         }

//         HeaderValue::from_maybe_shared(Bytes::from(s)).ok()
//     }

//     /// Store the `cookies` received from `url`
//     pub fn store_response_cookies<I: Iterator<Item = Cookie<'static>>>(
//         &mut self,
//         cookies: I,
//         url: &Url,
//     ) {
//         let store = self.get_store(url);
//         for cookie in cookies {
//             if cookie.secure() != Some(true) || cfg!(feature = "log_secure_cookie_values") {
//                 lunatic_log::debug!("inserting Set-Cookie '{:?}'", cookie);
//             } else {
//                 lunatic_log::debug!("inserting secure cookie '{}'", cookie.name());
//             }

//             if let Err(e) = store.add(cookie.clone()) {
//                 lunatic_log::debug!("unable to store Set-Cookie: {:?}", e);
//             }
//         }
//     }

//     /// Return an `Iterator` of the cookie (`name`, `value`) pairs for `url` in the store, suitable
//     /// for use in the `Cookie` header of an HTTP request. For iteration over `Cookie` instances,
//     /// please refer to [`CookieStore::matches`].
//     pub fn get_request_values(&self, url: &Url) -> impl Iterator<Item = (&str, &str)> {
//         self.0.iter().filter(|cookie| cookie.).matches(url).into_iter().map(|c| c.name_value())
//     }
// }
