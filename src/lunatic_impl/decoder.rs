use std::fmt;
use std::io::Read;
use std::{convert::TryFrom, str::FromStr};

use flate2::read::{GzDecoder, ZlibDecoder};

use http::{
    header::{CONTENT_ENCODING, CONTENT_LENGTH, TRANSFER_ENCODING},
    HeaderMap, Method,
};

use httparse::{Status, EMPTY_HEADER};
use serde::{Deserialize, Serialize};

use super::http_stream::HttpStream;
use super::request::InnerRequest;
use super::InnerClient;
use crate::HttpResponse;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Accepts {
    pub(super) gzip: bool,
    pub(super) brotli: bool,
    pub(super) deflate: bool,
}

/// A response decompressor over a non-blocking stream of chunks.
///
/// The inner decoder may be constructed asynchronously.
pub(crate) struct Decoder {
    encoding: MessageEncoding,
    reader: HttpBodyReader,
}

#[derive(Debug)]
enum MessageEncoding {
    Gzip,
    Brotli,
    Deflate,
    Octets,
}

impl fmt::Debug for Decoder {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Decoder").finish()
    }
}

impl Decoder {
    /// A plain text decoder.
    ///
    /// This decoder will emit the underlying chunks as-is.
    fn plain_text(reader: HttpBodyReader) -> Decoder {
        Decoder {
            reader,
            encoding: MessageEncoding::Octets,
        }
    }

    /// A gzip decoder.
    ///
    /// This decoder will buffer and decompress chunks that are gzipped.
    fn gzip(reader: HttpBodyReader) -> Decoder {
        Decoder {
            reader,
            encoding: MessageEncoding::Gzip,
        }
    }

    /// A brotli decoder.
    ///
    /// This decoder will buffer and decompress chunks that are brotlied.
    fn brotli(reader: HttpBodyReader) -> Decoder {
        Decoder {
            reader,
            encoding: MessageEncoding::Brotli,
        }
    }

    /// A deflate decoder.
    ///
    /// This decoder will buffer and decompress chunks that are deflated.
    fn deflate(reader: HttpBodyReader) -> Decoder {
        Decoder {
            reader,
            encoding: MessageEncoding::Deflate,
        }
    }

    pub fn decode(&mut self) -> HttpResponse {
        if let MessageEncoding::Octets = self.encoding {
            let reader = &mut self.reader;
            let body = if let Some(content_length) = reader.content_length() {
                let mut body = vec![0u8; content_length];
                reader.read_exact(&mut body).unwrap();
                body
            } else if reader.no_content_length_required() {
                vec![]
            } else {
                // this should not happen
                panic!("Content-encoded body without content-length");
            };
            return HttpResponse {
                headers: reader.res.headers().to_owned(),
                status: reader.res.status().to_owned(),
                // transform type into http::Version type
                version: reader.res.version().into(),
                body,
                url: reader.req.url.clone(),
            };
        }

        let buf = if !self.reader.no_content_length_required() {
            match &self.encoding {
                MessageEncoding::Brotli => {
                    let mut decoder = brotli::Decompressor::new(&mut self.reader, 4096);
                    let mut buf = Vec::new();
                    let _ = decoder.read_to_end(&mut buf).unwrap();
                    buf
                }
                MessageEncoding::Gzip => {
                    let mut decoder = GzDecoder::new(&mut self.reader);
                    let mut buf = Vec::new();
                    let _ = decoder.read_to_end(&mut buf).unwrap();
                    // end_buf
                    buf
                }
                MessageEncoding::Deflate => {
                    let mut decoder = ZlibDecoder::new(&mut self.reader);
                    let mut buf = Vec::new();
                    let _ = decoder.read_to_end(&mut buf).unwrap();
                    buf
                }
                _ => panic!("Cannot happen"),
            }
        } else {
            vec![]
        };
        HttpResponse {
            headers: self.reader.res.headers().to_owned(),
            status: self.reader.res.status().to_owned(),
            version: self.reader.res.version().into(),
            body: buf,
            url: self.reader.req.url.clone(),
        }
    }

