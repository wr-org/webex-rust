use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub bot_token: String,
    //pub admins: Vec<String>,
    pub email: String,
}
use std::collections::HashMap;

use webex::{
    self,
    adaptive_card::{AdaptiveCard, CardElement},
};

const HELP_MESSAGE: &str = r#"
# Test AdaptiveCard
Usage:
 * `help` - show this help message
 * `start` - show test card
"#;

#[tokio::main]
async fn main() {
    let config = std::fs::read_to_string("config.json").expect("Failed to find config.json");
    let config: Config = serde_json::from_str(&config).expect("config.json invalid format");
    let webex = webex::Webex::new(&config.bot_token).await;
    let mut eventstream = webex
        .event_stream()
        .await
        .expect("Failed to get eventstream");

    while eventstream.is_open {
        let event = match eventstream.next().await {
            Ok(event) => event,
            Err(e) => {
                println!("Eventstream failed: {}", e);
                continue;
            }
        };
        match event.activity_type() {
            webex::ActivityType::Message(webex::MessageActivity::Posted) => {
                respond_to_message(&webex, &config, &event).await
            }
            webex::ActivityType::AdaptiveCardSubmit => handle_adaptive_card(&webex, &event).await,
            _ => {
                //dbg!(event);
            }
        }
    }
    println!("Eventstream closed!");
}

async fn handle_adaptive_card(webex: &webex::Webex, event: &webex::Event) {
    // get attachmentactions
    let actions: webex::types::AttachmentAction = match webex.get(&event.get_global_id()).await {
        Ok(a) => a,
        Err(e) => {
            println!("Error: {}", e);
            return;
        }
    };
    let which_card = actions.inputs.as_ref().and_then(|inputs| inputs.get("id"));
    match which_card {
        None => println!(
            "ERROR: expected card to have both inputs and id, got {:?}",
            actions
        ),
        Some(s) => match s.as_str() {
            "init" => handle_adaptive_card_init(webex, &actions).await,
            id => println!("AdaptiveCard id {id} not handled!"),
        },
    }
}

async fn handle_adaptive_card_init(webex: &webex::Webex, actions: &webex::AttachmentAction) {
    // get attachmentactions
    let input1 = actions
        .inputs
        .as_ref()
        .and_then(|inputs| inputs.get("input1"));
    let input2 = actions
        .inputs
        .as_ref()
        .and_then(|inputs| inputs.get("input2"));
    if let (Some(input1), Some(input2)) = (input1, input2) {
        println!(
            "Recieved initial adaptive card, inputs {} and {}",
            input1, input2
        );
        return;
    }

    let mut reply = webex::MessageOut::from(actions);
    reply.text = Some(format!(
        "Your replies were: {:?}",
        actions
            .inputs
            .as_ref()
            .expect("expected action to have inputs")
    ));
    webex
        .send_message(&reply)
        .await
        .expect("Couldn't send reply");
}

async fn respond_to_message(webex: &webex::Webex, config: &Config, event: &webex::Event) {
    // Got a posted message
    let message: webex::Message = match webex.get(&event.get_global_id()).await {
        Ok(msg) => msg,
        Err(e) => {
            println!("Failed to get message: {}", e);
            return;
        }
    };
    if message.person_email.as_ref() == Some(&config.email) {
        return;
    }

    let mut reply: webex::MessageOut = webex::MessageOut::from(&message);
    if message
        .text
        .as_ref()
        .map(|msg| msg.contains("help"))
        .unwrap_or(false)
    {
        // Send help message
        reply.markdown = Some(HELP_MESSAGE.into());
        webex
            .send_message(&reply)
            .await
            .expect("Failed to send help message");
        return;
    }

    // Send event card
    reply.text = Some("Welcome to Adaptivecard Tester Bot".into());
    let mut body = CardElement::container();
    body.add_element(CardElement::text_block(
        "Welcome to Adaptivecard Tester Bot!",
    ));
    body.add_element(
        CardElement::column_set()
            .add_column(
                webex::adaptive_card::Column::new()
                    .add_element(CardElement::text_block("Input 1:"))
                    .add_element(CardElement::text_block("Input 2:")),
            )
            .add_column(
                webex::adaptive_card::Column::new()
                    .add_element(
                        CardElement::input_choice_set("input1", None::<&'static str>)
                            .add_key_value("Option A", "First Option")
                            .add_key_value("Option B", "Second Option"),
                    )
                    .add_element(CardElement::input_text("input2", None::<&'static str>)),
            ),
    );
    body.add_element(CardElement::action_set().add_action_to_set(
        webex::adaptive_card::Action::Submit {
            data: Some(HashMap::from([("id".into(), "init".into())])),
            title: Some("Submit".into()),
            style: None,
        },
    ));
    let card = AdaptiveCard::new().add_body(body);
    reply.add_attachment(card);
    let _resp = webex
        .send_message(&reply)
        .await
        .expect("Failed to send message");
}
