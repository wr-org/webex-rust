#![deny(missing_docs)]
//! Adaptive Card implementation
//!
//! [Webex Teams currently supports only version 1.1](https://developer.webex.com/docs/cards)
//!
//! More info about the schema can be found [here](https://adaptivecards.io/explorer/)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Adaptive Card structure for message attachment
#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AdaptiveCard {
    /// Must be "AdaptiveCard"
    #[serde(rename = "type")]
    pub card_type: String,
    /// Schema version that this card requires. If a client is lower than this version, the fallbackText will be rendered.
    /// Maximum version is 1.1
    pub version: String,
    /// The card elements to show in the primary card region.
    pub body: Option<Vec<CardElement>>,
    /// Actions available for this card
    pub actions: Option<Vec<Action>>,
    /// An Action that will be invoked when the card is tapped or selected.
    #[serde(rename = "selectAction")]
    pub select_action: Option<Box<Action>>,
    /// Text shown when the client doesn’t support the version specified (may contain markdown).
    #[serde(rename = "fallbackText")]
    pub fallback_text: Option<String>,
    /// Specifies the minimum height of the card.
    #[serde(rename = "minHeight")]
    pub min_height: Option<String>,
    /// The 2-letter ISO-639-1 language used in the card. Used to localize any date/time functions.
    pub lang: Option<String>,
    /// The Adaptive Card schema.
    /// http://adaptivecards.io/schemas/adaptive-card.json
    #[serde(rename = "$schema")]
    pub schema: String,
}

