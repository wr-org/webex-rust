use hyper::{http::header, Body, Client};
use hyper_tls::HttpsConnector;
use tungstenite::handshake::client::Request;
use webex::error::{Error, ErrorKind, Result, ResultExt};
use webex::Webex;

pub struct DeviceAuthenticator {
    client_id: String,
    client_secret: String,
}

pub struct VerificationToken {
    pub user_code: String,
    device_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: String,
    polling_interval: u64,
}

impl DeviceAuthenticator {
    pub fn new(client_id: &str, client_secret: &str) -> DeviceAuthenticator {
        DeviceAuthenticator {
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
        }
    }

    pub async fn verify(&self) -> Result<()> {
        let https = HttpsConnector::new();
        let client = Client::builder().build::<_, hyper::Body>(https);
        let Ok(req) = Request::builder()
            .method("POST")
            .uri("https://webexapis.com/v1/device/authorize")
            .header(
                header::CONTENT_TYPE,
                "application/x-www-form-urlencoded; charset=utf-8",
            )
            .body(Body::from(format!(
                "client_id={}&scope={}",
                self.client_id, "spark:all"
            ))) else {
                return Err(ErrorKind::Authentication.into());
            };
        match client.request(req).await {
            Ok(response) => println!("{:#?}", response),
            Err(e) => println!("{:#?}", e),
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let authenticator = DeviceAuthenticator {
        client_id: "Cfe9391b1cd1d0cafb092749b2db9dc53fd5e4d9803514d7f107fcb876ac447ad".to_string(),
        client_secret: "e6cbe3a559820b5302c08652fa90415fc546cf156bc67bf77e4184010df1ed85"
            .to_string(),
    };
    authenticator.verify().await.expect("error verifying");
}
