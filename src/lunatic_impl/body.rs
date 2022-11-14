use bytes::Bytes;
use serde::{Deserialize, Serialize};

/// Body struct
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Body(Vec<u8>);

impl From<String> for Body {
    fn from(s: String) -> Body {
        Body(s.into())
    }
}

impl From<&str> for Body {
    fn from(s: &str) -> Body {
        Body(s.into())
    }
}

impl From<Bytes> for Body {
    fn from(b: Bytes) -> Body {
        Body(b.into())
    }
}

impl From<Vec<u8>> for Body {
    fn from(v: Vec<u8>) -> Body {
        Body(v)
    }
}

impl From<&[u8]> for Body {
    fn from(slice: &[u8]) -> Body {
        Body(slice.into())
    }
}

impl From<()> for Body {
    fn from(_: ()) -> Body {
        Body::empty()
    }
}

impl From<HttpResponse> for Body {
    fn from(body: HttpResponse) -> Self {
        body.into()
    }
}

impl From<Body> for Bytes {
    fn from(body: Body) -> Bytes {
        Bytes::from(body.0)
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

    /// tells whether body is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
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
    string::FromUtf8Error,
};

use crate::HttpResponse;
