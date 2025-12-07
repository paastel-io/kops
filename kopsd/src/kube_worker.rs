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
use futures::StreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::runtime::reflector::store::Writer;
use kube::{
    Api, Client,
    config::{KubeConfigOptions, Kubeconfig},
};
use kube_runtime::{
    reflector::{self, Store},
    watcher,
};
use tokio::task;
use tracing::{info, warn};

use crate::config::ClusterConfig;
use crate::state::{ClusterName, ClusterState};

/// Initialize a ClusterState for a given cluster config and start
/// a background reflector task to keep the Store<Pod> up-to-date.
pub async fn init_cluster_state(
    cluster_name: ClusterName,
    client: kube::Client,
) -> Result<Arc<ClusterState>> {
    // let cluster_name: ClusterName = cfg.name.clone();

    // let client = build_client_for_cluster(&cfg).await?;

    let pods_api: Api<Pod> = Api::all(client);

    let (store, writer): (Store<Pod>, Writer<Pod>) = reflector::store();

    let watcher_cfg = watcher::Config::default();

    let rf = reflector::reflector(writer, watcher(pods_api, watcher_cfg));

    let state = Arc::new(ClusterState::new(cluster_name.clone(), store));

    task::spawn(async move {
        info!(cluster = %cluster_name, "starting pod reflector");

        // `for_each` consome o stream; não precisamos do valor em si,
        // o objetivo é só manter o Store sincronizado.
        rf.for_each(|event_result| {
            if let Err(err) = &event_result {
                warn!(cluster = %cluster_name, %err, "reflector event error");
            }
            futures::future::ready(())
        })
        .await;

        // if let Err(err) = fut.await {
        //     // Isso só acontece se o stream em si quebrar de forma grave
        //     warn!(cluster = %cluster_name, "reflector stream ended: {err:?}");
        // }

        info!(cluster = %cluster_name, "pod reflector finished");
    });

    Ok(state)
}

/// Build a Kubernetes client using kubeconfig + context from ClusterConfig.
///
/// If `kubeconfig` is None, it falls back to the default discovery:
///   - $KUBECONFIG
///   - in-cluster config
async fn build_client_for_cluster(cfg: &ClusterConfig) -> Result<Client> {
    if let Some(path) = &cfg.kubeconfig {
        // Usa kubeconfig explícito + context opcional
        let kubeconfig = Kubeconfig::read_from(path)?;
        let options = KubeConfigOptions {
            context: cfg.context.clone(),
            ..KubeConfigOptions::default()
        };
        let config =
            kube::Config::from_custom_kubeconfig(kubeconfig, &options).await?;
        Ok(Client::try_from(config)?)
    } else {
        // Usa a detecção padrão (KUBECONFIG, in-cluster, etc.)
        Ok(Client::try_default().await?)
    }
}

// use std::sync::Arc;

// use anyhow::Result;
// use futures::TryStreamExt;
// use futures::pin_mut;
// use k8s_openapi::api::core::v1::Pod;
// use kops_protocol::{PodKey, PodSummary};
// use kube::{
//     Api, Client, Config,
//     config::{KubeConfigOptions, Kubeconfig},
// };
// use kube_runtime::watcher::{self, Event};
// use tracing::info;

// use crate::config::ClusterConfig;
// use crate::state::ClusterState;

// pub async fn start_cluster_worker(
//     cfg: ClusterConfig,
//     state: Arc<ClusterState>,
//     cluster_name: String,
// ) -> Result<()> {
//     let kubeconfig = if let Some(path) = &cfg.kubeconfig {
//         let options = KubeConfigOptions {
//             context: cfg.context.clone(),
//             ..KubeConfigOptions::default()
//         };
//         let kc = Kubeconfig::read_from(path.clone())?;
//         Config::from_custom_kubeconfig(kc, &options).await?
//     } else {
//         Config::infer().await?
//     };

//     let client = Client::try_from(kubeconfig)?;

//     initial_pod_sync(&client, &state, &cluster_name).await?;
//     // watch_pods(&client, &state, &cluster_name).await?;

//     Ok(())
// }

// // async fn initial_pod_sync(
// //     client: &Client,
// //     state: &ClusterState,
// //     cluster_name: &str,
// // ) -> Result<()> {
// //     info!(cluster = cluster_name, "starting initial pod sync");

// //     let pods_api: Api<Pod> = Api::all(client.clone());
// //     let lp = kube::api::ListParams::default();
// //     let pod_list = pods_api.list(&lp).await?;

// //     let mut map = state.pods.write().await;
// //     map.clear();

// //     for p in pod_list.items {
// //         if let Some(summary) = PodSummary::from_pod(cluster_name, &p) {
// //             let key = PodKey {
// //                 cluster: cluster_name.to_string(),
// //                 namespace: summary.namespace.clone(),
// //                 name: summary.name.clone(),
// //             };
// //             map.insert(key, summary);
// //         }
// //     }

// //     info!(
// //         cluster = cluster_name,
// //         total = map.len(),
// //         "initial pod sync completed"
// //     );

// //     Ok(())
// // }

// // pub async fn watch_pods(
// //     client: &kube::Client,
// //     state: &ClusterState,
// //     cluster_name: &str,
// // ) -> Result<()> {
// //     let pods_api: Api<Pod> = Api::all(client.clone());

// //     let pod_watcher =
// //         kube::runtime::watcher(pods_api, watcher::Config::default());

// //     pin_mut!(pod_watcher);

// //     info!(cluster = cluster_name, "starting pod watcher");

// //     while let Some(event) = pod_watcher.try_next().await? {
// //         match event {
// //             Event::Apply(pod) | Event::InitApply(pod) => {
// //                 if let Some(summary) = PodSummary::from_pod(cluster_name, &pod)
// //                 {
// //                     let key = PodKey {
// //                         cluster: cluster_name.to_string(),
// //                         namespace: summary.namespace.clone(),
// //                         name: summary.name.clone(),
// //                     };
// //                     let mut map = state.pods.write().await;
// //                     map.insert(key, summary);
// //                 }
// //             }
// //             Event::Delete(pod) => {
// //                 if let Some(name) = pod.metadata.name {
// //                     let ns = pod
// //                         .metadata
// //                         .namespace
// //                         .unwrap_or_else(|| "default".to_string());
// //                     let key = PodKey {
// //                         cluster: cluster_name.to_string(),
// //                         namespace: ns,
// //                         name,
// //                     };
// //                     let mut map = state.pods.write().await;
// //                     map.remove(&key);
// //                 }
// //             }
// //             Event::Init => {
// //                 info!(cluster = cluster_name, "watcher init");
// //             }
// //             Event::InitDone => {
// //                 info!(cluster = cluster_name, "watcher init done");
// //             }
// //         }
// //     }

// //     Ok(())
// // }