/// Card element types
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum CardElement {
    /// Containers group items together.
    Container {
        /// The card elements to render inside the Container.
        items: Vec<CardElement>,
        /// An Action that will be invoked when the Container is tapped or selected.
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        /// Style hint for Container.
        style: Option<ContainerStyle>,
        /// Defines how the content should be aligned vertically within the container.
        #[serde(rename = "verticalContentAlignment")]
        vertical_content_alignment: Option<VerticalContentAlignment>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// ColumnSet divides a region into Columns, allowing elements to sit side-by-side.
    ColumnSet {
        /// The array of Columns to divide the region into.
        columns: Vec<Column>,
        /// An Action that will be invoked when the ColumnSet is tapped or selected.
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// The FactSet element displays a series of facts (i.e. name/value pairs) in a tabular form.
    FactSet {
        /// 	The array of Fact‘s.
        facts: Vec<Column>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// The ImageSet displays a collection of Images similar to a gallery.
    ImageSet {
        /// The array of Image elements to show.
        images: Vec<CardElement>,
        /// Controls the approximate size of each image. The physical dimensions will vary per host.
        #[serde(rename = "imageSize")]
        image_size: Option<ImageSize>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Displays text, allowing control over font sizes, weight, and color.
    TextBlock {
        /// Text to display
        text: String,
        /// If true, allow text to wrap. Otherwise, text is clipped.
        wrap: Option<bool>,
        /// Controls the color of TextBlock elements.
        color: Option<Color>,
        /// Controls the horizontal text alignment.
        #[serde(rename = "HorizontalAlignment")]
        horizontal_alignment: Option<HorizontalAlignment>,
        /// If true, displays text slightly toned down to appear less prominent.
        #[serde(rename = "isSubtle")]
        is_subtle: Option<bool>,
        /// Specifies the maximum number of lines to display.
        #[serde(rename = "maxLines")]
        max_lines: Option<u64>,
        /// Controls size of text.
        size: Option<Size>,
        /// Controls the weight of TextBlock elements.
        weight: Option<Weight>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Displays an image.
    Image {
        /// The URL to the image.
        url: String,
        /// Alternate text describing the image.
        #[serde(rename = "altText")]
        alt_text: Option<String>,
        /// Applies a background to a transparent image. This property will respect the image style.
        /// hex value of a color (e.g. #982374)
        #[serde(rename = "backgroundColor")]
        background_color: Option<String>,
        /// The desired on-screen width of the image, ending in ‘px’. E.g., 50px. This overrides the size property.
        width: Option<String>,
        /// The desired height of the image. If specified as a pixel value, ending in ‘px’, E.g., 50px, the image will distort to fit that exact height. This overrides the size property.
        height: Option<String>,
        /// Controls how this element is horizontally positioned within its parent.
        #[serde(rename = "horizontalAlignment")]
        horizontal_alignment: Option<HorizontalAlignment>,
        /// An Action that will be invoked when the Image is tapped or selected. Action.ShowCard is not supported.
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        /// Controls the approximate size of the image. The physical dimensions will vary per host.
        size: Option<ImageSize>,
        /// Controls how this Image is displayed.
        style: Option<ImageStyle>,
        /// A unique identifier associated with the item.
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Lets a user enter text.
    #[serde(rename = "Input.Text")]
    InputText {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no text has been input.
        placeholder: Option<String>,
        /// If true, allow multiple lines of input.
        #[serde(rename = "isMultiline")]
        is_multiline: Option<bool>,
        /// Hint of maximum length characters to collect (may be ignored by some clients).
        #[serde(rename = "maxLength")]
        max_length: Option<u64>,
        /// Text Input Style
        style: Option<TextInputStyle>,
        /// The initial value for this field.
        value: Option<String>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Allows a user to enter a number.
    #[serde(rename = "Input.Number")]
    InputNumber {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no selection has been made.
        placeholder: Option<String>,
        /// Hint of maximum value (may be ignored by some clients).
        max: Option<f64>,
        /// Hint of minimum value (may be ignored by some clients).
        min: Option<f64>,
        /// Initial value for this field.
        value: Option<f64>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Lets a user choose a date.
    #[serde(rename = "Input.Date")]
    InputDate {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no selection has been made.
        placeholder: Option<String>,
        /// Hint of maximum value expressed in YYYY-MM-DD(may be ignored by some clients).
        max: Option<String>,
        /// Hint of minimum value expressed in YYYY-MM-DD(may be ignored by some clients).
        min: Option<String>,
        /// The initial value for this field expressed in YYYY-MM-DD.
        value: Option<String>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Lets a user select a time.
    #[serde(rename = "Input.Time")]
    InputTime {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Hint of maximum value expressed in HH:MM (may be ignored by some clients).
        max: Option<String>,
        /// Hint of minimum value expressed in HH:MM (may be ignored by some clients).
        min: Option<String>,
        /// The initial value for this field expressed in HH:MM.
        value: Option<String>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Lets a user choose between two options.
    #[serde(rename = "Input.Toggle")]
    InputToggle {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// The initial selected value. If you want the toggle to be initially on, set this to the value of valueOn‘s value.
        value: Option<String>,
        /// The value when toggle is off
        #[serde(rename = "valueOff")]
        value_off: Option<String>,
        /// The value when toggle is on
        #[serde(rename = "valueOn")]
        value_on: Option<String>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Allows a user to input a Choice.
    #[serde(rename = "Input.ChoiceSet")]
    InputChoiceSet {
        /// Choice options.
        choices: Vec<Choice>,
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Allow multiple choices to be selected.
        #[serde(rename = "isMultiSelect")]
        is_multi_select: Option<bool>,
        /// Input Choice Style
        style: Option<ChoiceInputStyle>,
        /// The initial choice (or set of choices) that should be selected. For multi-select, specify a comma-separated string of values.
        value: Option<String>,
        /// Specifies the height of the element.
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        spacing: Option<Spacing>,
    },

    /// Displays a set of actions.
    ActionSet {
        /// The array of Action elements to show.
        actions: Vec<Action>,
        /// Specifies the height of the element.
        height: Option<Height>,
    },
}

/// Defines a container that is part of a ColumnSet.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Column {
    /// The card elements to render inside the Column.
    items: Vec<CardElement>,
    /// An Action that will be invoked when the Column is tapped or selected.
    #[serde(rename = "selectAction")]
    select_action: Option<Action>,
    /// Style hint for Column.
    style: Option<ContainerStyle>,
    /// Defines how the content should be aligned vertically within the column.
    #[serde(rename = "verticalContentAlignment")]
    vertical_content_alignment: Option<VerticalContentAlignment>,
    /// When true, draw a separating line between this column and the previous column.
    separator: Option<bool>,
    /// Controls the amount of spacing between this column and the preceding column.
    spacing: Option<Spacing>,
    /// "auto", "stretch", a number representing relative width of the column in the column group, or in version 1.1 and higher, a specific pixel width, like "50px".
    width: Option<String>,
    /// A unique identifier associated with the item.
    id: Option<String>,
}

/// Describes a Fact in a FactSet as a key/value pair.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Fact {
    /// The title of the fact.
    title: String,
    /// 	The value of the fact.
    value: String,
}

/// Available color options
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Color {
    Default,
    Dark,
    Light,
    Accent,
    Good,
    Warning,
    Attention,
}

/// Container Styles
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ContainerStyle {
    Default,
    Emphasis,
    Good,
    Attention,
    Warning,
    Accent,
}

/// Spacing options
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Spacing {
    Default,
    None,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Padding,
}

/// Choice Input Style
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ChoiceInputStyle {
    Compact,
    Expanded,
}

/// Vertical alignment of content
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum VerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

/// Text Input Style
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TextInputStyle {
    Text,
    Tel,
    Url,
    Email,
}

/// Height
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Height {
    Auto,
    Stretch,
}

/// Image Style
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ImageStyle {
    Default,
    Person,
}

/// Text Weight
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Weight {
    Default,
    Lighter,
    Bolder,
}

/// Text Size
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Size {
    Default,
    Small,
    Medium,
    Large,
    ExtraLarge,
}

/// Image Size
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ImageSize {
    Auto,
    Stretch,
    Small,
    Medium,
    Large,
}

/// Controls how this element is horizontally positioned within its parent.
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

/// Available Card Actions
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Action {
    /// Gathers input fields, merges with optional data field, and sends an event to the client. It is up to the client to determine how this data is processed. For example: With BotFramework bots, the client would send an activity through the messaging medium to the bot.
    #[serde(rename = "Action.Submit")]
    Submit {
        /// Initial data that input fields will be combined with. These are essentially ‘hidden’ properties.
        data: Option<HashMap<String, String>>,
        /// Label for button or link that represents this action.
        title: Option<String>,
    },

    /// When invoked, show the given url either by launching it in an external web browser or showing within an embedded web browser.
    #[serde(rename = "Action.OpenUrl")]
    OpenUrl {
        /// The URL to open.
        url: String,
        /// Label for button or link that represents this action.
        title: Option<String>,
    },

    /// Defines an AdaptiveCard which is shown to the user when the button or link is clicked.
    #[serde(rename = "Action.ShowCard")]
    ShowCard {
        /// The Adaptive Card to show.
        card: AdaptiveCard,
        /// Label for button or link that represents this action.
        title: Option<String>,
    },
}

/// Describes a choice for use in a ChoiceSet.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Choice {
    /// Text to display.
    pub title: String,
    /// The raw value for the choice. **NOTE:** do not use a , in the value, since a ChoiceSet with isMultiSelect set to true returns a comma-delimited string of choice values.
    pub value: String,
}