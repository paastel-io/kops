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

/// Initialize tracing based on RUST_LOG and the CLI verbosity.
///
/// Rules:
/// - If RUST_LOG is set, it is fully respected.
/// - If RUST_LOG is not set and verbose == 0 -> INFO level.
/// - If RUST_LOG is not set and verbose  > 0 -> DEBUG level.
pub fn init(verbose: u8) {
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
        EnvFilter::new("kopsd=debug")
    } else {
        EnvFilter::new("kopsd=info")
    };

    tracing_subscriber::registry().with(filter).with(stdout_layer).init();
}