    fn detect_encoding(headers: &mut HeaderMap, encoding_str: &str) -> bool {
        let mut is_content_encoded = {
            headers
                .get_all(CONTENT_ENCODING)
                .iter()
                .any(|enc| enc == encoding_str)
                || headers
                    .get_all(TRANSFER_ENCODING)
                    .iter()
                    .any(|enc| enc == encoding_str)
        };
        if is_content_encoded {
            if let Some(content_length) = headers.get(CONTENT_LENGTH) {
                if content_length == "0" {
                    lunatic_log::warn!("{} response with content-length of 0", encoding_str);
                    is_content_encoded = false;
                }
            }
        }
        // if is_content_encoded {
        //     headers.remove(CONTENT_ENCODING);
        //     headers.remove(CONTENT_LENGTH);
        // }
        is_content_encoded
    }

    /// Constructs a Decoder from a partial http response.
    ///
    /// A decoder is just a wrapper around the hyper request that knows
    /// how to decode the content body of the request.
    ///
    /// Uses the correct variant by inspecting the Content-Encoding header.
    pub(super) fn detect(mut reader: HttpBodyReader, _accepts: Accepts) -> Decoder {
        let _headers = reader.res.headers_mut();
        if _accepts.gzip && Decoder::detect_encoding(_headers, "gzip") {
            return Decoder::gzip(reader);
        }
        if _accepts.brotli && Decoder::detect_encoding(_headers, "br") {
            return Decoder::brotli(reader);
        }
        if _accepts.deflate && Decoder::detect_encoding(_headers, "deflate") {
            return Decoder::deflate(reader);
        }

        Decoder::plain_text(reader)
    }
}

const MAX_REQUEST_SIZE: usize = 10 * 1024 * 1024;
const REQUEST_BUFFER_SIZE: usize = 4096;
const MAX_HEADERS: usize = 128;

/// The result of parsing a response from a buffer.
type ResponseResult = Result<HttpResponse, ParseResponseError>;

#[derive(Debug)]
pub(crate) enum ParseResponseError {
    TcpStreamClosed,
    TcpStreamClosedWithoutData,
    HttpParseError(httparse::Error),
    ResponseTooLarge,
    UnknownCode,
}

pub(crate) fn parse_response(
    mut response_buffer: Vec<u8>,
    mut stream: HttpStream,
    req: InnerRequest,
    client: &mut InnerClient,
) -> ResponseResult {
    let mut buffer = [0_u8; REQUEST_BUFFER_SIZE];
    let mut headers = [EMPTY_HEADER; MAX_HEADERS];

    // Loop until at least one complete response is read.
    let (response_raw, offset) = loop {
        // In case of pipelined responses the `response_buffer` is going to come
        // prefilled with some data, and we should attempt to parse it into a response
        // before we decide to read more from `TcpStream`.
        let mut response_raw = httparse::Response::new(&mut headers);
        match response_raw.parse(&response_buffer) {
            Ok(state) => match state {
                Status::Complete(offset) => {
                    // Continue outside the loop.
                    break (response_raw, offset);
                }
                Status::Partial => {
                    // Read more data from TCP stream
                    let n = stream.read(&mut buffer);
                    if n.is_err() || *n.as_ref().unwrap() == 0 {
                        if response_buffer.is_empty() {
                            return Err(ParseResponseError::TcpStreamClosedWithoutData);
                        } else {
                            return Err(ParseResponseError::TcpStreamClosed);
                        }
                    }
                    let n = n.unwrap();
                    // Invalidate references in `headers` that could point to the previous
                    // `response_buffer` before extending it.
                    headers = [EMPTY_HEADER; MAX_HEADERS];
                    response_buffer.extend(&buffer[..n]);
                    // If response passed max size, abort
                    if response_buffer.len() > MAX_REQUEST_SIZE {
                        return Err(ParseResponseError::ResponseTooLarge);
                    }
                }
            },
            Err(err) => {
                return Err(ParseResponseError::HttpParseError(err));
            }
        }
    };

    lunatic_log::debug!("Received RAW Response {:?}", response_raw);

    // At this point one full response header is available, but the body (if it
    // exists) might not be fully loaded yet.

    let status_code = match http::StatusCode::try_from(response_raw.code.unwrap()) {
        Ok(code) => code,
        Err(_) => {
            return Err(ParseResponseError::UnknownCode);
        }
    };
    let response = http::Response::builder().status(status_code);
    let response = response_raw
        .headers
        .iter()
        .fold(response, |response, header| {
            response.header(header.name, header.value)
        });

    let reader = HttpBodyReader {
        stream,
        response_buffer,
        offset,
        body_offset: offset,
        res: response.body(vec![]).unwrap(),
        req,
        chunk_body: vec![],
        chunk_offset: 0,
    };
    Ok(Decoder::detect(reader, client.accepts()).decode())
}

