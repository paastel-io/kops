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

use std::time::{Duration, SystemTime};

use anyhow::{Context, Result, anyhow};
use aws_config::SdkConfig;
use aws_credential_types::provider::ProvideCredentials;
use aws_sdk_eks as eks;
use aws_sigv4::http_request::{
    SignableBody, SignableRequest, SignatureLocation, SigningSettings,
};
use aws_smithy_runtime_api::client::identity::Identity;
use base64::{
    Engine,
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
};
use rustls::crypto::aws_lc_rs;

pub async fn create_kube_client(
    sdk_config: &SdkConfig,
    cluster_name: &str,
) -> Result<kube::Client> {
    aws_lc_rs::default_provider()
        .install_default()
        .expect("failed to install aws-lc provider");

    let (eks_cluster_url, eks_cluster_cert) =
        eks_k8s_cluster_info(sdk_config, cluster_name).await?;

    let token = create_cluster_token(sdk_config, cluster_name).await?;

    let kubeconfig = kube::Config {
        cluster_url: eks_cluster_url,
        default_namespace: "observability".to_string(),
        auth_info: kube::config::AuthInfo {
            token: Some(token.clone().into()),
            ..Default::default()
        },
        root_cert: Some(eks_cluster_cert),
        accept_invalid_certs: false,
        connect_timeout: Some(Duration::from_secs(30)),
        read_timeout: Some(Duration::from_secs(295)),
        write_timeout: None,
        proxy_url: None,
        tls_server_name: None,
    };

    let client = kube::Client::try_from(kubeconfig)?;
    Ok(client)
}

pub async fn eks_k8s_cluster_info(
    sdk_config: &SdkConfig,
    cluster_name: &str,
) -> Result<(http::Uri, Vec<Vec<u8>>)> {
    let client = eks::Client::new(&sdk_config);

    let resp = client.describe_cluster().name(cluster_name).send().await?;

    let cluster = resp.cluster().context("Unable to find cluster")?.to_owned();
    let b64_cert = cluster
        .certificate_authority()
        .context("Unable to find certificate authority")?
        .data()
        .context("Unable to find certificate data")?;
    let cert = pem::parse(STANDARD.decode(b64_cert)?)?.into_contents();
    let endpoint = cluster
        .endpoint()
        .context("Unable to find endpoint")?
        .parse::<http::Uri>()?;

    Ok((endpoint, [cert].to_vec()))
}

async fn create_cluster_token(
    sdk_config: &SdkConfig,
    cluster_name: &str,
) -> Result<String> {
    let credentials = sdk_config
        .credentials_provider()
        .ok_or_else(|| anyhow!("no credentials provider in sdk_config"))?
        .provide_credentials()
        .await
        .context("failed to provide AWS credentials")?;
    let region = sdk_config.region().unwrap();

    let mut signing_settings = SigningSettings::default();
    signing_settings.expires_in = Some(Duration::from_secs(60));
    signing_settings.signature_location = SignatureLocation::QueryParams;

    let identity = Identity::from(credentials.clone());

    let signing_params = match aws_sigv4::sign::v4::SigningParams::builder()
        .identity(&identity)
        .region(region.as_ref())
        .name("sts")
        .time(SystemTime::now())
        .settings(signing_settings)
        .build()
    {
        Ok(params) => params,
        Err(e) => {
            return Err(anyhow!("Unable to create signing params: {:?}", e));
        }
    };

    let url = format!(
        "https://sts.{region}.amazonaws.com/?Action=GetCallerIdentity&Version=2011-06-15"
    );
    let headers = vec![("x-k8s-aws-id", cluster_name)];
    let signable_request = SignableRequest::new(
        "GET",
        url.clone(),
        headers.into_iter(),
        SignableBody::Bytes(&[]),
    )?;

    let (signing_instructions, _signature) = aws_sigv4::http_request::sign(
        signable_request,
        &aws_sigv4::http_request::SigningParams::V4(signing_params),
    )?
    .into_parts();

    let mut fake_req = http::Request::builder()
        .uri(url)
        .body(())
        .expect("empty body request should not fail");

    signing_instructions.apply_to_request_http1x(&mut fake_req);
    let uri = fake_req.uri().to_string();

    Ok(format!("k8s-aws-v1.{}", &URL_SAFE_NO_PAD.encode(uri)))
}
