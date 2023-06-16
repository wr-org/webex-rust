#![deny(missing_docs)]
//! Different authenticators

use crate::{Authorization, RequestBody, RestClient};
use hyper::StatusCode;
use serde::Deserialize;
use tokio::time::{self, Duration, Instant};

const SCOPE: &str = "spark:all";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[allow(dead_code)]
/// Authenticates a device based on a Webex Integration
/// "client id" and a "client secret". More information
/// can be found on https://developer.webex.com/docs/login-with-webex#device-grant-flow
pub struct DeviceAuthenticator {
    client_id: String,
    client_secret: String,
    client: RestClient,
}

/// This struct contains the codes and URIs necessary
/// to complete the "device grant flow" log in.
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
pub struct VerificationToken {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    interval: u64,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

pub type Bearer = String;

impl DeviceAuthenticator {
    pub fn new(id: &str, secret: &str) -> DeviceAuthenticator {
        let client = RestClient::new();
        DeviceAuthenticator {
            client_id: id.to_string(),
            client_secret: secret.to_string(),
            client: client,
        }
    }

    pub async fn verify(&self) -> Result<VerificationToken, crate::Error> {
        let params = &[("client_id", self.client_id.as_str()), ("scope", SCOPE)];
        Ok(self
            .client
            .api_post::<VerificationToken, _>(
                "device/authorize",
                RequestBody {
                    media_type: "application/x-www-form-urlencoded; charset=utf-8",
                    content: serde_urlencoded::to_string(params)?,
                },
                Authorization::None,
            )
            .await?)
    }

    pub async fn wait_for_authentication(
        &self,
        verification_token: &VerificationToken,
    ) -> Result<Bearer, crate::Error> {
        let params = [
            ("grant_type", GRANT_TYPE),
            ("device_code", &verification_token.device_code),
            ("client_id", &self.client_id),
        ];

        let mut interval = time::interval_at(
            Instant::now() + Duration::from_secs(verification_token.interval),
            Duration::from_secs(verification_token.interval + 1),
        );

        loop {
            interval.tick().await;

            match self
                .client
                .api_post::<TokenResponse, String>(
                    "device/token",
                    RequestBody {
                        media_type: "application/x-www-form-urlencoded; charset=utf-8",
                        content: serde_urlencoded::to_string(params)?,
                    },
                    Authorization::Basic {
                        username: &self.client_id,
                        password: &self.client_secret,
                    },
                )
                .await
            {
                Ok(token) => return Ok(token.access_token),
                Err(e) => match e.kind() {
                    crate::error::ErrorKind::StatusText(http_status, _) => {
                        if *http_status != StatusCode::PRECONDITION_REQUIRED {
                            panic!("unexpected HTTP status {}", *http_status);
                        }
                    }
                    _ => panic!("{}", e),
                },
            }
        }
    }
}
