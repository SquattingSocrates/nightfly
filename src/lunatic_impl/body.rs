use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Body struct
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Body(Vec<u8>);

impl Into<Body> for String {
    fn into(self) -> Body {
        Body(self.into())
    }
}

impl Into<Body> for &str {
    fn into(self) -> Body {
        Body(self.into())
    }
}

impl Into<Body> for Bytes {
    fn into(self) -> Body {
        Body(self.into())
    }
}

impl Into<Body> for Vec<u8> {
    fn into(self) -> Body {
        Body(self)
    }
}

impl Into<Body> for &[u8] {
    fn into(self) -> Body {
        Body(self.into())
    }
}

impl Into<Body> for () {
    fn into(self) -> Body {
        Body::empty()
    }
}

impl Into<Body> for HttpResponse {
    fn into(self) -> Body {
        self.body.into()
    }
}

impl Into<Bytes> for Body {
    fn into(self) -> Bytes {
        Bytes::from(self.0)
    }
}

impl TryInto<String> for Body {
    type Error = FromUtf8Error;

    fn try_into(self) -> Result<String, Self::Error> {
        String::from_utf8(self.0)
    }
}

impl Body {
    /// empty body
    pub fn empty() -> Body {
        Body(vec![])
    }

    /// length of body
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// retrieve body
    pub fn inner(self) -> Vec<u8> {
        self.0
    }

    /// create a json body
    pub fn json<T: Serialize>(data: T) -> crate::Result<Body> {
        match serde_json::to_string(&data) {
            Ok(r) => Ok(Body(r.into())),
            Err(_e) => Err(crate::Error::new(
                crate::error::Kind::Request,
                Some("".to_string()),
            )),
        }
    }

    /// create a regular text body
    pub fn text<T: Into<Vec<u8>>>(data: T) -> crate::Result<Body> {
        Ok(Body(data.into()))
    }
}

impl Read for Body {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        Cursor::new(self.0.clone()).read(buf)
    }
}

use std::{
    convert::TryInto,
    io::{Cursor, Read},
    str::Utf8Error,
    string::FromUtf8Error,
};

use crate::HttpResponse;
