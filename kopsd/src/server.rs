#![allow(unused_imports)]
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

use std::{
    collections::HashMap,
    os::unix::fs::PermissionsExt,
    sync::{Arc, Mutex},
};

use anyhow::{Context, Result};
use daemonize::Daemonize;
use tokio::{
    fs::remove_file,
    net::{UnixListener, UnixStream},
    signal, task,
};
use tracing::{debug, error, info, warn};

use kops_protocol::{
    Request,
    wire::{read_message, write_message},
};

use crate::{
    config::{self, KopsdConfig},
    handler::Handler,
    // kube_worker::start_cluster_worker,
    kube_worker::init_cluster_state,
    state::{ClusterState, DaemonState},
};

const SOCKET_PATH: &str = "/var/run/kopsd/kopsd.sock";

pub(crate) fn run(args: &crate::Args) -> Result<()> {
    kops_log::init(args.verbose);

    let config = config::load()?;

    if args.daemon {
        run_fg(&config)?;
    }
    // } else {
    //     run_bg(&config, handler)?;
    // }

    Ok(())
}

fn run_fg(config: &KopsdConfig) -> Result<()> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    rt.block_on(async move {
        // for c in &config.cluster {
        //     let cs = init_cluster_state(c.clone()).await.unwrap();
        //     // clusters_map.insert(c.name.clone(), cs);
        // }

        let default_cluster = config
            .kops
            .default_cluster
            .clone()
            .unwrap_or_else(|| config.cluster[0].name.clone());

        // let state =
        //     Arc::new(DaemonState { clusters: clusters_map, default_cluster });
        let state = Arc::new(DaemonState {
            clusters: HashMap::new(),
            default_cluster,
            aws_sessions: Mutex::new(HashMap::new()),
        });

        // for c in config.cluster.clone() {
        //     let cluster_name = c.name.clone();
        //     let cluster_state = state
        //         .clusters
        //         .get(&cluster_name)
        //         .cloned()
        //         .expect("cluster state must exist");

        //     task::spawn(async move {
        //         if let Err(err) =
        //             start_cluster_worker(c, cluster_state, cluster_name.clone())
        //                 .await
        //         {
        //             error!(cluster = %cluster_name, "cluster worker failed: {err:?}");
        //         }
        //     });
        // }

        let handler = Arc::new(Handler::new(state.clone()));

        _run(config, handler).await
    })
}

fn run_bg(config: &KopsdConfig, handler: Arc<Handler>) -> Result<()> {
    let daemon_cfg = config.daemon.clone().unwrap_or_default();

    let stdout = if let Some(ref path) = daemon_cfg.stdout {
        Some(std::fs::File::create(path).with_context(|| {
            format!("failed to create daemon stdout file at {path}")
        })?)
    } else {
        None
    };

    let stderr = if let Some(ref path) = daemon_cfg.stderr {
        Some(std::fs::File::create(path).with_context(|| {
            format!("failed to create daemon stderr file at {path}")
        })?)
    } else {
        None
    };

    let mut daemon = Daemonize::new();

    if let Some(ref user) = daemon_cfg.user {
        daemon = daemon.user(user.as_str());
    }

    if let Some(ref group) = daemon_cfg.group {
        daemon = daemon.group(group.as_str());
    }

    if let Some(ref pid_file) = daemon_cfg.pid_file {
        daemon = daemon.pid_file(pid_file).chown_pid_file(true);
    }

    if let Some(stdout) = stdout {
        daemon = daemon.stdout(stdout);
    }

    if let Some(stderr) = stderr {
        daemon = daemon.stderr(stderr);
    }

    // Fork and detach
    daemon.start().context("failed to daemonize kopsd process")?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build tokio runtime")?;

    rt.block_on(async move { _run(config, handler).await })
}

async fn _run(_config: &KopsdConfig, handler: Arc<Handler>) -> Result<()> {
    info!("starting kopsd");

    // try to remove a stale socket if it exists
    let _ = remove_file(SOCKET_PATH).await;

    let listener = UnixListener::bind(SOCKET_PATH).with_context(|| {
        format!("failed to create socket path {SOCKET_PATH}")
    })?;
    info!("listening on unix socket {}", SOCKET_PATH);

    if let Err(e) = std::fs::set_permissions(
        SOCKET_PATH,
        std::fs::Permissions::from_mode(0o660),
    ) {
        error!("failed to set socket permissions: {e:?}");
    }

    loop {
        tokio::select! {
            res = listener.accept() => {
                match res {
                    Ok((stream, _addr)) => {
                        let handler = handler.clone();
                        debug!("new client connection");
                        tokio::spawn(async move {
                            if let Err(e) = handle_client(stream, handler).await {
                                error!("client handler error: {e:?}");
                            }
                        });
                    }
                    Err(e) => {
                        error!("failed to accept connection: {e:?}");
                    }
                }
            }

            // handle ctrl+c sigint
            _ = signal::ctrl_c() => {
                warn!("CTRL+C received, shutting down gracefully...");
                break;
            }
        }
    }

    // Dropping the listener closes the socket
    drop(listener);

    if let Err(e) = remove_file(SOCKET_PATH).await {
        if e.kind() != std::io::ErrorKind::NotFound {
            error!("failed to remove socket file on shutdown: {e:?}");
        }
    } else {
        info!("removed socket file {}", SOCKET_PATH);
    }

    info!("kopsd server stopped");

    Ok(())
}

/// Handle a single client connection
///
/// Read `kops_protocol::Request` and write `kops_protocol::Response`.
async fn handle_client(
    mut stream: UnixStream,
    handler: Arc<Handler>,
) -> Result<()> {
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

        let resp = handler.handle(req).await;

        if let Err(e) = write_message(&mut stream, &resp).await {
            error!("failed to write response: {e:?}");
            break;
        }
    }

    Ok(())
}
