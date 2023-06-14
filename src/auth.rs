use crate::RestClient;
use serde::Deserialize;

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
#[derive(Deserialize)]
pub struct VerificationToken {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    interval: u64,
}

impl DeviceAuthenticator {
    pub fn new(id: &str, secret: &str) -> DeviceAuthenticator {
        let client = RestClient::new();
        DeviceAuthenticator {
            client_id: id.to_string(),
            client_secret: secret.to_string(),
            client: client,
        }
    }

    pub async fn verify(&self) {
        let params = &[
            ("client_id", self.client_id.as_str()),
            ("scope", "spark:all"),
        ];
        if let Ok(body) = serde_urlencoded::to_string(params) {
            println!("{body}");
            self.client
                .api_post::<VerificationToken, _>("device/authorize", params)
                .await
                .expect("not able to get verification token");
        }
    }
}
