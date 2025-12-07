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

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::reflector::Store;

/// AWS session stored in daemon memory.
#[derive(Clone)]
pub struct AwsSession {
    pub account_id: String,
    pub role_name: String,
    pub region: Option<String>,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: String,
    pub expires_at: DateTime<Utc>,
}

/// Logical name of the cluster (from config).
pub type ClusterName = String;
pub type ProfileName = String;

/// Global daemon state shared by handlers.
pub struct DaemonState {
    pub clusters: HashMap<ClusterName, Arc<ClusterState>>,
    pub default_cluster: ClusterName,

    /// AWS sessions keyed by logical profile name ("dev", "prod", ...).
    pub aws_sessions: Mutex<HashMap<ProfileName, AwsSession>>,
}

impl DaemonState {
    pub fn default_cluster(&self) -> &str {
        &self.default_cluster
    }

    #[allow(dead_code)]
    pub fn get_session(&self, name: &str) -> Option<AwsSession> {
        let sessions = self.aws_sessions.lock().ok()?;
        sessions.get(name).cloned()
    }
}

/// Per-cluster in-memory state backed by a reflector Store.
///
/// The Store is automatically kept up-to-date by the kube_worker
/// background task (reflector + watcher).
pub struct ClusterState {
    name: ClusterName,
    store: Store<Pod>,
}

impl ClusterState {
    /// Create a new ClusterState from a cluster name and a reflector Store.
    pub fn new(name: ClusterName, store: Store<Pod>) -> Self {
        Self { name, store }
    }

    /// Name of this cluster (as in config).
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Access the underlying Store for this cluster.
    ///
    /// You can call:
    ///   - `store.state()` para snapshot
    ///   - `store.get(ObjectRef)` para um Pod específico
    ///   - `store.len()`, etc.
    pub fn store(&self) -> &Store<Pod> {
        &self.store
    }
}
