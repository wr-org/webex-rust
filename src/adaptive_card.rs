use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AdaptiveCard {
    #[serde(rename = "type")]
    pub card_type: String, // Must be "AdaptiveCard"
    pub version: String,
    pub body: Option<Vec<AdaptiveCardBodyElement>>,
    pub actions: Option<Vec<AdaptiveCardAction>>,
    #[serde(rename = "selectAction")]
    pub select_action: Option<AdaptiveCardSelectAction>,
    #[serde(rename = "fallbackText")]
    pub fallback_text: Option<String>,
    pub background_image: Option<String>,
    #[serde(rename = "minHeight")]
    pub min_height: Option<String>,
    pub speak: Option<String>,
    pub lang: Option<String>,
    #[serde(rename = "$schema")]
    pub schema: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum AdaptiveCardBodyElement {
    Container { items: Vec<CardElement> },
    TextBlock,
    ActionSet { actions: Vec<Action> },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum CardElement {
    TextBlock {
        text: String,
        weight: String,
        size: String,
        wrap: bool,
    },

    #[serde(rename = "Input.ChoiceSet")]
    InputChoiceSet {
        placeholder: String,
        choices: Vec<Choice>,
        id: String,
    },

    #[serde(rename = "Input.Text")]
    InputText { placeholder: String, id: String },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "Action.Submit")]
    Submit {
        title: String,
        id: String,
        style: String,
        data: Option<HashMap<String, String>>,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Choice {
    pub title: String,
    pub value: String,
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AdaptiveCardContainer {
    #[serde(flatten)]
    #[serde(rename = "type", default = "str_container")]
    pub container_type: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AdaptiveCardAction {
    RawText(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum AdaptiveCardSelectAction {
    RawText(String),
}
