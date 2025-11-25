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

use std::sync::Arc;

use kops_protocol::{PodsRequest, Request, Response};

use crate::state::DaemonState;

pub struct Handler {
    state: Arc<DaemonState>,
}

impl Handler {
    pub fn new(state: Arc<DaemonState>) -> Self {
        Self { state }
    }

    pub async fn handle(&self, req: Request) -> Response {
        match req {
            Request::Ping => Response::Pong,
            Request::Version => self.handle_version().await,
            Request::Pods(p) => self.handle_pods(p).await,
        }
    }

    async fn handle_version(&self) -> Response {
        let daemon_version = env!("CARGO_PKG_VERSION").to_string();
        let protocol_version = "1".to_string();

        let git_sha = option_env!("GIT_HASH").map(|s| s.to_string());
        let build_date = option_env!("BUILD_DATE").map(|s| s.to_string());

        let info = kops_protocol::VersionInfo {
            daemon_version,
            protocol_version,
            git_sha,
            build_date,
        };

        Response::Version(info)
    }

    async fn handle_pods(&self, req: PodsRequest) -> Response {
        let cluster_name = req
            .cluster
            .as_deref()
            .unwrap_or_else(|| self.state.default_cluster());

        let Some(cluster_state) = self.state.clusters.get(cluster_name) else {
            return Response::Error {
                message: format!("cluster not found: {cluster_name}"),
            };
        };

        let map = cluster_state.pods.read().await;

        let mut pods: Vec<_> = map
            .values()
            .cloned()
            .filter(|p| {
                if let Some(ns) = &req.namespace {
                    if &p.namespace != ns {
                        return false;
                    }
                }
                if req.failed_only {
                    if p.phase.as_deref() != Some("Failed")
                        && p.reason.as_deref() != Some("CrashLoopBackOff")
                    {
                        return false;
                    }
                }
                true
            })
            .collect();

        pods.sort_by(|a, b| {
            a.namespace.cmp(&b.namespace).then(a.name.cmp(&b.name))
        });

        Response::Pods { pods }
    }

    // async fn handle_reset(&self, cluster: Option<String>) -> Response {
    //     todo!()
    //     // if let Some(name) = cluster {
    //     //     if let Some(c) = self.state.clusters.get(&name) {
    //     //         c.pods.write().await.clear();
    //     //     } else {
    //     //         return Response::Error {
    //     //             message: format!("cluster not found: {name}"),
    //     //         };
    //     //     }
    //     // } else {
    //     //     for c in self.state.clusters.values() {
    //     //         c.pods.write().await.clear();
    //     //     }
    //     // }

    //     // Response::ResetOk
    // }
}
