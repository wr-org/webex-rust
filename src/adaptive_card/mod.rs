//! Support for [Adaptive Cards](https://adaptivecards.io/) in Webex messages.
//!
//! Adaptive Cards are a way to create rich, interactive content that can be sent in messages.
//! They consist of various elements like text blocks, images, input fields, and actions.
//!
//! # Example
//! ```rust,no_run
//! use webex::adaptive_card::{AdaptiveCard, CardElement};
//!
//! let mut card = AdaptiveCard::new();
//! card.add_body(CardElement::text_block("Hello, World!"));
//! ```
//!
//! More info about the schema can be found [here](https://adaptivecards.io/explorer/)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Submodules
pub mod containers;
pub mod elements;
pub mod styles;

// Re-export main types
pub use containers::{Choice, Column, Fact};
pub use elements::CardElement;
pub use styles::*;

/// An Adaptive Card is the top-level object that describes a card.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AdaptiveCard {
    /// Must be "`AdaptiveCard`"
    #[serde(rename = "type")]
    pub card_type: String,
    /// Schema version that this card requires. If a client is lower than this version, the fallbackText will be rendered.
    /// Maximum version is 1.1
    #[serde(default = "default_version")] // Workaround for Webex not always providing it :/
    pub version: String,
    /// The card elements to show in the primary card region.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<CardElement>>,
    /// Actions available for this card
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<Action>>,
    /// An Action that will be invoked when the card is tapped or selected.
    #[serde(rename = "selectAction", skip_serializing_if = "Option::is_none")]
    pub select_action: Option<Box<Action>>,
    /// Text shown when the client doesn't support the version specified (may contain markdown).
    #[serde(rename = "fallbackText", skip_serializing_if = "Option::is_none")]
    pub fallback_text: Option<String>,
    /// Specifies the minimum height of the card.
    #[serde(rename = "minHeight", skip_serializing_if = "Option::is_none")]
    pub min_height: Option<String>,
    /// The 2-letter ISO-639-1 language used in the card. Used to localize any date/time functions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// The Adaptive Card schema.
    /// <http://adaptivecards.io/schemas/adaptive-card.json>
    #[serde(rename = "$schema")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
}

impl AdaptiveCard {
    /// Create new adaptive card with mandatory defaults
    #[must_use]
    pub fn new() -> Self {
        Self {
            card_type: "AdaptiveCard".to_string(),
            version: "1.1".to_string(),
            body: None,
            actions: None,
            select_action: None,
            fallback_text: None,
            min_height: None,
            lang: None,
            schema: Some("http://adaptivecards.io/schemas/adaptive-card.json".to_string()),
        }
    }

    /// Adds Element to body
    ///
    /// # Arguments
    ///
    /// * `card` - `CardElement` to add
    #[must_use]
    pub fn add_body<T: Into<CardElement>>(&mut self, card: T) -> Self {
        match self.body.take() {
            None => {
                self.body = Some(vec![card.into()]);
            }
            Some(mut body) => {
                body.push(card.into());
                self.body = Some(body);
            }
        }
        self.into()
    }

    /// Adds Actions
    ///
    /// # Arguments
    ///
    /// * `action` - Action to add
    #[must_use]
    pub fn add_action<T: Into<Action>>(&mut self, a: T) -> Self {
        match self.actions.take() {
            None => {
                self.actions = Some(vec![a.into()]);
            }
            Some(mut actions) => {
                actions.push(a.into());
                self.actions = Some(actions);
            }
        }
        self.into()
    }
}

impl Default for AdaptiveCard {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&Self> for AdaptiveCard {
    fn from(item: &Self) -> Self {
        item.clone()
    }
}

impl From<&mut Self> for AdaptiveCard {
    fn from(item: &mut Self) -> Self {
        item.clone()
    }
}

/// Actions that can be triggered by user interaction with an Adaptive Card.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum Action {
    /// Gathers input fields, merges with optional data field, and sends an event to the client. It is up to the client to determine how this data is processed. For example: With `BotFramework` bots, the client would send an activity through the messaging medium to the bot.
    #[serde(rename = "Action.Submit")]
    Submit {
        /// Initial data that input fields will be combined with. These are essentially 'hidden' properties.
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<HashMap<String, String>>,
        /// Label for button or link that represents this action.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ActionStyle>,
    },
    /// When invoked, show the given url either by launching it in an external web browser or showing within an embedded web browser.
    #[serde(rename = "Action.OpenUrl")]
    OpenUrl {
        /// The URL to open.
        url: String,
        /// Label for button or link that represents this action.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ActionStyle>,
    },
    /// Defines an `AdaptiveCard` which is shown to the user when the button or link is clicked.
    #[serde(rename = "Action.ShowCard")]
    ShowCard {
        /// The Adaptive Card to show.
        card: AdaptiveCard,
        /// Label for button or link that represents this action.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ActionStyle>,
    },
}

/// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
#[allow(missing_docs)]
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum ActionStyle {
    /// Action is displayed as normal
    #[default]
    Default,
    /// Action is displayed with a positive style (typically the button becomes accent color)
    Positive,
    /// Action is displayed with a destructive style (typically the button becomes red)
    Destructive,
}

fn default_version() -> String {
    "1.1".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adaptive_card_new() {
        let card = AdaptiveCard::new();
        assert_eq!(
            card.schema,
            Some("http://adaptivecards.io/schemas/adaptive-card.json".to_string())
        );
        assert_eq!(card.version, "1.1");
        assert_eq!(card.card_type, "AdaptiveCard");
        assert!(card.body.is_none());
        assert!(card.actions.is_none());
    }

    #[test]
    fn test_adaptive_card_add_body() {
        let mut card = AdaptiveCard::new();
        let text_block = CardElement::text_block("Hello World");
        let _ = card.add_body(text_block);

        assert!(card.body.is_some());
        assert_eq!(card.body.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_adaptive_card_add_multiple_body_elements() {
        let mut card = AdaptiveCard::new();
        let _ = card.add_body(CardElement::text_block("First"));
        let _ = card.add_body(CardElement::text_block("Second"));
        let _ = card.add_body(CardElement::text_block("Third"));

        assert_eq!(card.body.as_ref().unwrap().len(), 3);
    }

    #[test]
    fn test_adaptive_card_add_action() {
        let mut card = AdaptiveCard::new();
        let action = Action::ShowCard {
            title: Some("Show More".to_string()),
            card: AdaptiveCard::new(),
            style: None,
        };
        let _ = card.add_action(action);

        assert!(card.actions.is_some());
        assert_eq!(card.actions.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn test_adaptive_card_add_multiple_actions() {
        let mut card = AdaptiveCard::new();
        let _ = card.add_action(Action::ShowCard {
            title: Some("First".to_string()),
            card: AdaptiveCard::new(),
            style: None,
        });
        let _ = card.add_action(Action::ShowCard {
            title: Some("Second".to_string()),
            card: AdaptiveCard::new(),
            style: None,
        });

        assert_eq!(card.actions.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_card_element_text_block() {
        let element = CardElement::text_block("Test text");
        match element {
            CardElement::TextBlock { text, .. } => {
                assert_eq!(text, "Test text");
            }
            _ => panic!("Expected TextBlock"),
        }
    }

    #[test]
    fn test_card_element_set_separator() {
        let mut element = CardElement::text_block("Test");
        let _ = element.set_separator(true);

        match element {
            CardElement::TextBlock { separator, .. } => {
                assert_eq!(separator, Some(true));
            }
            _ => panic!("Expected TextBlock"),
        }
    }

    #[test]
    fn test_card_element_set_spacing() {
        let mut element = CardElement::text_block("Test");
        let _ = element.set_spacing(Spacing::Large);

        match element {
            CardElement::TextBlock { spacing, .. } => {
                assert_eq!(spacing, Some(Spacing::Large));
            }
            _ => panic!("Expected TextBlock"),
        }
    }

    #[test]
    fn test_card_element_action_set() {
        let action_set = CardElement::action_set();
        match action_set {
            CardElement::ActionSet {
                actions,
                horizontal_alignment,
                separator,
                spacing,
                ..
            } => {
                assert_eq!(actions.len(), 0);
                assert_eq!(horizontal_alignment, None);
                assert_eq!(separator, None);
                assert_eq!(spacing, None);
            }
            _ => panic!("Expected ActionSet"),
        }
    }

    #[test]
    fn test_card_element_set_horizontal_alignment() {
        let mut element = CardElement::text_block("Test");
        let _ = element.set_horizontal_alignment(HorizontalAlignment::Center);

        match element {
            CardElement::TextBlock {
                horizontal_alignment,
                ..
            } => {
                assert_eq!(horizontal_alignment, Some(HorizontalAlignment::Center));
            }
            _ => panic!("Expected TextBlock"),
        }
    }

    #[test]
    fn test_card_element_container() {
        let container = CardElement::container();
        match container {
            CardElement::Container { items, .. } => {
                assert_eq!(items.len(), 0);
            }
            _ => panic!("Expected Container"),
        }
    }

    #[test]
    fn test_column_new() {
        let column = Column::new();
        assert_eq!(column.items.len(), 0);
    }
}