pub struct HttpBodyReader {
    pub(crate) stream: HttpStream,
    // used to check headers, but has no body yet
    pub(crate) res: http::Response<Vec<u8>>,
    pub(crate) response_buffer: Vec<u8>,
    pub(crate) offset: usize,
    pub(crate) req: InnerRequest,
    pub(crate) body_offset: usize,
    pub(crate) chunk_body: Vec<u8>,
    pub(crate) chunk_offset: usize, // pub(crate) client: &'a mut Client,
}

impl HttpBodyReader {
    pub fn content_length(&self) -> Option<usize> {
        self.res
            .headers()
            .get(http::header::CONTENT_LENGTH)
            .map(|header| {
                let value_string = std::str::from_utf8(header.as_bytes()).unwrap();
                value_string.parse::<usize>().unwrap()
            })
    }

    pub fn transfer_encoding(&self) -> Vec<String> {
        self.res
            .headers()
            .get_all(http::header::TRANSFER_ENCODING)
            .iter()
            .filter_map(|header| {
                String::from_utf8(header.as_bytes().to_vec())
                    .map(Some)
                    .unwrap_or(None)
            })
            .collect()
    }

    pub fn is_chunked(&self) -> bool {
        self.transfer_encoding()
            .iter()
            .any(|e| e.as_str() == "chunked")
    }

    fn should_close_conn(&self) -> bool {
        let connection_header = self.res.headers().get("connection");
        connection_header.is_none() || connection_header.unwrap().as_bytes() == "close".as_bytes()
    }

    pub fn no_content_length_required(&self) -> bool {
        let method = Method::from_str(&self.req.method).unwrap();
        let status = self.res.status();
        let status_num = status.as_u16();
        method == http::Method::HEAD
            || (method == http::Method::GET && self.should_close_conn() && !self.is_chunked())
            || status == http::StatusCode::NO_CONTENT
            || status == http::StatusCode::NOT_MODIFIED
            || (100..200).contains(&status_num)
    }

    // simply load a bit more data from the underlying stream
    // because the parser is probably missing some data from the buffer
    fn load_more(&mut self) -> std::io::Result<usize> {
        // start reading from tcp stream
        let mut next_batch = vec![0u8; 1000];
        let read_size = self.stream.read(&mut next_batch)?;
        self.response_buffer
            .extend(next_batch[..read_size].to_vec());
        Ok(read_size)
    }

    fn inner_read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        // if response buffer doesn't have all the data
        // try to read more from the stream
        if self.offset >= self.response_buffer.len() {
            lunatic_log::debug!(
                "Fetching data from tcp stream. Buffer exceeded at {}",
                self.offset
            );
            // start reading from tcp stream
            let mut next_batch = vec![0u8; buf.len()];
            let read_size = self.stream.read(&mut next_batch)?;
            self.response_buffer
                .extend(next_batch[..read_size].to_vec());
        }
        let mut len_read = 0;
        for (idx, byte) in self.response_buffer[self.offset..].iter().enumerate() {
            if idx >= buf.len() {
                break;
            }
            len_read += 1;
            buf[idx] = *byte;
        }
        self.offset += len_read;
        Ok(len_read)
    }

    fn read_chunk(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        lunatic_log::debug!(
            "Reading from chunked body: chunk_offset {} | len {}",
            self.chunk_offset,
            self.chunk_body.len(),
        );
        if self.chunk_offset >= self.chunk_body.len() {
            // start reading from tcp stream
            return Ok(0);
        }
        let mut len_read = 0;
        for (idx, byte) in self.chunk_body[self.chunk_offset..].iter().enumerate() {
            if idx >= buf.len() {
                break;
            }
            len_read += 1;
            buf[idx] = *byte;
        }
        self.chunk_offset += len_read;
        Ok(len_read)
    }

    fn skip_clrf(&mut self) -> std::io::Result<()> {
        // if the clrf of the chunk is not yet in the response
        // buffer we need to load the data first
        if self.response_buffer.len() - self.offset < 2 {
            let mut clrf = [0u8; REQUEST_BUFFER_SIZE];
            let len = self.stream.read(&mut clrf)?;
            self.response_buffer.extend(clrf[..len].to_vec());
        }
        // in any case we need to "skip" the clrf tokens at the end of the chunk
        self.offset += 2;
        Ok(())
    }
}

