//
// Copyright (c) 2025 murilo ijanc <murilo@ijanc.org>
//
// Permission to use, copy, modify, and distribute this software for any
// purpose with or without fee is hereby granted, provided that the above
// copyright notice and this permission notice appear in all copies.
//
// THE SOFTWARE IS PROVIDED "AS IS" AND THE AUTHOR DISCLAIMS ALL WARRANTIES
// WITH REGARD TO THIS SOFTWARE INCLUDING ALL IMPLIED WARRANTIES OF
// MERCHANTABILITY AND FITNESS. IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR
// ANY SPECIAL, DIRECT, INDIRECT, OR CONSEQUENTIAL DAMAGES OR ANY DAMAGES
// WHATSOEVER RESULTING FROM LOSS OF USE, DATA OR PROFITS, WHETHER IN AN
// ACTION OF CONTRACT, NEGLIGENCE OR OTHER TORTIOUS ACTION, ARISING OUT OF
// OR IN CONNECTION WITH THE USE OR PERFORMANCE OF THIS SOFTWARE.
//

use std::{fmt, io};

use bincode::{Decode, Encode};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

/// Error type for framed bincode I/O on the wire.
#[derive(Debug)]
pub enum WireError {
    Io(io::Error),
    BinDecode(bincode::error::DecodeError),
    BinEncode(bincode::error::EncodeError),
}

impl fmt::Display for WireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WireError::Io(e) => write!(f, "I/O error: {e}"),
            WireError::BinDecode(e) => write!(f, "bincode decode error: {e}"),
            WireError::BinEncode(e) => write!(f, "bincode encode error: {e}"),
        }
    }
}

impl std::error::Error for WireError {}

impl From<io::Error> for WireError {
    fn from(e: io::Error) -> Self {
        WireError::Io(e)
    }
}

impl From<bincode::error::DecodeError> for WireError {
    fn from(e: bincode::error::DecodeError) -> Self {
        WireError::BinDecode(e)
    }
}

impl From<bincode::error::EncodeError> for WireError {
    fn from(e: bincode::error::EncodeError) -> Self {
        WireError::BinEncode(e)
    }
}

/// Read a lenght-prefixed bincode message from the stream.
///
/// Returns Ok(None) if the client closed the connection cleanly.
pub async fn read_message<R, T>(reader: &mut R) -> Result<Option<T>, WireError>
where
    R: AsyncRead + Unpin,
    T: Decode<()>,
{
    let mut buf_sz = [0u8; 4];

    match reader.read_exact(&mut buf_sz).await {
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => {
            // connection closed without a new frame
            return Ok(None);
        }
        Err(e) => return Err(WireError::Io(e)),
    }

    let len = u32::from_be_bytes(buf_sz) as usize;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).await?;

    let config = bincode::config::standard();
    let (msg, _len): (T, usize) = bincode::decode_from_slice(&buf, config)?;

    Ok(Some(msg))
}

/// Write a length-prefixed bincode message to an async writer.
pub async fn write_message<W, T>(
    writer: &mut W,
    msg: &T,
) -> Result<(), WireError>
where
    W: AsyncWrite + Unpin,
    T: Encode,
{
    let config = bincode::config::standard();
    let encoded = bincode::encode_to_vec(msg, config)?;

    let len = encoded.len() as u32;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(&encoded).await?;
    writer.flush().await?;

    Ok(())
}
