#![deny(missing_docs)]
//! Ways to authenticate with the Webex API

use crate::{Authorization, RequestBody, RestClient};
use hyper::StatusCode;
use serde::Deserialize;
use tokio::time::{self, Duration, Instant};

const SCOPE: &str = "spark:all";
const GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";

#[allow(dead_code)]
/// Authenticates a device based on a Webex Integration
/// "client id" and a "client secret".
///
/// More information can be found on <https://developer.webex.com/docs/login-with-webex#device-grant-flow>.
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
    /// Unique user verification code.
    pub user_code: String,
    device_code: String,
    /// A verification URL the user can navigate
    /// to on a different device and provide the unique
    /// user verification code.
    pub verification_uri: String,
    /// A verification URL containing the embedded
    /// hashed user verification code.
    pub verification_uri_complete: String,
    interval: u64,
}

#[derive(Deserialize, Debug)]
struct TokenResponse {
    access_token: String,
}

/// Type alias for the bearer token.
pub type Bearer = String;

impl DeviceAuthenticator {
    /// Creates a new [`DeviceAuthenticator`] using the "client ID" and
    /// "client secret" provided by a Webex Integration.
    ///
    /// For more details: <https://developer.webex.com/docs/integrations>.
    #[must_use]
    pub fn new(id: &str, secret: &str) -> Self {
        let client = RestClient::new();
        Self {
            client_id: id.to_string(),
            client_secret: secret.to_string(),
            client,
        }
    }

    /// First step of device authentication. Returns a [`VerificationToken`]
    /// containing the codes and URLs that can be entered and navigated to
    /// on a different device.
    pub async fn verify(&self) -> Result<VerificationToken, crate::Error> {
        let params = &[("client_id", self.client_id.as_str()), ("scope", SCOPE)];
        let verification_token = self
            .client
            .api_post::<VerificationToken, _>(
                "device/authorize",
                RequestBody {
                    media_type: "application/x-www-form-urlencoded; charset=utf-8",
                    content: serde_urlencoded::to_string(params)?,
                },
                Authorization::None,
            )
            .await?;
        Ok(verification_token)
    }

    /// Second and final step of device authentication. Receives a [`VerificationToken`]
    /// provided by [`verify`](DeviceAuthenticator::verify) and blocks until the user enters their crendentials using
    /// the provided codes/links from [`VerificationToken`]. Returns a [`Bearer`] if successful.
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
                            return Err(crate::ErrorKind::Authentication.into());
                        }
                    }
                    _ => {
                        return Err(crate::ErrorKind::Authentication.into());
                    }
                },
            }
        }
    }
}
