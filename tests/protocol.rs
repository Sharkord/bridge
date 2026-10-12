use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use sharkord_bridge::hub::{self, HubMsg};
use sharkord_bridge::keymap::{InputEvent, Phase};
use sharkord_bridge::server;
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

type Client = WebSocketStream<MaybeTlsStream<TcpStream>>;

const TOKEN: &str = "secret";

async fn start() -> (String, UnboundedSender<HubMsg>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("ws://{}", listener.local_addr().unwrap());
    let (hub_tx, hub_rx) = unbounded_channel();

    tokio::spawn(hub::run(hub_rx, None, false));
    tokio::spawn(server::serve(listener, Arc::from(TOKEN), hub_tx.clone()));

    (url, hub_tx)
}

async fn send(client: &mut Client, value: Value) {
    client
        .send(Message::Text(value.to_string().into()))
        .await
        .unwrap();
}

async fn next_json(client: &mut Client) -> Value {
    let message = timeout(Duration::from_secs(2), client.next())
        .await
        .expect("timed out waiting for a message")
        .expect("socket closed")
        .expect("socket error");

    match message {
        Message::Text(text) => serde_json::from_str(&text).unwrap(),
        other => panic!("unexpected message {other:?}"),
    }
}

async fn is_closed(client: &mut Client) -> bool {
    matches!(
        timeout(Duration::from_secs(2), client.next()).await,
        Ok(None) | Ok(Some(Err(_))) | Ok(Some(Ok(Message::Close(_))))
    )
}

async fn hello(url: &str, token: &str, server_id: &str) -> Client {
    let (mut client, _) = connect_async(url).await.unwrap();

    send(
        &mut client,
        json!({ "type": "hello", "v": 1, "token": token, "serverId": server_id, "serverName": "Test" }),
    )
    .await;

    client
}

#[tokio::test]
async fn pairs_syncs_binds_and_delivers_events() {
    let (url, hub) = start().await;
    let mut client = hello(&url, TOKEN, "srv-1").await;

    let ready = next_json(&mut client).await;

    assert_eq!(ready["type"], "ready");
    assert_eq!(ready["v"], 1);
    assert_eq!(ready["capabilities"], json!({ "keyUp": true }));

    send(
        &mut client,
        json!({ "type": "bindings", "bindings": [{ "id": "core:push-to-talk", "combo": "F8" }] }),
    )
    .await;

    // bindings are handled by the hub task, so give it a moment to apply them
    tokio::time::sleep(Duration::from_millis(100)).await;

    hub.send(HubMsg::Input(InputEvent {
        token: "F8",
        phase: Phase::Down,
    }))
    .unwrap();
    hub.send(HubMsg::Input(InputEvent {
        token: "F8",
        phase: Phase::Up,
    }))
    .unwrap();

    assert_eq!(
        next_json(&mut client).await,
        json!({ "type": "event", "id": "core:push-to-talk", "phase": "down" })
    );
    assert_eq!(
        next_json(&mut client).await,
        json!({ "type": "event", "id": "core:push-to-talk", "phase": "up" })
    );
}

#[tokio::test]
async fn refuses_a_wrong_token() {
    let (url, _hub) = start().await;
    let mut client = hello(&url, "nope", "srv-1").await;

    assert!(is_closed(&mut client).await);
}

#[tokio::test]
async fn refuses_an_unknown_protocol_version() {
    let (url, _hub) = start().await;
    let (mut client, _) = connect_async(&url).await.unwrap();

    send(
        &mut client,
        json!({ "type": "hello", "v": 99, "token": TOKEN, "serverId": "srv-1" }),
    )
    .await;

    assert!(is_closed(&mut client).await);
}

#[tokio::test]
async fn refuses_anything_before_hello() {
    let (url, _hub) = start().await;
    let (mut client, _) = connect_async(&url).await.unwrap();

    send(&mut client, json!({ "type": "bindings", "bindings": [] })).await;

    assert!(is_closed(&mut client).await);
}

#[tokio::test]
async fn a_second_connection_for_a_server_replaces_the_first() {
    let (url, _hub) = start().await;
    let mut old = hello(&url, TOKEN, "srv-1").await;

    assert_eq!(next_json(&mut old).await["type"], "ready");

    let mut new = hello(&url, TOKEN, "srv-1").await;

    assert_eq!(next_json(&mut new).await["type"], "ready");
    assert_eq!(next_json(&mut old).await["type"], "replaced");
    assert!(is_closed(&mut old).await);
}
