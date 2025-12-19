//! Container structures for organizing Adaptive Card content.

use serde::{Deserialize, Serialize};

use super::elements::CardElement;
use super::{Action, ContainerStyle, Spacing, VerticalContentAlignment};

/// Describes a choice for use in a `ChoiceSet`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Choice {
    /// Text to display.
    pub title: String,
    /// The raw value for the choice. **NOTE:** do not use a , in the value, since a `ChoiceSet` with isMultiSelect set to true returns a comma-delimited string of choice values.
    pub value: String,
}

/// Describes a Fact in a `FactSet` as a key/value pair.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Fact {
    /// The title of the fact.
    pub title: String,
    /// The value of the fact.
    pub value: String,
}

/// Column in a `ColumnSet`
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Column {
    /// The card elements to render inside the Column.
    #[serde(default)]
    pub items: Vec<CardElement>,
    /// An Action that will be invoked when the Column is tapped or selected.
    #[serde(rename = "selectAction", skip_serializing_if = "Option::is_none")]
    select_action: Option<Action>,
    /// Style hint for Column.
    #[serde(skip_serializing_if = "Option::is_none")]
    style: Option<ContainerStyle>,
    /// Defines how the content should be aligned vertically within the column.
    #[serde(
        rename = "verticalContentAlignment",
        skip_serializing_if = "Option::is_none"
    )]
    vertical_content_alignment: Option<VerticalContentAlignment>,
    /// When true, draw a separating line between this column and the previous column.
    #[serde(skip_serializing_if = "Option::is_none")]
    separator: Option<bool>,
    /// Controls the amount of spacing between this column and the preceding column.
    #[serde(skip_serializing_if = "Option::is_none")]
    spacing: Option<Spacing>,
    /// "auto", "stretch", a number representing relative width of the column in the column group, or in version 1.1 and higher, a specific pixel width, like "50px".
    #[serde(skip_serializing_if = "Option::is_none")]
    width: Option<serde_json::Value>,
    /// A unique identifier associated with the item.
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
}

impl From<&Self> for Column {
    fn from(item: &Self) -> Self {
        item.clone()
    }
}

impl From<&mut Self> for Column {
    fn from(item: &mut Self) -> Self {
        item.clone()
    }
}

impl Column {
    /// Creates new Column
    #[must_use]
    pub const fn new() -> Self {
        Self {
            items: vec![],
            select_action: None,
            style: None,
            vertical_content_alignment: None,
            separator: None,
            spacing: None,
            width: None,
            id: None,
        }
    }

    /// Adds element to column
    #[must_use]
    pub fn add_element(&mut self, item: CardElement) -> Self {
        self.items.push(item);
        self.into()
    }

    /// Sets separator
    #[must_use]
    pub fn set_separator(&mut self, s: bool) -> Self {
        self.separator = Some(s);
        self.into()
    }

    /// Sets `VerticalContentAlignment`
    #[must_use]
    pub fn set_vertical_alignment(&mut self, s: VerticalContentAlignment) -> Self {
        self.vertical_content_alignment = Some(s);
        self.into()
    }

    /// Sets width
    #[must_use]
    pub fn set_width<T: Into<String>>(&mut self, s: T) -> Self {
        self.width = Some(serde_json::Value::String(s.into()));
        self.into()
    }
}

impl Default for Column {
    fn default() -> Self {
        Self::new()
    }
}
