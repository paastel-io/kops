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

use k8s_openapi::api::core::v1::Pod;
use kops_protocol::{
    EnvEntry, EnvRequest, PodSummary, PodsRequest, Request, Response,
};
use kube::ResourceExt;

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
            Request::Env(r) => self.handle_env(r).await,
        }
    }

    async fn handle_env(&self, req: EnvRequest) -> Response {
        let cluster = req
            .cluster
            .as_deref()
            .unwrap_or_else(|| self.state.default_cluster());

        let Some(cs) = self.state.clusters.get(cluster) else {
            return Response::Error {
                message: format!("cluster not found: {cluster}"),
            };
        };

        // snapshot atual do cluster
        let pods = cs.store().state();

        // encontrar o pod
        let pod: Option<Arc<Pod>> = pods
            .iter()
            .find(|p| {
                p.namespace().as_deref() == Some(&req.namespace)
                    && p.name_any() == req.pod
            })
            .cloned();

        let Some(pod) = pod else {
            return Response::Error {
                message: format!(
                    "pod {}/{} not found",
                    req.namespace, req.pod
                ),
            };
        };

        // selecionar container
        let spec = match &pod.spec {
            Some(s) => s,
            None => {
                return Response::Error { message: "pod has no spec".into() };
            }
        };

        let mut vars: Vec<EnvEntry> = Vec::new();

        // let container_name = req.container.clone().unwrap_or_else(|| {
        //     spec.containers[0].name.clone() // default: first container
        // });

        for container in spec.containers.clone() {
            let container_vars: Vec<EnvEntry> = container
                .env
                .clone()
                .unwrap_or_default()
                .into_iter()
                .filter(|_e| {
                    // if let Some(re) = &regex { re.is_match(&e.name) } else { true }
                    true
                })
                .map(|e| EnvEntry { name: e.name, value: e.value })
                .collect();
            vars.extend(container_vars);
        }

        vars.sort();

        // let container =
        //     match spec.containers.iter().find(|c| c.name == container_name) {
        //         Some(c) => c,
        //         None => {
        //             return Response::Error {
        //                 message: format!(
        //                     "container '{}' not found in pod {}",
        //                     container_name, req.pod
        //                 ),
        //             };
        //         }
        //     };

        // filtrar vars
        // let regex = match req.filter_regex {
        //     Some(r) => Some(match Regex::new(&r) {
        //         Ok(re) => re,
        //         Err(err) => {
        //             return Response::Error {
        //                 message: format!("invalid regex: {err}"),
        //             };
        //         }
        //     }),
        //     None => None,
        // };

        // let vars: Vec<EnvEntry> = container
        //     .env
        //     .clone()
        //     .unwrap_or_default()
        //     .into_iter()
        //     .filter(|_e| {
        //         // if let Some(re) = &regex { re.is_match(&e.name) } else { true }
        //         true
        //     })
        //     .map(|e| EnvEntry { name: e.name, value: e.value })
        //     .collect();

        Response::EnvVars { vars }
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

        let mut pods: Vec<PodSummary> = Vec::new();
        let pods_snapshot = cluster_state.store().state();
        for pod in pods_snapshot {
            if let Some(summary) = PodSummary::from_pod(cluster_name, &pod) {
                pods.push(summary);
            }
        }

        // // let map = cluster_state.pods.read().await;
        // let map = cluster_state.store().state();

        // let mut pods: Vec<_> = map
        //     .values()
        //     .cloned()
        //     .filter(|p| {
        //         if let Some(ns) = &req.namespace {
        //             if &p.namespace != ns {
        //                 return false;
        //             }
        //         }
        //         if req.failed_only {
        //             if p.phase.as_deref() != Some("Failed")
        //                 && p.reason.as_deref() != Some("CrashLoopBackOff")
        //             {
        //                 return false;
        //             }
        //         }
        //         true
        //     })
        //     .collect();

        // pods.sort_by(|a, b| {
        //     a.namespace.cmp(&b.namespace).then(a.name.cmp(&b.name))
        // });

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
