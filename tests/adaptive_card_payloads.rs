use serde_json::{json, Value};
use webex::adaptive_card::{
    Action, ActionStyle, AdaptiveCard, CardElement, HorizontalAlignment, Spacing,
};

#[test]
fn text_alignment_uses_the_schema_property_and_accepts_legacy_input() {
    let text = CardElement::text_block("Aligned text")
        .set_horizontal_alignment(HorizontalAlignment::Center);
    let expected = json!({
        "type": "TextBlock",
        "text": "Aligned text",
        "horizontalAlignment": "Center"
    });
    assert_eq!(serde_json::to_value(text).unwrap(), expected);
    for property in ["horizontalAlignment", "HorizontalAlignment"] {
        let legacy = json!({
            "type": "TextBlock", "text": "Aligned text", property: "Center"
        });
        let text: CardElement = serde_json::from_value(legacy).unwrap();
        assert_eq!(serde_json::to_value(text).unwrap(), expected);
    }
}

#[test]
fn image_alignment_still_uses_the_schema_property() {
    let image = CardElement::image("https://example.com/image.png")
        .set_horizontal_alignment(HorizontalAlignment::Right);
    assert_eq!(
        serde_json::to_value(image).unwrap(),
        json!({"type": "Image", "url": "https://example.com/image.png", "horizontalAlignment": "Right"})
    );
}

#[test]
fn nested_action_set_produces_a_versioned_portable_payload() {
    let actions = CardElement::action_set()
        .add_action_to_set(Action::Submit {
            data: None,
            title: Some("Send".to_string()),
            style: Some(ActionStyle::Positive),
        })
        .set_separator(true)
        .set_spacing(Spacing::Medium);
    let container = CardElement::container().add_element(actions);
    let mut card = AdaptiveCard::new();
    let _ = card.add_body(container);
    let expected: Value =
        serde_json::from_str(include_str!("fixtures/action-set-card.json")).unwrap();
    let payload = serde_json::to_value(card).unwrap();
    assert_eq!(payload, expected);
    // Unsupported ActionSet alignment is not emitted even if its setter is called.
    let actions: CardElement =
        serde_json::from_value(payload["body"][0]["items"][0].clone()).unwrap();
    let mut actions = actions;
    let _ = actions.set_horizontal_alignment(HorizontalAlignment::Center);
    assert_eq!(
        serde_json::to_value(actions).unwrap(),
        payload["body"][0]["items"][0]
    );
}

#[test]
fn missing_versions_default_to_1_2_and_explicit_versions_are_preserved() {
    let card: AdaptiveCard = serde_json::from_value(json!({"type": "AdaptiveCard"})).unwrap();
    assert_eq!(card.version, "1.2");
    for version in ["1.0", "1.1", "1.2", "1.3"] {
        let card: AdaptiveCard = serde_json::from_value(json!({
            "type": "AdaptiveCard", "version": version
        }))
        .unwrap();
        assert_eq!(serde_json::to_value(card).unwrap()["version"], version);
    }
}
