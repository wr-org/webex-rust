use webex::auth::DeviceAuthenticator;

#[tokio::main]
async fn main() {
    let authenticator = DeviceAuthenticator::new("", "");
    authenticator.verify().await;
}
