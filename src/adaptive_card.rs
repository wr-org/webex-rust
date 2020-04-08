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

impl AdaptiveCard {
    /// Create new adaptive card with mandatory defaults
    pub fn new() -> Self {
        AdaptiveCard {
            card_type: "AdaptiveCard".to_string(),
            version: "1.1".to_string(),
            body: None,
            actions: None,
            select_action: None,
            fallback_text: None,
            min_height: None,
            lang: None,
            schema: "http://adaptivecards.io/schemas/adaptive-card.json".to_string(),
        }
    }

    /// Adds Element to body
    ///
    /// # Arguments
    ///
    /// * `card` - CardElement to add
    pub fn add_body<T: Into<CardElement>>(&mut self, card: T) -> Self {
        self.body = Some(match self.body.clone() {
            None => { vec![card.into()] }
            Some(mut body) => {
                body.push(card.into());
                body
            }
        });
        self.into()
    }

    /// Adds Actions
    ///
    /// # Arguments
    ///
    /// * `action` - Action to add
    pub fn add_action<T: Into<Action>>(&mut self, a: T) -> Self {
        self.actions = Some(match self.actions.clone() {
            None => { vec![a.into()] }
            Some(mut action) => {
                action.push(a.into());
                action
            }
        });
        self.into()
    }
}

impl From<&AdaptiveCard> for AdaptiveCard {
    fn from(item: &AdaptiveCard) -> Self {
        item.clone()
    }
}

impl From<&mut AdaptiveCard> for AdaptiveCard {
    fn from(item: &mut AdaptiveCard) -> Self {
        item.clone()
    }
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
        facts: Vec<Fact>,
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
        /// Specifies the font type
        #[serde(rename = "fontType")]
        font_type: Option<FontType>,
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
        /// The inline action for the input. Typically displayed to the right of the input.
        #[serde(rename = "inlineAction")]
        inline_action: Option<Action>,
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

impl From<&CardElement> for CardElement {
    fn from(item: &CardElement) -> Self {
        item.clone()
    }
}

impl From<&mut CardElement> for CardElement {
    fn from(item: &mut CardElement) -> Self {
        item.clone()
    }
}

/// Functions for Card Element
impl CardElement {
    /// Create container
    pub fn container() -> Self {
        CardElement::Container {
            items: vec![],
            select_action: None,
            style: None,
            vertical_content_alignment: None,
            height: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Add element to Container
    pub fn add_element<T: Into<CardElement>>(&mut self, element: T) -> Self {
        match self {
            CardElement::Container {
                items, select_action: _, style: _, vertical_content_alignment: _, height: _, id: _, separator: _, spacing: _
            } => { items.push(element.into()) }
            _ => {}
        }
        self.into()
    }

    /// Set Container Style
    pub fn set_container_style(&mut self, s: ContainerStyle) -> Self {
        if let CardElement::Container {
            items: _, select_action: _, style, vertical_content_alignment: _, height: _, id: _, separator: _, spacing: _
        } = self { *style = Some(s); }
        self.into()
    }
    /// Create input.Text
    pub fn input_text<T: Into<String>, S: Into<String>>(id: T, value: Option<S>) -> Self {
        CardElement::InputText {
            id: id.into(),
            placeholder: None,
            is_multiline: None,
            max_length: None,
            style: None,
            inline_action: None,
            value: value.map(Into::into),
            height: None,
            separator: None,
            spacing: None,
        }
    }

    /// Set Text Input Multiline
    pub fn set_multiline(&mut self, s: bool) -> Self {
        if let CardElement::InputText {
            id:_, placeholder:_, is_multiline, max_length:_, style:_, inline_action:_, value:_, height:_, separator:_, spacing:_
        } = self { *is_multiline = Some(s); }
        self.into()
    }

    /// Create input.ChoiceSet
    pub fn input_choice_set<T: Into<String>, S: Into<String>>(id: T, value: Option<S>) -> Self {
        CardElement::InputChoiceSet {
            choices: vec![],
            id: id.into(),
            is_multi_select: None,
            style: None,
            value: value.map(Into::into),
            height: None,
            separator: None,
            spacing: None,
        }
    }

    /// Create input.Toggle
    pub fn input_toggle<T: Into<String>>(id: T, value: bool) -> Self {
        CardElement::InputToggle {
            id: id.into(),
            value: Some(value.to_string()),
            value_off: None,
            value_on: None,
            height: None,
            separator: None,
            spacing: None,
        }
    }

    /// Set choiceSet Style
    pub fn set_style(&mut self, s: ChoiceInputStyle) -> Self {
        if let CardElement::InputChoiceSet {
            choices: _, id: _, is_multi_select: _, style, value: _, height: _, separator: _, spacing: _
        } = self { *style = Some(s); }
        self.into()
    }

    /// Set choiceSet Style
    pub fn set_multiselect(&mut self, b: bool) -> Self {
        if let CardElement::InputChoiceSet {
            choices: _, id: _, is_multi_select, style: _, value: _, height: _, separator: _, spacing: _
        } = self { *is_multi_select = Some(b); }
        self.into()
    }

    /// Create textBlock
    ///
    /// # Arguments
    ///
    /// * `text` - Text to set to the new text block(Must implement Into<String>
    pub fn text_block<T: Into<String>>(text: T) -> Self {
        CardElement::TextBlock {
            text: text.into(),
            wrap: None,
            color: None,
            horizontal_alignment: None,
            is_subtle: None,
            max_lines: None,
            font_type: None,
            size: None,
            weight: None,
            height: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Set Text Weight
    pub fn set_weight(&mut self, w: Weight) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap: _, color: _, horizontal_alignment: _, is_subtle: _, max_lines: _, font_type: _, size: _, weight, height: _, id: _, separator: _, spacing: _
        } = self { *weight = Some(w); }
        self.into()
    }

    /// Set Text Font Type
    pub fn set_font(&mut self, f: FontType) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap: _, color: _, horizontal_alignment: _, is_subtle: _, max_lines: _, font_type, size: _, weight: _, height: _, id: _, separator: _, spacing: _
        } = self { *font_type = Some(f); }
        self.into()
    }

