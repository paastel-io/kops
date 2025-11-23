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

const SOCKET_PATH: &str = "/tmp/kopsd.sock";

use anyhow::Result;
use tokio::{
    fs::remove_file,
    net::{UnixListener, UnixStream},
};
use tracing::{debug, error, info};

use kops_protocol::{
    Request, Response,
    wire::{read_message, write_message},
};

pub(crate) async fn run() -> Result<()> {
    // try to remove a stale socket if it exists
    let _ = remove_file(SOCKET_PATH).await;

    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!("listening on unix socket {}", SOCKET_PATH);

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                debug!("new client connection");
                tokio::spawn(async move {
                    if let Err(e) = handle_client(stream).await {
                        error!("client handler error: {e:?}");
                    }
                });
            }
            Err(e) => {
                error!("failed to accept connection: {e:?}");
            }
        }
    }
}

/// Handle a single client connection
///
/// Read `kops_protocol::Request` and write `kops_protocol::Response`.
async fn handle_client(mut stream: UnixStream) -> Result<()> {
    loop {
        let req: Request = match read_message(&mut stream).await {
            Ok(Some(msg)) => msg,
            Ok(None) => {
                debug!("client closed connection");
                break;
            }
            Err(e) => {
                error!("failed to read message: {e:?}");
                break;
            }
        };

        debug!("received request: {:?}", req);

        let resp = match req {
            Request::Ping => Response::Pong,
        };

        if let Err(e) = write_message(&mut stream, &resp).await {
            error!("failed to write response: {e:?}");
            break;
        }
    }

    Ok(())
}
