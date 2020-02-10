use std::env;

const BOT_ACCESS_TOKEN: &'static str = "BOT_ACCESS_TOKEN";
const DEST_EMAIL: &'static str = "DEST_EMAIL";

/// # Hello World
///
/// This example sends the specified user a direct message.
///
/// # Usage
///
/// BOT_ACCESS_TOKEN="<token>" DEST_EMAIL="you@where.com" cargo run --example hello-world
///
/// You can obtain a bot token by logging into the [Cisco Webex developer site](https://developer.webex.com/), then
///
/// * Select "My Webex Apps" from your profile menu (available by clicking on your avatar on the top right)
/// * Select "Create New App"
/// * Select "Create a Bot"
/// * Choose something unique to yourself for testing, e.g., "username-hello"
/// * **Save** the "Bot's Access Token" you see on the next page.  If you fail to do so, you can
///   regenerate it later, but this will invalidate the old token.
///

#[tokio::main]
async fn main() {
    let token = env::var(BOT_ACCESS_TOKEN)
        .expect(format!("{} not specified in environment", BOT_ACCESS_TOKEN).as_str());
    let to_email = env::var(DEST_EMAIL)
        .expect(format!("{} not specified in environment", DEST_EMAIL).as_str());

    let webex = webex::Webex::new(token.as_str());
    let text = format!("Hello, {}", to_email);

    let msg_to_send = webex::types::MessageOut {
        to_person_email: Some(to_email),
        text: Some(text),
        ..Default::default()
    };

    webex.send_message(&msg_to_send).await.unwrap();
}
