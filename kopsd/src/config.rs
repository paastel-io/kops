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

use std::path::PathBuf;

use anyhow::Result;
use serde::Deserialize;
use tracing::debug;

#[derive(Debug, Deserialize, Default, Clone)]
pub struct KopsSection {
    pub default_cluster: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClusterConfig {
    pub name: String,
    pub kubeconfig: Option<PathBuf>,
    pub context: Option<String>,
    pub namespaces: Option<Vec<String>>,
}
#[derive(Debug, Deserialize, Default, Clone)]
pub struct DaemonConfig {
    pub pid_file: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub user: Option<String>,
    pub group: Option<String>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct KopsdConfig {
    pub kops: KopsSection,
    pub daemon: Option<DaemonConfig>,
    pub cluster: Vec<ClusterConfig>,
}

pub(crate) fn load() -> Result<KopsdConfig> {
    debug!("loading");
    let mut settings = config::Config::builder();

    settings = settings
        .add_source(config::File::with_name("config/kopsd").required(false))
        .add_source(config::Environment::with_prefix("KOPSD").separator("__"));

    let cfg = settings.build()?;

    Ok(cfg.try_deserialize()?)
}
