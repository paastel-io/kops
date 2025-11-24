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

// src/state.rs

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub type ClusterName = String;

/// Global daemon state.
pub struct DaemonState {
    pub clusters: HashMap<ClusterName, Arc<ClusterState>>,
    pub default_cluster: ClusterName,
}

impl DaemonState {
    pub fn default_cluster(&self) -> &str {
        &self.default_cluster
    }
}

pub struct ClusterState {
    /// In-memory cache of pods for this cluster.
    pub pods: RwLock<HashMap<PodKey, PodSummary>>,
}

#[derive(Clone, Debug, Hash, Eq, PartialEq)]
pub struct PodKey {
    pub cluster: String,
    pub namespace: String,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct PodSummary {
    pub cluster: String,
    pub namespace: String,
    pub name: String,
    pub phase: Option<String>,
    pub reason: Option<String>,
    pub message: Option<String>,
    pub ready: bool,
    pub restart_count: i32,
}

impl PodSummary {
    pub fn from_pod(
        cluster: &str,
        pod: &k8s_openapi::api::core::v1::Pod,
    ) -> Option<Self> {
        let meta = pod.metadata.clone();
        let status = pod.status.clone();

        let name = meta.name?;
        let namespace =
            meta.namespace.unwrap_or_else(|| "default".to_string());

        let phase = status.as_ref().and_then(|s| s.phase.clone());
        let (reason, message, ready, restart_count) =
            extract_status_fields(status.as_ref());

        Some(PodSummary {
            cluster: cluster.to_string(),
            namespace,
            name,
            phase,
            reason,
            message,
            ready,
            restart_count,
        })
    }
}

fn extract_status_fields(
    status: Option<&k8s_openapi::api::core::v1::PodStatus>,
) -> (Option<String>, Option<String>, bool, i32) {
    let mut ready = false;
    let mut restarts = 0;
    let mut reason = None;
    let mut message = None;

    if let Some(s) = status {
        if let Some(conditions) = &s.conditions {
            ready = conditions
                .iter()
                .any(|c| c.type_ == "Ready" && c.status == "True");
        }

        if let Some(cs) = &s.container_statuses {
            for c in cs {
                restarts += c.restart_count as i32;
                if let Some(state) = &c.state {
                    if let Some(w) = &state.waiting {
                        reason = w.reason.clone();
                        message = w.message.clone();
                    }
                    if let Some(t) = &state.terminated {
                        reason = t.reason.clone();
                        message = t.message.clone();
                    }
                }
            }
        }
    }

    (reason, message, ready, restarts)
}
