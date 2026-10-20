//! Person-related types for the Webex API.

use serde::{Deserialize, Serialize};

/// Information about a Webex Teams person/user.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Person {
    /// A unique identifier for the person.
    pub id: String,
    /// The email addresses of the person.
    pub emails: Vec<String>,
    /// Phone numbers for the person.
    pub phone_numbers: Option<Vec<PhoneNumber>>,
    /// The full name of the person.
    #[serde(rename = "displayName")]
    pub display_name: String,
    /// The nickname of the person if configured. If no nickname is configured for the person, this field will not be present.
    pub nick_name: Option<String>,
    /// The first name of the person.
    pub first_name: Option<String>,
    /// The last name of the person.
    pub last_name: Option<String>,
    /// The URL to the person's avatar in PNG format.
    pub avatar: Option<String>,
    /// The ID of the organization to which this person belongs.
    #[serde(rename = "orgId")]
    pub org_id: String,
    /// The date and time the person was created.
    pub created: String,
    /// The date and time of the person's last activity within Webex Teams.
    pub last_activity: String,
    /// The current presence status of the person.
    ///
    /// active - active within the last 10 minutes
    /// call - the user is in a call
    /// `DoNotDisturb` - the user has manually set their status to "Do Not Disturb"
    /// inactive - last activity occurred more than 10 minutes ago
    /// meeting - the user is in a meeting
    /// `OutOfOffice` - the user or a Hybrid Calendar service has indicated that they are "Out of Office"
    /// pending - the user has never logged in; a status cannot be determined
    /// presenting - the user is sharing content
    /// unknown - the user's status could not be determined
    pub status: String,
    /// The type of person account, such as person or bot.
    ///
    /// person- account belongs to a person
    /// bot - account is a bot user
    /// appuser - account is a guest user
    #[serde(rename = "type")]
    pub person_type: String,
}

/// Phone number information for a person.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default)]
pub struct PhoneNumber {
    /// Phone number type
    #[serde(rename = "type")]
    pub number_type: String,
    /// Phone number
    pub value: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn person_preserves_camel_case_fields() {
        let input = json!({
            "id": "person-id",
            "emails": ["person@example.com"],
            "phoneNumbers": [{"type": "work", "value": "123456"}],
            "displayName": "Test Person",
            "nickName": "Tester",
            "firstName": "Test",
            "lastName": "Person",
            "avatar": "https://example.com/avatar.png",
            "orgId": "org-id",
            "created": "2026-01-01T00:00:00Z",
            "lastActivity": "2026-10-20T12:00:00Z",
            "status": "active",
            "type": "person"
        });
        let person: Person = serde_json::from_value(input.clone()).unwrap();
        assert_eq!(person.first_name.as_deref(), Some("Test"));
        assert_eq!(person.last_activity, "2026-10-20T12:00:00Z");
        assert_eq!(person.phone_numbers.as_ref().unwrap()[0].value, "123456");
        assert_eq!(serde_json::to_value(person).unwrap(), input);
    }
}
