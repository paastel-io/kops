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

use dialoguer::FuzzySelect;
use kops_protocol::{EnvEntry, EnvRequest, PodsRequest, Request, Response};

use crate::helper::send_request;

pub async fn execute(
    cluster: Option<String>,
    namespace: Option<String>,
    _pod: Option<String>,
    container: Option<String>,
    filter: Option<String>,
) -> Result<()> {
    let req = PodsRequest {
        cluster: cluster.clone(),
        namespace,
        failed_only: false,
    };
    let resp = send_request(Request::Pods(req)).await?;

    match resp {
        Response::Pods { pods } => {
            let mut items: Vec<String> = pods
                .iter()
                .map(|p| format!("{} / {}", p.namespace, p.name))
                .collect();

            items.sort();

            let selection = FuzzySelect::new()
                .with_prompt("Select pod")
                .items(&items)
                .interact()
                .unwrap();

            let item = &items[selection];
            let item: Vec<&str> = item.split('/').collect();

            if item.len() != 2 {
                bail!("unexpected response to version");
            };

            let resp = send_request(Request::Env(EnvRequest {
                cluster,
                namespace: item[0].trim().to_string(),
                pod: item[1].trim().to_string(),
                container,
                filter_regex: filter,
            }))
            .await?;

            match resp {
                Response::EnvVars { vars } => print_vars(&vars),
                Response::Error { message } => {
                    bail!("reponse error {message}")
                }
                _ => bail!("unexpected response to version"),
            }
        }
        _ => bail!("unexpected response to version"),
    };

    Ok(())
}

fn print_vars(vars: &Vec<EnvEntry>) {
    for v in vars {
        println!(
            "{} = {}",
            v.name,
            v.value.clone().unwrap_or("<none>".to_string())
        );
    }
}
