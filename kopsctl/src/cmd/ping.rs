//
// Copyright (c) 2025 murilo ijanc' <murilo@ijanc.org>
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

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use tracing::{debug, warn};

const SOCKET_PATH: &str = "/tmp/kopsd.sock";

pub async fn execute() -> Result<()> {
    debug!("connecting to kopsd at {}", SOCKET_PATH);

    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    debug!("sending PING command");
    stream.write_all(b"PING\n").await?;
    stream.flush().await?;

    let mut buf = vec![0u8; 1024];
    let n = stream.read(&mut buf).await?;

    if n == 0 {
        warn!("no response from server");
        return Ok(());
    }

    let msg = String::from_utf8_lossy(&buf[..n]).trim().to_string();
    debug!("server replied: {}", msg);

    Ok(())
}
