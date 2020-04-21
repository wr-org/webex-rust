use serde_json::error::Error as SerdeError;
use hyper::StatusCode;
use tungstenite;

error_chain! {
    foreign_links {
        Io(std::io::Error);
        Json(SerdeError);
        UTF8(std::string::FromUtf8Error);
    }
    errors {
        Status(s: StatusCode) {
            description("HTTP Status Code")
            display("HTTP Status: '{}'", s)
        }

        Limited(s: StatusCode, t: Option<i64>) {
            description("Reached API Limits")
            display("{} Retry in: '{:?}'", s, t)
        }

        Tungstenite(e: tungstenite::Error, t: String) {
            description("Failed WS")
            display("{} {}", e, t)
        }
    }
}