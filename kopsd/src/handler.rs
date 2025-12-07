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
use anyhow::Context;

use chrono::{TimeZone, Utc};
use k8s_openapi::api::core::v1::Pod;
use kops_protocol::{
    EnvEntry, EnvRequest, LoginRequest, PodSummary, PodsRequest, Request,
    Response,
};
use kube::ResourceExt;
use tracing::info;

use crate::state::{AwsSession, DaemonState};

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
            Request::Login(login_req) => self.handle_login(login_req).await,
            Request::Version => self.handle_version().await,
            Request::Pods(p) => self.handle_pods(p).await,
            Request::Env(r) => self.handle_env(r).await,
        }
    }

    async fn handle_login(&self, req: LoginRequest) -> Response {
        info!(
            "received AWS login for profile '{}' (account {} role {})",
            req.name, req.account_id, req.role_name
        );

        let expires_at = Utc
            .timestamp_millis_opt(req.expires_at_epoch_ms)
            .single()
            .unwrap_or_else(|| Utc::now());

        let session = AwsSession {
            account_id: req.account_id,
            role_name: req.role_name,
            region: req.region.clone(),
            access_key_id: req.access_key_id,
            secret_access_key: req.secret_access_key,
            session_token: req.session_token,
            expires_at,
        };

        {
            let mut map = match self.state.aws_sessions.lock() {
                Ok(m) => m,
                Err(_) => {
                    return Response::Error {
                        message: "failed to lock aws_sessions map".into(),
                    };
                }
            };

            map.insert(req.name.clone(), session);
            info!("stored AWS session for profile '{}'", req.name);
        }

        if let Err(err) = self.start_clusters_for_profile(&req.name).await {
            return Response::Error {
                message: format!(
                    "stored session but failed to start clusters for profile {}: {err}",
                    req.name
                ),
            };
        }

        Response::LoginOk
    }

    async fn start_clusters_for_profile(
        &self,
        profile: &str,
    ) -> anyhow::Result<()> {
        let session = {
            let map = self
                .state
                .aws_sessions
                .lock()
                .unwrap();
                // .context("failed to lock aws_sessions map")?;

            map.get(profile)
                .cloned()
                .context("no aws session stored for this profile")?
        };

        // for (name, cfg) in &self.state.clusters {
            // if cfg.session_name != profile {
            //     continue;
            // }

            // // Se cluster já está rodando, não faz nada
            // if self.state.clusters.contains_key(name) {
            //     continue;
            // }

        let name = String::from("eks-platform-dev");
            tracing::info!(
                "starting cluster worker for cluster '{}' (profile '{}')",
                name,
                profile
            );

            let sdk_config = sdk_config_from_session(&session).await?;

            let client = kops_aws_eks::create_kube_client(&sdk_config, &name)
                .await
                .with_context(|| format!("failed to create kube client for cluster {}", name))?;

            let cluster_state = crate::kube_worker::init_cluster_state(name.clone(), client)
                .await
                .with_context(|| format!("failed to start worker for cluster {}", name))?;

            self.state
    .clusters
    .lock()
    .unwrap()
    .insert(name.clone(), cluster_state);

        // }

        Ok(())
    }



    async fn handle_env(&self, req: EnvRequest) -> Response {
        let cluster = req
            .cluster
            .as_deref()
            .unwrap_or_else(|| self.state.default_cluster());

        let clusters = self.state.clusters.lock().unwrap();
        let Some(cs) = clusters.get(cluster) else {
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

        let clusters = self.state.clusters.lock().unwrap();
        let Some(cluster_state) = clusters.get(cluster_name) else {
            return Response::Error {
                message: format!("cluster not found: {cluster_name}"),
            };
        };

        // let mut pods: Vec<PodSummary> = Vec::new();
        let pods_snapshot = cluster_state.store().state();
        // for pod in pods_snapshot {
        //     if let Some(summary) = PodSummary::from_pod(cluster_name, &pod) {
        //         pods.push(summary);
        //     }
        // }

        // // let map = cluster_state.pods.read().await;
        // let map = cluster_state.store().state();

        let mut pods: Vec<PodSummary> = pods_snapshot
            .into_iter()
            .filter_map(|p| PodSummary::from_pod(cluster_name, &p))
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

use aws_config::{Region, SdkConfig};
use aws_credential_types::{provider::SharedCredentialsProvider, Credentials};

pub async fn sdk_config_from_session(
    session: &AwsSession,
) -> anyhow::Result<SdkConfig> {
    // 1. Cria objeto Credentials a partir da sessão
    let creds = Credentials::new(
        session.access_key_id.clone(),
        session.secret_access_key.clone(),
        Some(session.session_token.clone()),
        Some(session.expires_at.into()),
        "kops-sso-session-dev",
    );

    let creds_provider = SharedCredentialsProvider::new(creds);

    // 2. Resolve região
    let region = session
        .region
        .clone()
        .unwrap_or_else(|| "us-east-1".to_string());

    let region = Region::new(region);

    // 3. Monta o SdkConfig manualmente
    let sdk_config = aws_config::from_env()
        .region(region)
        .credentials_provider(creds_provider)
        .load()
        .await;

    Ok(sdk_config)
}
