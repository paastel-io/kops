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

use anyhow::{Result, bail};
use tokio::net::UnixStream;
use tracing::{debug, error};

use kops_protocol::{
    Request, Response,
    wire::{read_message, write_message},
};

const SOCKET_PATH: &str = "/tmp/kopsd.sock";

pub async fn execute() -> Result<()> {
    let resp = send_request(Request::Ping).await?;

    if let Response::Error { message } = resp {
        error!("error from daemon: {message}");
    }
    debug!("received pong response");
    Ok(())
}

async fn send_request(req: Request) -> Result<Response> {
    debug!("connecting to kopsd at {}", SOCKET_PATH);
    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

    write_message(&mut stream, &req).await?;
    let resp: Response = match read_message(&mut stream).await? {
        Some(r) => r,
        None => bail!("daemon closed connection without reply"),
    };

    Ok(resp)
}
