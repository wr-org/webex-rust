use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct AdaptiveCard {
    #[serde(rename = "type")]
    pub card_type: String,
    // Must be "AdaptiveCard"
    pub version: String,
    pub body: Option<Vec<CardElement>>,
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
pub enum CardElement {
    Container {
        items: Vec<CardElement>,
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        style: Option<ContainerStyle>,
        #[serde(rename = "verticalContentAlignment")]
        vertical_content_alignment: Option<VerticalContentAlignment>,
        // Inherited
        height: Option<Height>,
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    ColumnSet {
        columns: Vec<Column>,
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        // Inherited
        height: Option<Height>,
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    FactSet {
        facts: Vec<Column>,
        // Inherited
        height: Option<Height>,
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    ImageSet {
        images: Vec<CardElement>,
        #[serde(rename = "imageSize")]
        image_size: Option<ImageSize>,
        // Inherited
        height: Option<Height>,
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    TextBlock {
        text: String,
        wrap: Option<bool>,
        color: Option<Color>,
        #[serde(rename = "HorizontalAlignment")]
        horizontal_alignment: Option<HorizontalAlignment>,
        #[serde(rename = "isSubtle")]
        is_subtle: Option<bool>,
        #[serde(rename = "maxLines")]
        max_lines: Option<u64>,
        size: Option<Size>,
        weight: Option<Weight>,
        height: Option<Height>,
        // Inherited
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    Image {
        url: String,
        #[serde(rename = "altText")]
        alt_text: Option<String>,
        #[serde(rename = "backgroundColor")]
        background_color: Option<String>,
        width: Option<String>,
        height: Option<String>,
        #[serde(rename = "horizontalAlignment")]
        horizontal_alignment: Option<HorizontalAlignment>,
        #[serde(rename = "selectAction")]
        select_action: Option<Action>,
        size: Option<ImageSize>,
        style: Option<ImageStyle>,
        // Inherited
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    Media {
        sources: Vec<MediaSource>,
        poster: Option<String>,
        #[serde(rename = "altText")]
        alt_text: Option<String>,
        // Inherited
        height: Option<Height>,
        id: Option<String>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.Text")]
    InputText {
        id: String,
        placeholder: Option<String>,
        #[serde(rename = "isMultiline")]
        is_multiline: Option<bool>,
        #[serde(rename = "maxLength")]
        max_length: Option<u64>,
        style: Option<String>,
        value: Option<String>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.Number")]
    InputNumber {
        id: String,
        placeholder: Option<String>,
        max: Option<f64>,
        min: Option<f64>,
        value: Option<f64>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.Date")]
    InputDate {
        id: String,
        placeholder: Option<String>,
        max: Option<String>,
        min: Option<String>,
        value: Option<String>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.Time")]
    InputTime {
        id: String,
        placeholder: Option<String>,
        max: Option<String>,
        min: Option<String>,
        value: Option<String>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.Toggle")]
    InputToggle {
        id: String,
        placeholder: Option<String>,
        value: Option<String>,
        #[serde(rename = "valueOff")]
        value_off: Option<String>,
        #[serde(rename = "valueOn")]
        value_on: Option<String>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    #[serde(rename = "Input.ChoiceSet")]
    InputChoiceSet {
        placeholder: String,
        choices: Vec<Choice>,
        id: String,
        #[serde(rename = "isMultiSelect")]
        is_multi_select: Option<bool>,
        style: Option<ChoiceInputStyle>,
        value: Option<String>,
        // Inherited
        height: Option<Height>,
        separator: Option<bool>,
        spacing: Option<Spacing>,
    },

    ActionSet {
        actions: Vec<Action>,
        // Inherited,
        height: Option<Height>,
    },
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Column {
    items: Vec<CardElement>,
    #[serde(rename = "selectAction")]
    select_action: Option<Action>,
    style: Option<ContainerStyle>,
    #[serde(rename = "verticalContentAlignment")]
    vertical_content_alignment: Option<VerticalContentAlignment>,
    separator: Option<bool>,
    spacing: Option<Spacing>,
    width: Option<String>,
    // Inherited
    id: Option<String>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Fact {
    title: String,
    value: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct MediaSource {
    #[serde(rename = "mimeType")]
    mime_type: String,
    url: String,
}

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ContainerStyle {
    Default,
    Emphasis,
    Good,
    Attention,
    Warning,
    Accent,
}

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

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ChoiceInputStyle {
    Compact,
    Expanded,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum VerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TextInputStyle {
    Text,
    Tel,
    Url,
    Email,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Height {
    Auto,
    Stretch,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ImageStyle {
    Default,
    Person,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Weight {
    Default,
    Lighter,
    Bolder,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum Size {
    Default,
    Small,
    Medium,
    Large,
    ExtraLarge,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ImageSize {
    Auto,
    Stretch,
    Small,
    Medium,
    Large,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum SelectAction {
    Left,
    Center,
    Right,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Action {
    #[serde(rename = "Action.Submit")]
    Submit {
        data: Option<HashMap<String, String>>,
        title: Option<String>,
        #[serde(rename = "iconUrl")]
        icon_url: Option<String>,
    },
    #[serde(rename = "Action.OpenUrl")]
    OpenUrl {
        url: String,
        title: Option<String>,
        #[serde(rename = "iconUrl")]
        icon_url: Option<String>,
        data: Option<HashMap<String, String>>,
    },
    #[serde(rename = "Action.ShowCard")]
    ShowCard {
        card: AdaptiveCard,
        title: Option<String>,
        #[serde(rename = "iconUrl")]
        icon_url: Option<String>,
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
