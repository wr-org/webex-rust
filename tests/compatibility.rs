use webex::{adaptive_card::AdaptiveCard, Attachment};

#[test]
fn attachment_supports_default_struct_update() {
    let attachment = Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        ..Attachment::default()
    };
    assert_eq!(attachment.content, AdaptiveCard::new());
}
