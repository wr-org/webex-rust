use webex::{adaptive_card::AdaptiveCard, Attachment};

#[test]
fn attachment_supports_default_struct_update() {
    let attachment = Attachment {
        content_type: "application/vnd.microsoft.card.adaptive".to_string(),
        ..Attachment::default()
    };
    assert_eq!(attachment.content, AdaptiveCard::new());
}

// This deliberately has no wildcard: adding variants or #[non_exhaustive] to
// the existing public Error enum breaks unchanged downstream exhaustive matches.
#[test]
fn error_remains_exhaustively_matchable() {
    use webex::error::Error;
    let error = Error::Api("example");
    let kind = match error {
        Error::Io(_) => "io",
        Error::Json(_) => "json",
        Error::FormEncoding(_) => "form",
        Error::UTF8(_) => "utf8",
        Error::Reqwest(_) => "http",
        Error::Closed(_) => "closed",
        Error::Status(_) => "status",
        Error::StatusText(_, _) => "status text",
        Error::Limited(_, _) => "limited",
        Error::Tungstenite(_, _) => "websocket",
        Error::Api(_) => "api",
        Error::Authentication => "authentication",
        Error::UserError(_) => "user",
        Error::Other(_) => "other",
    };
    assert_eq!(kind, "api");
}

#[test]
fn original_list_callers_keep_their_error_type() {
    // Compile an unchanged caller using ? on each pre-existing list method.
    async fn original_caller(webex: &webex::Webex) -> Result<(), webex::error::Error> {
        let _: Vec<webex::Room> = webex.list().await?;
        let _: Vec<webex::Room> = webex
            .list_with_params(webex::RoomListParams::default())
            .await?;
        let _: Vec<webex::Room> = webex.get_all_rooms().await?;
        Ok(())
    }
    let _ = original_caller;
}
