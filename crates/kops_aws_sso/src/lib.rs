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

use std::time::SystemTime;

use anyhow::{Context, Result, anyhow};
use aws_config::SdkConfig;
use aws_credential_types::Credentials;
use aws_sdk_sso as sso;
use aws_sdk_sso::error::ProvideErrorMetadata;
use aws_sdk_ssooidc as ssooidc;
use chrono::{DateTime, Duration, Utc};
use tokio::time::sleep;

#[derive(Debug, Clone)]
pub struct SsoLoginConfig {
    pub region: String,
    pub start_url: String,
    pub account_id: String,
    pub role_name: String,
    pub client_name: String,
}

#[derive(Debug, Clone)]
pub struct AwsSsoSession {
    pub credentials: Credentials,
    pub account_id: String,
    pub role_name: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct DeviceVerificationInfo {
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub expires_in: u64,
}

pub async fn login_device_flow<F>(
    sdk_config: &SdkConfig,
    config: &SsoLoginConfig,
    on_verification: F,
) -> Result<AwsSsoSession>
where
    F: Fn(&DeviceVerificationInfo) + Send + Sync,
{
    let oidc_client = ssooidc::Client::new(sdk_config);

    let register_out = oidc_client
        .register_client()
        .client_name(config.client_name.clone())
        .client_type("public")
        .send()
        .await
        .context("failed to register OIDC client")?;

    let client_id = register_out
        .client_id()
        .ok_or_else(|| anyhow!("missing client_id from register_client"))?
        .to_string();

    let client_secret = register_out
        .client_secret()
        .ok_or_else(|| anyhow!("missing client_secret from register_client"))?
        .to_string();

    let device_auth = oidc_client
        .start_device_authorization()
        .client_id(client_id.clone())
        .client_secret(client_secret.clone())
        .start_url(config.start_url.clone())
        .send()
        .await
        .context("failed to start device authorization")?;

    let verification_uri = device_auth
        .verification_uri_complete()
        .ok_or(anyhow!("missing verification URI"))?;

    let device_code = must(device_auth.device_code(), "device_code")?;
    let verification_uri = device_auth
        .verification_uri_complete()
        .or(device_auth.verification_uri())
        .ok_or_else(|| anyhow!("verification_uri missing"))?
        .to_string();
    let user_code = must(device_auth.user_code(), "user_code")?;
    let mut interval_secs = device_auth.interval() as u64;
    let expires_in = device_auth.expires_in() as u64;

    let verification_info = DeviceVerificationInfo {
        user_code,
        verification_uri: verification_uri.clone(),
        verification_uri_complete: device_auth
            .verification_uri_complete()
            .map(|s| s.to_string()),
        expires_in,
    };

    on_verification(&verification_info);

    let max_attempts = expires_in / interval_secs + 1;
    let access_token = {
        let mut access_token: Option<String> = None;

        for _ in 0..max_attempts {
            let res = oidc_client
                .create_token()
                .client_id(client_id.clone())
                .client_secret(client_secret.clone())
                .grant_type("urn:ietf:params:oauth:grant-type:device_code")
                .device_code(device_code.clone())
                .send()
                .await;

            match res {
                Ok(out) => {
                    access_token = out.access_token().map(|s| s.to_string());
                    break;
                }
                Err(e) => {
                    let code = e.code().unwrap_or("Unknown");
                    let msg = e.message().unwrap_or("");

                    match code {
                        "AuthorizationPendingException" => {
                            sleep(std::time::Duration::from_secs(
                                interval_secs,
                            ))
                            .await;
                            continue;
                        }
                        "SlowDownException" => {
                            interval_secs += 5;
                            sleep(std::time::Duration::from_secs(
                                interval_secs,
                            ))
                            .await;
                            continue;
                        }
                        "ExpiredTokenException" => {
                            return Err(anyhow::anyhow!(
                                "Device authorization expired (ExpiredTokenException): {msg}"
                            ));
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "CreateToken failed: {code}: {msg}"
                            ));
                        }
                    }
                }
            }
        }

        access_token.ok_or_else(|| {
            anyhow!("did not obtain access_token before timeout")
        })?
    };

    let sso_client = sso::Client::new(sdk_config);
    let out = sso_client
        .get_role_credentials()
        .access_token(access_token.clone())
        .account_id(config.account_id.clone())
        .role_name(config.role_name.clone())
        .send()
        .await
        .context("get_role_credentials failed")?;

    let role_creds = out
        .role_credentials()
        .ok_or_else(|| anyhow!("missing roleCredentials"))?;

    let access_key_id = must(role_creds.access_key_id(), "accessKeyId")?;
    let secret_access_key =
        must(role_creds.secret_access_key(), "secretAccessKey")?;
    let session_token = must(role_creds.session_token(), "sessionToken")?;

    let expires_ms = role_creds.expiration();
    let expires_at = DateTime::<Utc>::from(SystemTime::UNIX_EPOCH)
        + Duration::milliseconds(expires_ms);

    let creds = Credentials::new(
        access_key_id,
        secret_access_key,
        Some(session_token),
        Some(expires_at.into()),
        "kops_aws_sso::login_device_flow",
    );

    Ok(AwsSsoSession {
        credentials: creds,
        account_id: config.account_id.clone(),
        role_name: config.role_name.clone(),
        expires_at,
    })
}

fn must(v: Option<&str>, name: &str) -> Result<String> {
    v.ok_or_else(|| anyhow!("missing {name}")).map(|s| s.to_string())
}
