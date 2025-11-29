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

use anyhow::{Result, anyhow, bail};
use aws_config::BehaviorVersion;
use aws_types::region::Region;
use kops_aws_sso::{SsoLoginConfig, login_device_flow};
use kops_protocol::{LoginRequest, Request, Response};

use crate::helper::send_request;

pub async fn execute(name: String, region: Option<String>) -> Result<()> {
    let region = region
        .or_else(|| std::env::var("AWS_REGION").ok())
        .unwrap_or_else(|| "us-east-1".to_string());

    let start_url = std::env::var("KOPS_SSO_START_URL")
        .map_err(|_| anyhow!("KOPS_SSO_START_URL not set"))?;
    let account_id = std::env::var("KOPS_SSO_ACCOUNT_ID")
        .map_err(|_| anyhow!("KOPS_SSO_ACCOUNT_ID not set"))?;
    let role_name = std::env::var("KOPS_SSO_ROLE_NAME")
        .map_err(|_| anyhow!("KOPS_SSO_ROLE_NAME not set"))?;

    let client_name = format!("kops");

    let sso_cfg = SsoLoginConfig {
        region: region.clone(),
        start_url,
        account_id: account_id.clone(),
        role_name: role_name.clone(),
        client_name,
    };

    let sdk_config = aws_config::from_env()
        .region(Region::new(region.clone()))
        .load()
        .await;

    println!("Starting AWS SSO device flow for profile '{name}'...");
    println!("Region     : {region}");
    println!("Account ID : {account_id}");
    println!("Role name  : {role_name}");
    println!();

    let session = login_device_flow(&sdk_config, &sso_cfg, |info| {
        println!("SSO user code       : {}", info.user_code);
        println!("Verification URL    : {}", info.verification_uri);

        if let Some(full) = &info.verification_uri_complete {
            println!("Verification (full) : {full}");

            if let Err(err) = webbrowser::open(full) {
                eprintln!("Failed to open browser automatically: {err}");
                eprintln!("Please open the URL manually.");
            } else {
                println!(
                    "Browser opened automatically, please finish authentication."
                );
            }
        } else if let Err(err) = webbrowser::open(&info.verification_uri) {
            eprintln!("Failed to open browser automatically: {err}");
            eprintln!("Please open the URL manually.");
        } else {
            println!(
                "Browser opened automatically, please finish authentication."
            );
        }

        println!();
        println!("Waiting for AWS SSO authorization...");
    })
    .await?;

    println!(
        "Successfully obtained AWS credentials for account {} role {}",
        session.account_id, session.role_name
    );

    let expires_at_epoch_ms = session.expires_at.timestamp_millis();

    let creds = session.credentials;
    let access_key_id = creds.access_key_id().to_string();
    let secret_access_key = creds.secret_access_key().to_string();
    let session_token = creds
        .session_token()
        .ok_or_else(|| anyhow!("missing session token in AWS credentials"))?
        .to_string();

    let req = Request::Login(LoginRequest {
        name: name.clone(),
        region: Some(region),
        account_id,
        role_name,
        access_key_id,
        secret_access_key,
        session_token,
        expires_at_epoch_ms,
    });

    let resp = send_request(req).await?;

    match resp {
        Response::LoginOk => {
            println!(
                "kopsd registered AWS session for profile '{name}' successfully."
            );
        }
        Response::Error { message } => {
            bail!("daemon returned error on login: {message}");
        }
        _ => bail!("unexpected response to login"),
    }

    Ok(())
}
