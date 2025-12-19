//! Style types for Adaptive Cards including colors, spacing, weights, and alignment options.

use serde::{Deserialize, Serialize};

/// Color for text
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
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
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum Spacing {
    Default,
    None,
    Small,
    Medium,
    Large,
    ExtraLarge,
    Padding,
}

/// Style for Input.ChoiceSet
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChoiceInputStyle {
    Compact,
    Expanded,
}

/// Vertical content alignment
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum VerticalContentAlignment {
    Top,
    Center,
    Bottom,
}

/// Text input style
#[allow(missing_docs)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "PascalCase")]
pub enum Size {
    Default,
    Small,
    Medium,
    Large,
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
#[serde(rename_all = "PascalCase")]
pub enum ImageSize {
    Auto,
    Stretch,
    Small,
    Medium,
    Large,
}
