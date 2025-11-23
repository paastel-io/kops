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
use clap::{ArgAction, Parser};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};
use tracing::{debug, warn};

const SOCKET_PATH: &str = "/tmp/kopsd.sock";

const VERSION: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (",
    env!("GIT_HASH", "unknown"),
    " ",
    env!("BUILD_DATE", "unknown"),
    ")",
);

#[derive(Debug, Parser)]
#[command(
    name = env!("CARGO_PKG_NAME"),
    about = "control the kops daemon",
    version = VERSION,
    author,
    propagate_version = true
)]
struct Args {
    /// Increase verbosity (use -v, -vv, ...).
    ///
    /// When no RUST_LOG is set, a single -v switches the log level to DEBUG.
    #[arg(short, long, global = true, action = ArgAction::Count)]
    verbose: u8,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    init_logger(args.verbose);

    ping().await?;

    Ok(())
}

async fn ping() -> Result<()> {
    debug!("connecting to kopsd at {}", SOCKET_PATH);

    let mut stream = UnixStream::connect(SOCKET_PATH).await?;

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

/// Initialize tracing based on RUST_LOG and the CLI verbosity.
///
/// Rules:
/// - If RUST_LOG is set, it is fully respected.
/// - If RUST_LOG is not set and verbose == 0 -> INFO level.
/// - If RUST_LOG is not set and verbose  > 0 -> DEBUG level.
fn init_logger(verbose: u8) {
    use tracing_subscriber::{
        EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt,
    };

    let stdout_layer =
        fmt::layer().without_time().with_writer(std::io::stdout);

    if std::env::var_os("RUST_LOG").is_some() {
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(stdout_layer)
            .init();
        return;
    }

    let filter = if verbose > 0 {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry().with(filter).with(stdout_layer).init();
}