    /// Set Text Size
    pub fn set_size(&mut self, s: Size) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap: _, color: _, horizontal_alignment: _, is_subtle: _, font_type: _, max_lines: _, size, weight: _, height: _, id: _, separator: _, spacing: _
        } = self { *size = Some(s); }
        self.into()
    }

    /// Set Text Color
    pub fn set_color(&mut self, c: Color) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap: _, color, horizontal_alignment: _, font_type: _, is_subtle: _, max_lines: _, size: _, weight: _, height: _, id: _, separator: _, spacing: _
        } = self { *color = Some(c); }
        self.into()
    }

    /// Set Text wrap
    pub fn set_wrap(&mut self, w: bool) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap, color: _, horizontal_alignment: _, font_type: _, is_subtle: _, max_lines: _, size: _, weight: _, height: _, id: _, separator: _, spacing: _
        } = self { *wrap = Some(w); }
        self.into()
    }

    /// Set Text subtle
    pub fn set_subtle(&mut self, s: bool) -> Self {
        if let CardElement::TextBlock {
            text: _, wrap: _, color: _, horizontal_alignment: _, font_type: _, is_subtle, max_lines: _, size: _, weight: _, height: _, id: _, separator: _, spacing: _
        } = self { *is_subtle = Some(s); }
        self.into()
    }

    /// Create factSet
    pub fn fact_set() -> CardElement {
        CardElement::FactSet {
            facts: vec![],
            height: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Create image
    pub fn image<T: Into<String>>(url: T) -> CardElement {
        CardElement::Image {
            url: url.into(),
            alt_text: None,
            background_color: None,
            width: None,
            height: None,
            horizontal_alignment: None,
            select_action: None,
            size: None,
            style: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Add fact to factSet
    pub fn add_key_value<T: Into<String>, S: Into<String>>(&mut self, title: T, value: S) -> Self {
        match self {
            CardElement::FactSet {
                facts, height: _, id: _, separator: _, spacing: _,
            } => { facts.push(Fact { title: title.into(), value: value.into() }) }
            CardElement::InputChoiceSet { choices, id: _, is_multi_select: _, style: _, value: _, height: _, separator: _, spacing: _ } => {
                choices.push(Choice { title: title.into(), value: value.into() })
            }
            _ => {}
        }
        self.into()
    }

    /// Create columnSet
    pub fn column_set() -> CardElement {
        CardElement::ColumnSet {
            columns: vec![],
            select_action: None,
            height: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Add column to columnSet
    pub fn add_column(&mut self, column: Column) -> Self {
        match self {
            CardElement::ColumnSet {
                columns, select_action: _, height: _, id: _, separator: _, spacing: _
            } => { columns.push(column) }
            _ => {}
        }
        self.into()
    }

    /// Set Separator
    pub fn set_separator(&mut self, s: bool) -> Self {
        match self {
            CardElement::TextBlock {
                text: _, wrap: _, color: _, horizontal_alignment: _, font_type: _, is_subtle: _, max_lines: _, size: _, weight: _, height: _, id: _, separator, spacing: _
            } => { *separator = Some(s); }
            CardElement::FactSet {
                facts: _, height: _, id: _, separator, spacing: _, } => { *separator = Some(s); }
            CardElement::ColumnSet {
                columns: _, select_action: _, height: _, id: _, separator, spacing: _
            } => { *separator = Some(s); }
            CardElement::Image {
                url: _, alt_text: _, background_color: _, width: _, height: _, horizontal_alignment: _, select_action: _, size: _, style: _, id: _, separator, spacing: _
            } => { *separator = Some(s); }
            CardElement::InputChoiceSet {
                choices: _, id: _, is_multi_select: _, style: _, value: _, height: _, separator, spacing: _
            } => { *separator = Some(s); }
            CardElement::InputText {
                id: _, placeholder: _, is_multiline: _, max_length: _, style: _, inline_action: _, value: _, height: _, separator, spacing: _
            } => { *separator = Some(s); }
            _ => {}
        }
        self.into()
    }

    /// Set Spacing
    pub fn set_spacing(&mut self, s: Spacing) -> Self {
        match self {
            CardElement::TextBlock {
                text: _, wrap: _, color: _, horizontal_alignment: _, font_type: _, is_subtle: _, max_lines: _, size: _, weight: _, height: _, id: _, separator: _, spacing
            } => { *spacing = Some(s); }
            CardElement::FactSet {
                facts: _, height: _, id: _, separator: _, spacing,
            } => { *spacing = Some(s); }
            CardElement::ColumnSet {
                columns: _, select_action: _, height: _, id: _, separator: _, spacing
            } => { *spacing = Some(s); }
            CardElement::Image {
                url: _, alt_text: _, background_color: _, width: _, height: _, horizontal_alignment: _, select_action: _, size: _, style: _, id: _, separator: _, spacing
            } => { *spacing = Some(s); }
            CardElement::InputChoiceSet {
                choices: _, id: _, is_multi_select: _, style: _, value: _, height: _, separator: _, spacing
            } => { *spacing = Some(s); }
            CardElement::InputText {
                id: _, placeholder: _, is_multiline: _, max_length: _, style: _, inline_action: _, value: _, height: _, separator: _, spacing
            } => { *spacing = Some(s); }
            _ => {}
        }
        self.into()
    }

    /// Create actionSet
    pub fn action_set() -> CardElement {
        CardElement::ActionSet {
            actions: vec![],
            height: None,
        }
    }

    /// Add action to actionSet
    pub fn add_action_to_set(&mut self, action: Action) -> Self {
        match self {
            CardElement::ActionSet {
                actions, height: _
            } => { actions.push(action) }
            _ => {}
        }
        self.into()
    }
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

impl From<&Column> for Column {
    fn from(item: &Column) -> Self {
        item.clone()
    }
}

impl From<&mut Column> for Column {
    fn from(item: &mut Column) -> Self {
        item.clone()
    }
}

impl Column {
    /// Creates new Column
    pub fn new() -> Self {
        Column {
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
    pub fn add_element(&mut self, item: CardElement) -> Self {
        self.items.push(item);
        self.into()
    }

    /// Sets separator
    pub fn set_separator(&mut self, s: bool) -> Self {
        self.separator = Some(s);
        self.into()
    }

    /// Sets VerticalContentAlignment
    pub fn set_vertical_alignment(&mut self, s: VerticalContentAlignment) -> Self {
        self.vertical_content_alignment = Some(s);
        self.into()
    }

    /// Sets width
    pub fn set_width<T: Into<String>>(&mut self, s: T) -> Self {
        self.width = Some(s.into());
        self.into()
    }
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

/// Type of font to use for rendering
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum FontType {
    Default,
    Monospace,
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
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        style: Option<ActionStyle>,
    },
    /// When invoked, show the given url either by launching it in an external web browser or showing within an embedded web browser.
    #[serde(rename = "Action.OpenUrl")]
    OpenUrl {
        /// The URL to open.
        url: String,
        /// Label for button or link that represents this action.
        title: Option<String>,
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        style: Option<ActionStyle>,
    },
    /// Defines an AdaptiveCard which is shown to the user when the button or link is clicked.
    #[serde(rename = "Action.ShowCard")]
    ShowCard {
        /// The Adaptive Card to show.
        card: AdaptiveCard,
        /// Label for button or link that represents this action.
        title: Option<String>,
        /// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
        style: Option<ActionStyle>,
    },
}

/// Controls the style of an Action, which influences how the action is displayed, spoken, etc.
#[allow(missing_docs)]
#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ActionStyle {
    /// Action is displayed as normal
    Default,
    /// Action is displayed with a positive style (typically the button becomes accent color)
    Positive,
    /// Action is displayed with a destructive style (typically the button becomes red)
    Destructive,
}

/// Describes a choice for use in a ChoiceSet.
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Choice {
    /// Text to display.
    pub title: String,
    /// The raw value for the choice. **NOTE:** do not use a , in the value, since a ChoiceSet with isMultiSelect set to true returns a comma-delimited string of choice values.
    pub value: String,
}