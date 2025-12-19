//! Card elements for building Adaptive Card content.

use serde::{Deserialize, Serialize};

use super::containers::{Choice, Column, Fact};
use super::styles::{
    ChoiceInputStyle, Color, ContainerStyle, FontType, Height, HorizontalAlignment, ImageSize,
    ImageStyle, Size, Spacing, TextInputStyle, VerticalContentAlignment, Weight,
};
use super::Action;

/// Represents the various types of elements that can be included in an Adaptive Card.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "type")]
pub enum CardElement {
    /// Containers group items together.
    Container {
        /// The card elements to render inside the Container.
        #[serde(default)]
        items: Vec<CardElement>,
        /// An Action that will be invoked when the Container is tapped or selected.
        #[serde(rename = "selectAction", skip_serializing_if = "Option::is_none")]
        select_action: Option<Action>,
        /// Style hint for Container.
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ContainerStyle>,
        /// Defines how the content should be aligned vertically within the container.
        #[serde(
            rename = "verticalContentAlignment",
            skip_serializing_if = "Option::is_none"
        )]
        vertical_content_alignment: Option<VerticalContentAlignment>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// `ColumnSet` divides a region into Columns, allowing elements to sit side-by-side.
    ColumnSet {
        /// The array of Columns to divide the region into.
        columns: Vec<Column>,
        /// An Action that will be invoked when the `ColumnSet` is tapped or selected.
        #[serde(rename = "selectAction", skip_serializing_if = "Option::is_none")]
        select_action: Option<Action>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// The `FactSet` element displays a series of facts (i.e. name/value pairs) in a tabular form.
    FactSet {
        /// The array of Fact's.
        facts: Vec<Fact>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// The `ImageSet` displays a collection of Images similar to a gallery.
    ImageSet {
        /// The array of Image elements to show.
        images: Vec<CardElement>,
        /// Controls the approximate size of each image. The physical dimensions will vary per host.
        #[serde(rename = "imageSize", skip_serializing_if = "Option::is_none")]
        image_size: Option<ImageSize>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Displays text, allowing control over font sizes, weight, and color.
    TextBlock {
        /// Text to display
        text: String,
        /// If true, allow text to wrap. Otherwise, text is clipped.
        #[serde(skip_serializing_if = "Option::is_none")]
        wrap: Option<bool>,
        /// Controls the color of `TextBlock` elements.
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<Color>,
        /// Controls the horizontal text alignment.
        #[serde(
            rename = "HorizontalAlignment",
            skip_serializing_if = "Option::is_none"
        )]
        horizontal_alignment: Option<HorizontalAlignment>,
        /// If true, displays text slightly toned down to appear less prominent.
        #[serde(rename = "isSubtle", skip_serializing_if = "Option::is_none")]
        is_subtle: Option<bool>,
        /// Specifies the maximum number of lines to display.
        #[serde(rename = "maxLines", skip_serializing_if = "Option::is_none")]
        max_lines: Option<u64>,
        /// Specifies the font type
        #[serde(rename = "fontType", skip_serializing_if = "Option::is_none")]
        font_type: Option<FontType>,
        /// Controls size of text.
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<Size>,
        /// Controls the weight of `TextBlock` elements.
        #[serde(skip_serializing_if = "Option::is_none")]
        weight: Option<Weight>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Displays an image.
    Image {
        /// The URL to the image.
        url: String,
        /// Alternate text describing the image.
        #[serde(rename = "altText", skip_serializing_if = "Option::is_none")]
        alt_text: Option<String>,
        /// Applies a background to a transparent image. This property will respect the image style.
        /// hex value of a color (e.g. #982374)
        #[serde(rename = "backgroundColor", skip_serializing_if = "Option::is_none")]
        background_color: Option<String>,
        /// The desired on-screen width of the image, ending in 'px'. E.g., 50px. This overrides the size property.
        #[serde(skip_serializing_if = "Option::is_none")]
        width: Option<String>,
        /// The desired height of the image. If specified as a pixel value, ending in 'px', E.g., 50px, the image will distort to fit that exact height. This overrides the size property.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<String>,
        /// Controls how this element is horizontally positioned within its parent.
        #[serde(
            rename = "horizontalAlignment",
            skip_serializing_if = "Option::is_none"
        )]
        horizontal_alignment: Option<HorizontalAlignment>,
        /// An Action that will be invoked when the Image is tapped or selected. Action.ShowCard is not supported.
        #[serde(rename = "selectAction", skip_serializing_if = "Option::is_none")]
        select_action: Option<Action>,
        /// Controls the approximate size of the image. The physical dimensions will vary per host.
        #[serde(skip_serializing_if = "Option::is_none")]
        size: Option<ImageSize>,
        /// Controls how this Image is displayed.
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ImageStyle>,
        /// A unique identifier associated with the item.
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Lets a user enter text.
    #[serde(rename = "Input.Text")]
    InputText {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no text has been input.
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// If true, allow multiple lines of input.
        #[serde(rename = "isMultiline", skip_serializing_if = "Option::is_none")]
        is_multiline: Option<bool>,
        /// Hint of maximum length characters to collect (may be ignored by some clients).
        #[serde(rename = "maxLength", skip_serializing_if = "Option::is_none")]
        max_length: Option<u64>,
        /// Text Input Style
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<TextInputStyle>,
        /// The inline action for the input. Typically displayed to the right of the input.
        #[serde(rename = "inlineAction", skip_serializing_if = "Option::is_none")]
        inline_action: Option<Action>,
        /// The initial value for this field.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Allows a user to enter a number.
    #[serde(rename = "Input.Number")]
    InputNumber {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no selection has been made.
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Hint of maximum value (may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<f64>,
        /// Hint of minimum value (may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<f64>,
        /// Initial value for this field.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<f64>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Lets a user choose a date.
    #[serde(rename = "Input.Date")]
    InputDate {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Description of the input desired. Displayed when no selection has been made.
        #[serde(skip_serializing_if = "Option::is_none")]
        placeholder: Option<String>,
        /// Hint of maximum value expressed in YYYY-MM-DD(may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<String>,
        /// Hint of minimum value expressed in YYYY-MM-DD(may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<String>,
        /// The initial value for this field expressed in YYYY-MM-DD.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Lets a user select a time.
    #[serde(rename = "Input.Time")]
    InputTime {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Hint of maximum value expressed in HH:MM (may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        max: Option<String>,
        /// Hint of minimum value expressed in HH:MM (may be ignored by some clients).
        #[serde(skip_serializing_if = "Option::is_none")]
        min: Option<String>,
        /// The initial value for this field expressed in HH:MM.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Lets a user choose between two options.
    #[serde(rename = "Input.Toggle")]
    InputToggle {
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// The initial selected value. If you want the toggle to be initially on, set this to the value of valueOn's value.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// The value when toggle is off
        #[serde(rename = "valueOff", skip_serializing_if = "Option::is_none")]
        value_off: Option<String>,
        /// The value when toggle is on
        #[serde(rename = "valueOn", skip_serializing_if = "Option::is_none")]
        value_on: Option<String>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
    },

    /// Allows a user to input a Choice.
    #[serde(rename = "Input.ChoiceSet")]
    InputChoiceSet {
        /// Choice options.
        choices: Vec<Choice>,
        /// Unique identifier for the value. Used to identify collected input when the Submit action is performed.
        id: String,
        /// Allow multiple choices to be selected.
        #[serde(rename = "isMultiSelect", skip_serializing_if = "Option::is_none")]
        is_multi_select: Option<bool>,
        /// Input Choice Style
        #[serde(skip_serializing_if = "Option::is_none")]
        style: Option<ChoiceInputStyle>,
        /// The initial choice (or set of choices) that should be selected. For multi-select, specify a comma-separated string of values.
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<String>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },

    /// Displays a set of actions.
    ActionSet {
        /// The array of Action elements to show.
        actions: Vec<Action>,
        /// Specifies the height of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        height: Option<Height>,
        /// Controls the horizontal text alignment.
        #[serde(
            rename = "HorizontalAlignment",
            skip_serializing_if = "Option::is_none"
        )]
        horizontal_alignment: Option<HorizontalAlignment>,
        /// When true, draw a separating line at the top of the element.
        #[serde(skip_serializing_if = "Option::is_none")]
        separator: Option<bool>,
        /// Controls the amount of spacing between this element and the preceding element.
        #[serde(skip_serializing_if = "Option::is_none")]
        spacing: Option<Spacing>,
    },
}

impl From<&Self> for CardElement {
    fn from(item: &Self) -> Self {
        item.clone()
    }
}

impl From<&mut Self> for CardElement {
    fn from(item: &mut Self) -> Self {
        item.clone()
    }
}

/// Functions for Card Element
impl CardElement {
    /// Create container
    #[must_use]
    pub const fn container() -> Self {
        Self::Container {
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
    #[must_use]
    pub fn add_element<T: Into<Self>>(&mut self, element: T) -> Self {
        if let Self::Container { items, .. } = self {
            items.push(element.into());
        }
        self.into()
    }

    /// Set Container Style
    #[must_use]
    pub fn set_container_style(&mut self, s: ContainerStyle) -> Self {
        if let Self::Container { style, .. } = self {
            *style = Some(s);
        }
        self.into()
    }

    /// Set container contents vertical alignment
    #[must_use]
    pub fn set_vertical_alignment(&mut self, align: VerticalContentAlignment) -> Self {
        if let Self::Container {
            vertical_content_alignment,
            ..
        } = self
        {
            *vertical_content_alignment = Some(align);
        }
        self.into()
    }

    /// Create input.Text
    #[must_use]
    pub fn input_text<T: Into<String>, S: Into<String>>(id: T, value: Option<S>) -> Self {
        Self::InputText {
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
    #[must_use]
    pub fn set_multiline(&mut self, s: bool) -> Self {
        if let Self::InputText { is_multiline, .. } = self {
            *is_multiline = Some(s);
        }
        self.into()
    }

    /// Create input.ChoiceSet
    #[must_use]
    pub fn input_choice_set<T: Into<String>, S: Into<String>>(id: T, value: Option<S>) -> Self {
        Self::InputChoiceSet {
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
    #[must_use]
    pub fn input_toggle<T: Into<String>>(id: T, value: bool) -> Self {
        Self::InputToggle {
            id: id.into(),
            value: Some(value.to_string()),
            value_off: None,
            value_on: None,
            height: None,
            separator: None,
            spacing: None,
            title: None,
        }
    }

    /// Set choiceSet Style
    #[must_use]
    pub fn set_style(&mut self, s: ChoiceInputStyle) -> Self {
        if let Self::InputChoiceSet { style, .. } = self {
            *style = Some(s);
        }
        self.into()
    }

    /// Set title Style
    #[must_use]
    pub fn set_title(&mut self, s: String) -> Self {
        if let Self::InputToggle { title, .. } = self {
            *title = Some(s);
        }
        self.into()
    }

    /// Set choiceSet Style
    #[must_use]
    pub fn set_multiselect(&mut self, b: bool) -> Self {
        if let Self::InputChoiceSet {
            is_multi_select, ..
        } = self
        {
            *is_multi_select = Some(b);
        }
        self.into()
    }

    /// Create textBlock
    ///
    /// # Arguments
    ///
    /// * `text` - Text to set to the new text block (Must implement `Into<String>`)
    #[must_use]
    pub fn text_block<T: Into<String>>(text: T) -> Self {
        Self::TextBlock {
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
    #[must_use]
    pub fn set_weight(&mut self, w: Weight) -> Self {
        if let Self::TextBlock { weight, .. } = self {
            *weight = Some(w);
        }
        self.into()
    }

    /// Set Text Font Type
    #[must_use]
    pub fn set_font(&mut self, f: FontType) -> Self {
        if let Self::TextBlock { font_type, .. } = self {
            *font_type = Some(f);
        }
        self.into()
    }

    /// Set Text Size
    #[must_use]
    pub fn set_size(&mut self, s: Size) -> Self {
        if let Self::TextBlock { size, .. } = self {
            *size = Some(s);
        }
        self.into()
    }

    /// Set Text Color
    #[must_use]
    pub fn set_color(&mut self, c: Color) -> Self {
        if let Self::TextBlock { color, .. } = self {
            *color = Some(c);
        }
        self.into()
    }

    /// Set Text wrap
    #[must_use]
    pub fn set_wrap(&mut self, w: bool) -> Self {
        if let Self::TextBlock { wrap, .. } = self {
            *wrap = Some(w);
        }
        self.into()
    }

    /// Set Text subtle
    #[must_use]
    pub fn set_subtle(&mut self, s: bool) -> Self {
        if let Self::TextBlock { is_subtle, .. } = self {
            *is_subtle = Some(s);
        }
        self.into()
    }

    /// Create factSet
    #[must_use]
    pub const fn fact_set() -> Self {
        Self::FactSet {
            facts: vec![],
            height: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Create image
    pub fn image<T: Into<String>>(url: T) -> Self {
        Self::Image {
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
    #[must_use]
    pub fn add_key_value<T: Into<String>, S: Into<String>>(&mut self, title: T, value: S) -> Self {
        match self {
            Self::FactSet { facts, .. } => facts.push(Fact {
                title: title.into(),
                value: value.into(),
            }),
            Self::InputChoiceSet { choices, .. } => choices.push(Choice {
                title: title.into(),
                value: value.into(),
            }),
            _ => {
                log::warn!("Card does not have key value type field");
            }
        }
        self.into()
    }

    /// Create columnSet
    #[must_use]
    pub const fn column_set() -> Self {
        Self::ColumnSet {
            columns: vec![],
            select_action: None,
            id: None,
            separator: None,
            spacing: None,
        }
    }

    /// Add column to columnSet
    #[must_use]
    pub fn add_column(&mut self, column: Column) -> Self {
        if let Self::ColumnSet { columns, .. } = self {
            columns.push(column);
        }
        self.into()
    }

    /// Set Separator
    #[must_use]
    pub fn set_separator(&mut self, s: bool) -> Self {
        match self {
            Self::TextBlock { separator, .. }
            | Self::FactSet { separator, .. }
            | Self::ColumnSet { separator, .. }
            | Self::Image { separator, .. }
            | Self::InputChoiceSet { separator, .. }
            | Self::ActionSet { separator, .. }
            | Self::InputText { separator, .. }
            | Self::InputToggle { separator, .. } => {
                *separator = Some(s);
            }
            _ => {
                log::warn!("Card does not have separator field");
            }
        }
        self.into()
    }

    /// Set Placeholder
    #[must_use]
    pub fn set_placeholder(&mut self, s: Option<String>) -> Self {
        match self {
            Self::InputText { placeholder, .. }
            | Self::InputNumber { placeholder, .. }
            | Self::InputDate { placeholder, .. } => {
                *placeholder = s;
            }
            _ => {
                log::warn!("Card does not have placeholder field");
            }
        }
        self.into()
    }

    /// Set Horizontal Alignment
    #[must_use]
    pub fn set_horizontal_alignment(&mut self, alignment: HorizontalAlignment) -> Self {
        match self {
            Self::TextBlock {
                horizontal_alignment,
                ..
            }
            | Self::Image {
                horizontal_alignment,
                ..
            }
            | Self::ActionSet {
                horizontal_alignment,
                ..
            } => {
                *horizontal_alignment = Some(alignment);
            }
            _ => {
                log::warn!("Card does not have horizontal alignment field");
            }
        }
        self.into()
    }

    /// Set Spacing
    #[must_use]
    pub fn set_spacing(&mut self, s: Spacing) -> Self {
        match self {
            Self::TextBlock { spacing, .. }
            | Self::FactSet { spacing, .. }
            | Self::ColumnSet { spacing, .. }
            | Self::Image { spacing, .. }
            | Self::InputChoiceSet { spacing, .. }
            | Self::ActionSet { spacing, .. }
            | Self::InputText { spacing, .. } => {
                *spacing = Some(s);
            }
            _ => {
                log::warn!("Card does not have spacing field");
            }
        }
        self.into()
    }

    /// Create actionSet
    #[must_use]
    pub const fn action_set() -> Self {
        Self::ActionSet {
            actions: vec![],
            height: None,
            horizontal_alignment: None,
            separator: None,
            spacing: None,
        }
    }

    /// Add action to actionSet
    #[must_use]
    pub fn add_action_to_set(&mut self, action: Action) -> Self {
        if let Self::ActionSet { actions, .. } = self {
            actions.push(action);
        }
        self.into()
    }
}
