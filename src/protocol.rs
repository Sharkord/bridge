use crate::PROTOCOL_VERSION;
use serde::Deserialize;
use serde_json::json;

// what a client sends. a bind is an id Sharkord chose and the combo it wants
#[derive(Debug, Deserialize)]
pub struct RawBinding {
    pub id: String,
    pub combo: String,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ClientMsg {
    Hello {
        v: u32,
        token: String,
        #[serde(rename = "serverId")]
        server_id: String,
        #[serde(rename = "serverName", default)]
        server_name: String,
    },
    Bindings {
        bindings: Vec<RawBinding>,
    },
}

// what the bridge sends back, built in one place so the wire format is too
pub fn ready() -> String {
    json!({
        "type": "ready",
        "v": PROTOCOL_VERSION,
        "capabilities": { "keyUp": true },
        "platform": std::env::consts::OS,
    })
    .to_string()
}

pub fn event(id: &str, down: bool) -> String {
    let phase = if down { "down" } else { "up" };

    json!({ "type": "event", "id": id, "phase": phase }).to_string()
}

pub fn error(id: &str, reason: &str) -> String {
    json!({ "type": "error", "id": id, "reason": reason }).to_string()
}

pub fn replaced() -> String {
    json!({ "type": "replaced" }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn parse(message: &str) -> Value {
        serde_json::from_str(message).unwrap()
    }

    #[test]
    fn builds_the_messages_the_client_expects() {
        assert_eq!(
            parse(&event("core:push-to-talk", true)),
            json!({ "type": "event", "id": "core:push-to-talk", "phase": "down" })
        );
        assert_eq!(parse(&event("a", false))["phase"], "up");
        assert_eq!(
            parse(&error("a", "invalid combo")),
            json!({ "type": "error", "id": "a", "reason": "invalid combo" })
        );
        assert_eq!(parse(&replaced()), json!({ "type": "replaced" }));
    }

    #[test]
    fn ready_announces_the_version_and_what_this_bridge_can_do() {
        let ready = parse(&ready());

        assert_eq!(ready["type"], "ready");
        assert_eq!(ready["v"], PROTOCOL_VERSION);
        assert_eq!(ready["capabilities"], json!({ "keyUp": true }));
    }

    #[test]
    fn parses_what_a_client_sends() {
        let hello = r#"{"type":"hello","v":1,"token":"t","serverId":"s"}"#;

        assert!(matches!(
            serde_json::from_str::<ClientMsg>(hello),
            Ok(ClientMsg::Hello { v: 1, .. })
        ));
        assert!(serde_json::from_str::<ClientMsg>(r#"{"type":"nope"}"#).is_err());
    }
}
