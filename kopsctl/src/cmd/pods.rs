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

use anyhow::{Result, bail};

use kops_protocol::{PodSummary, PodsRequest, Request, Response};

use crate::helper::send_request;

pub async fn execute(
    cluster: Option<String>,
    namespace: Option<String>,
    failed_only: bool,
) -> Result<()> {
    let req = PodsRequest { cluster, namespace, failed_only };
    let resp = send_request(Request::Pods(req)).await?;

    match resp {
        Response::Pods { pods } => print_pods(&pods, failed_only),
        Response::Error { message } => bail!("reponse error {message}"),
        _ => bail!("unexpected response to version"),
    }

    Ok(())
}

fn print_pods(pods: &Vec<PodSummary>, failed_only: bool) {
    println!(
        "{:<20} {:<20} {:<30} {:<10} {:<10}",
        "CLUSTER", "NAMESPACE", "NAME", "READY", "RESTARTS"
    );

    for p in pods {
        if failed_only {
            if let Some(msg) = &p.message {
                println!(
                    "{:<20} {:<20} {:<30} {:<10} {:<10} {:<10}",
                    p.cluster,
                    p.namespace,
                    p.name,
                    p.ready,
                    p.restart_count,
                    msg
                );
            } else {
                println!(
                    "{:<20} {:<20} {:<30} {:<10} {:<10}",
                    p.cluster, p.namespace, p.name, p.ready, p.restart_count,
                );
            }
        } else {
            println!(
                "{:<20} {:<20} {:<30} {:<10} {:<10}",
                p.cluster, p.namespace, p.name, p.ready, p.restart_count
            );
        }
    }
}