impl Read for HttpBodyReader {
    // fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
    //     let mut read_buf = [0u8; REQUEST_BUFFER_SIZE];
    //     let mut read_size = 0;
    //     loop {
    //         match self.read(&mut read_buf) {
    //             Ok(num) => {
    //                 if num == 0 {
    //                     return Ok(read_size);
    //                 }
    //                 read_size += num;
    //                 buf.extend(read_buf[..num].to_vec());
    //             }
    //             Err(e) =>
    //         }
    //     }
    // }

    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.is_chunked() {
            // if we're done with reading the chunked data
            // we just read from the `self.chunk_body` buffer
            if !self.chunk_body.is_empty() {
                return self.read_chunk(buf);
            }
            // if data is transfered as chunked we want to consume it into `self.response_buffer()`
            // this means that we need to pre-read the data because
            loop {
                let chunk = httparse::parse_chunk_size(&self.response_buffer[self.offset..]);
                match chunk {
                    // idx is the offset at which the content begins
                    // so there's the size as well as CRLF
                    // this means that the next chunk of size `size` starts from `idx`
                    Ok(Status::Complete((idx, size))) => {
                        // first, we need to consume the size and CRLF bytes
                        if size == 0
                            && self.response_buffer[self.offset + idx..].starts_with(b"\r\n")
                        {
                            // break because the reading has been completed and
                            // the whole response body has been written into `self.chunk_body`
                            self.skip_clrf()?;
                            return self.read_chunk(buf);
                        }

                        self.offset += idx;

                        // now that we skipped the prefix we can consume the rest from
                        // either the `response_buffer` or the `stream` in case the response_buffer
                        // does not yet have all the data
                        let mut read_from_chunk = 0;

                        // not done reading current chunk from tcp/tls stream
                        //need to finish reading by moving data from response_buffer
                        // into `self.chunk_body`
                        while read_from_chunk < size as usize {
                            let mut chunk = vec![0u8; size as usize - read_from_chunk];
                            let read_chunk_size = self.inner_read(&mut chunk)?;
                            read_from_chunk += read_chunk_size;
                            self.chunk_body.extend(chunk[..read_chunk_size].to_vec());
                        }
                        // skip CRLF tokens
                        self.skip_clrf()?;
                    }
                    // partial in this context means that the chunk header
                    // was not fully read, meaning that we need to attempt to read
                    // from the tcp/tls stream in order to get the rest of the chunk header
                    Ok(Status::Partial) => {
                        let _size = self.load_more()?;
                    }
                    Err(_) => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Other,
                            "decoder::HttpBodyReader::read InvalidChunkSize",
                        ));
                    }
                }
            }
        }

        if let Some(len) = self.content_length() {
            if self.offset - self.body_offset >= len {
                return Ok(0);
            }
        }
        self.inner_read(buf)
    }
}

// ===== impl Accepts =====

impl Accepts {
    // pub(super) fn none() -> Self {
    //     Accepts {
    //         gzip: false,
    //         brotli: false,
    //         deflate: false,
    //     }
    // }

    pub(super) fn as_str(&self) -> Option<&'static str> {
        match (self.is_gzip(), self.is_brotli(), self.is_deflate()) {
            (true, true, true) => Some("gzip, br, deflate"),
            (true, true, false) => Some("gzip, br"),
            (true, false, true) => Some("gzip, deflate"),
            (false, true, true) => Some("br, deflate"),
            (true, false, false) => Some("gzip"),
            (false, true, false) => Some("br"),
            (false, false, true) => Some("deflate"),
            (false, false, false) => None,
        }
    }

    fn is_gzip(&self) -> bool {
        self.gzip
    }

    fn is_brotli(&self) -> bool {
        self.brotli
    }

    fn is_deflate(&self) -> bool {
        self.deflate
    }
}

impl Default for Accepts {
    fn default() -> Accepts {
        Accepts {
            gzip: true,
            brotli: true,
            deflate: true,
        }
    }
}
