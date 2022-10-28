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

impl Into<Body> for Bytes {
    fn into(self) -> Body {
        Body(self.into())
    }
}

impl Into<Body> for () {
    fn into(self) -> Body {
        Body::empty()
    }
}

impl Into<Bytes> for Body {
    fn into(self) -> Bytes {
        Bytes::from(self.0)
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
            Err(e) => Err(crate::Error::new(
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

use std::io::{Cursor, Read};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EncodeError {
    #[cfg(feature = "msgpack_serializer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "msgpack_serializer")))]
    #[error("serialization to MessagePack failed: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),
    // #[cfg(feature = "json")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "json")))]
    #[error("serialization to Json failed: {0}")]
    Json(#[from] serde_json::Error),
    #[cfg(feature = "protobuf_serializer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "protobuf_serializer")))]
    #[error("serialization to Protocol Buffers failed: {0}")]
    ProtocolBuffers(#[from] protobuf::Error),
    #[error("serialization failed: {0}")]
    IO(#[from] std::io::Error),
    #[error("serialization failed: {0}")]
    Custom(String),
}

#[derive(Error, Debug)]
pub enum DecodeError {
    // #[error("deserialization from Bincode failed: {0}")]
    // Bincode(#[from] bincode::Error),
    #[cfg(feature = "msgpack_serializer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "msgpack_serializer")))]
    #[error("deserialization from MessagePack failed: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),
    // #[cfg(feature = "json_serializer")]
    // #[cfg_attr(docsrs, doc(cfg(feature = "json_serializer")))]
    #[error("deserialization from Json failed: {0}")]
    Json(#[from] serde_json::error::Error),
    #[cfg(feature = "protobuf_serializer")]
    #[cfg_attr(docsrs, doc(cfg(feature = "protobuf_serializer")))]
    #[error("deserialization from Protocol Buffers failed: {0}")]
    ProtocolBuffers(#[from] protobuf::Error),
    #[error("serialization failed: {0}")]
    IO(#[from] std::io::Error),
    #[error("deserialization failed: {0}")]
    Custom(String),
}

pub trait Serializer<M> {
    fn encode(message: &M) -> Result<Vec<u8>, EncodeError>;
    fn decode<R: Read>(reader: R) -> Result<M, DecodeError>;
}

/// A `Json` serializer.
///
/// It can serialize any message that satisfies the traits:
/// - `serde::Serialize`
/// - `serde::de::DeserializeOwned`
///
/// Refer to the [`Bincode`] docs for the difference between
/// `serde::de::DeserializeOwned` and `serde::Deserialize<'de>`.
// #[cfg(feature = "json_serializer")]
// #[cfg_attr(docsrs, doc(cfg(feature = "json_serializer")))]
#[derive(Debug, Hash)]
pub struct Json {}

// #[cfg(feature = "json_serializer")]
// #[cfg_attr(docsrs, doc(cfg(feature = "json_serializer")))]
impl<M> Serializer<M> for Json
where
    M: serde::Serialize + serde::de::DeserializeOwned,
{
    fn encode(message: &M) -> Result<Vec<u8>, EncodeError> {
        Ok(serde_json::to_vec(message)?)
    }

    fn decode<R: Read>(reader: R) -> Result<M, DecodeError> {
        Ok(serde_json::from_reader(reader)?)
    }
}
