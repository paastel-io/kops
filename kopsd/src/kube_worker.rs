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

use anyhow::Result;
use futures::TryStreamExt;
use futures::pin_mut;
use k8s_openapi::api::core::v1::Pod;
use kops_protocol::{PodKey, PodSummary};
use kube::{
    Api, Client, Config,
    config::{KubeConfigOptions, Kubeconfig},
};
use kube_runtime::watcher::{self, Event};
use tracing::info;

use crate::config::ClusterConfig;
use crate::state::ClusterState;

pub async fn start_cluster_worker(
    cfg: ClusterConfig,
    state: Arc<ClusterState>,
    cluster_name: String,
) -> Result<()> {
    let kubeconfig = if let Some(path) = &cfg.kubeconfig {
        let options = KubeConfigOptions {
            context: cfg.context.clone(),
            ..KubeConfigOptions::default()
        };
        let kc = Kubeconfig::read_from(path.clone())?;
        Config::from_custom_kubeconfig(kc, &options).await?
    } else {
        Config::infer().await?
    };

    let client = Client::try_from(kubeconfig)?;

    initial_pod_sync(&client, &state, &cluster_name).await?;
    // watch_pods(&client, &state, &cluster_name).await?;

    Ok(())
}

async fn initial_pod_sync(
    client: &Client,
    state: &ClusterState,
    cluster_name: &str,
) -> Result<()> {
    info!(cluster = cluster_name, "starting initial pod sync");

    let pods_api: Api<Pod> = Api::all(client.clone());
    let lp = kube::api::ListParams::default();
    let pod_list = pods_api.list(&lp).await?;

    let mut map = state.pods.write().await;
    map.clear();

    for p in pod_list.items {
        if let Some(summary) = PodSummary::from_pod(cluster_name, &p) {
            let key = PodKey {
                cluster: cluster_name.to_string(),
                namespace: summary.namespace.clone(),
                name: summary.name.clone(),
            };
            map.insert(key, summary);
        }
    }

    info!(
        cluster = cluster_name,
        total = map.len(),
        "initial pod sync completed"
    );

    Ok(())
}

pub async fn watch_pods(
    client: &kube::Client,
    state: &ClusterState,
    cluster_name: &str,
) -> Result<()> {
    let pods_api: Api<Pod> = Api::all(client.clone());

    let pod_watcher =
        kube::runtime::watcher(pods_api, watcher::Config::default());

    pin_mut!(pod_watcher);

    info!(cluster = cluster_name, "starting pod watcher");

    while let Some(event) = pod_watcher.try_next().await? {
        match event {
            Event::Apply(pod) | Event::InitApply(pod) => {
                if let Some(summary) = PodSummary::from_pod(cluster_name, &pod)
                {
                    let key = PodKey {
                        cluster: cluster_name.to_string(),
                        namespace: summary.namespace.clone(),
                        name: summary.name.clone(),
                    };
                    let mut map = state.pods.write().await;
                    map.insert(key, summary);
                }
            }
            Event::Delete(pod) => {
                if let Some(name) = pod.metadata.name {
                    let ns = pod
                        .metadata
                        .namespace
                        .unwrap_or_else(|| "default".to_string());
                    let key = PodKey {
                        cluster: cluster_name.to_string(),
                        namespace: ns,
                        name,
                    };
                    let mut map = state.pods.write().await;
                    map.remove(&key);
                }
            }
            Event::Init => {
                info!(cluster = cluster_name, "watcher init");
            }
            Event::InitDone => {
                info!(cluster = cluster_name, "watcher init done");
            }
        }
    }

    Ok(())
}
