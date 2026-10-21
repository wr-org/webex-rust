//! Style types for Adaptive Cards including colors, spacing, weights, and alignment options.

use serde::{Deserialize, Serialize};
use strum_macros::EnumString;

/// Color for text
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(ascii_case_insensitive)]
pub enum Color {
    Default,
    Dark,
    Light,
    Accent,
    Good,
    Warning,
    Attention,
}

/// Style hint for Container.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize, EnumString)]
#[strum(ascii_case_insensitive)]
pub enum ContainerStyle {
    Default,
    Emphasis,
    Good,
    Attention,
    Warning,
    Accent,
}

/// Controls the amount of spacing between this element and the preceding element.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, EnumString)]
#[serde(rename_all = "PascalCase")]
#[strum(ascii_case_insensitive)]
pub enum Spacing {
    #[serde(alias = "default")]
    Default,
    #[serde(alias = "none")]
    None,
    #[serde(alias = "small")]
    Small,
    #[serde(alias = "medium")]
    Medium,
    #[serde(alias = "large")]
    Large,
    #[serde(alias = "extraLarge")]
    ExtraLarge,
    #[serde(alias = "padding")]
    Padding,
}

/// Style for Input.ChoiceSet
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ChoiceInputStyle {
    Compact,
    Expanded,
}

/// Vertical content alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum VerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

/// Text input style
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum TextInputStyle {
    Text,
    Tel,
    Url,
    Email,
}

/// Height of element
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Height {
    Auto,
    Stretch,
}

/// Style hint for Image.
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum ImageStyle {
    Default,
    Person,
}

/// Weight of text
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum Weight {
    Default,
    Lighter,
    Bolder,
}

/// Font type
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum FontType {
    Default,
    Monospace,
}

/// Size of text
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Size {
    #[serde(alias = "Default")]
    Default,
    #[serde(alias = "Small")]
    Small,
    #[serde(alias = "Medium")]
    Medium,
    #[serde(alias = "Large")]
    Large,
    #[serde(alias = "ExtraLarge")]
    ExtraLarge,
}

/// Horizontal alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum HorizontalAlignment {
    Left,
    Center,
    Right,
}

/// Size of image (pixel width)
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ImageSize {
    #[serde(alias = "Auto")]
    Auto,
    #[serde(alias = "Stretch")]
    Stretch,
    #[serde(alias = "Small")]
    Small,
    #[serde(alias = "Medium")]
    Medium,
    #[serde(alias = "Large")]
    Large,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::de::DeserializeOwned;
    use std::fmt::Debug;

    #[test]
    fn styles_parse_case_insensitively() {
        assert_eq!("aCcEnT".parse::<Color>().unwrap(), Color::Accent);
        assert_eq!(
            "EMPHASIS".parse::<ContainerStyle>().unwrap(),
            ContainerStyle::Emphasis
        );
        assert_eq!(
            "extraLarge".parse::<Spacing>().unwrap(),
            Spacing::ExtraLarge
        );
        assert!("unknown".parse::<Color>().is_err());
    }

    fn check_wire_value<T: Serialize + DeserializeOwned + Eq + Debug>(
        value: &T,
        wire: &str,
        alias: &str,
    ) {
        assert_eq!(serde_json::to_value(value).unwrap(), wire);
        for input in [wire, alias] {
            assert_eq!(
                &serde_json::from_value::<T>(serde_json::json!(input)).unwrap(),
                value
            );
        }
    }

    #[test]
    fn styles_preserve_wire_values_and_aliases() {
        check_wire_value(&Size::Default, "default", "Default");
        check_wire_value(&Size::Small, "small", "Small");
        check_wire_value(&Size::Medium, "medium", "Medium");
        check_wire_value(&Size::Large, "large", "Large");
        check_wire_value(&Size::ExtraLarge, "extraLarge", "ExtraLarge");
        check_wire_value(&ImageSize::Auto, "auto", "Auto");
        check_wire_value(&ImageSize::Stretch, "stretch", "Stretch");
        check_wire_value(&ImageSize::Small, "small", "Small");
        check_wire_value(&ImageSize::Medium, "medium", "Medium");
        check_wire_value(&ImageSize::Large, "large", "Large");
        check_wire_value(&Spacing::Default, "Default", "default");
        check_wire_value(&Spacing::None, "None", "none");
        check_wire_value(&Spacing::Small, "Small", "small");
        check_wire_value(&Spacing::Medium, "Medium", "medium");
        check_wire_value(&Spacing::Large, "Large", "large");
        check_wire_value(&Spacing::ExtraLarge, "ExtraLarge", "extraLarge");
        check_wire_value(&Spacing::Padding, "Padding", "padding");
        check_wire_value(&VerticalContentAlignment::Top, "top", "top");
        check_wire_value(&VerticalContentAlignment::Center, "center", "center");
        check_wire_value(&VerticalContentAlignment::Bottom, "bottom", "bottom");
        check_wire_value(&ContainerStyle::Emphasis, "Emphasis", "Emphasis");
        check_wire_value(&ChoiceInputStyle::Compact, "Compact", "Compact");
        check_wire_value(&TextInputStyle::Text, "Text", "Text");
        check_wire_value(&ImageStyle::Person, "Person", "Person");
    }
}
