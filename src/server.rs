use crate::hub::{server_label, ConnId, HubMsg};
use crate::protocol::{self, ClientMsg};
use crate::{log_debug, log_info, log_warn, PROTOCOL_VERSION};
use futures_util::{SinkExt, StreamExt};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use tokio_tungstenite::tungstenite::Message;

const HELLO_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_MESSAGE_BYTES: usize = 64 * 1024;

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn tokens_match(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub async fn serve(listener: TcpListener, token: Arc<str>, hub: UnboundedSender<HubMsg>) {
    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };

        tokio::spawn(handle(stream, token.clone(), hub.clone()));
    }
}

async fn handle(stream: TcpStream, token: Arc<str>, hub: UnboundedSender<HubMsg>) {
    let mut config = WebSocketConfig::default();

    config.max_message_size = Some(MAX_MESSAGE_BYTES);
    config.max_frame_size = Some(MAX_MESSAGE_BYTES);

    let Ok(mut ws) = tokio_tungstenite::accept_async_with_config(stream, Some(config)).await else {
        log_debug!("a connection failed the websocket handshake");

        return;
    };

    let first = match timeout(HELLO_TIMEOUT, ws.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => text,
        Ok(_) => {
            log_warn!("rejected a connection: it did not start with a text message");

            return;
        }
        Err(_) => {
            log_warn!("rejected a connection: no hello within {HELLO_TIMEOUT:?}");

            return;
        }
    };

    let Ok(ClientMsg::Hello {
        v,
        token: given,
        server_id,
        server_name,
    }) = serde_json::from_str::<ClientMsg>(&first)
    else {
        log_warn!("rejected a connection: the first message was not a valid hello");

        return;
    };

    if v != PROTOCOL_VERSION {
        log_warn!(
            "rejected {server_name}: protocol version {v}, this bridge speaks {PROTOCOL_VERSION}"
        );

        return;
    }

    if !tokens_match(&given, &token) {
        log_warn!("rejected {server_name}: wrong pairing token");

        return;
    }

    if server_id.is_empty() {
        log_warn!("rejected {server_name}: no server id");

        return;
    }

    let id: ConnId = NEXT_ID.fetch_add(1, Ordering::Relaxed);
    let (tx, mut rx) = unbounded_channel::<String>();

    log_info!("connected: {}", server_label(&server_name, &server_id));

    let _ = hub.send(HubMsg::Register {
        id,
        server_id: server_id.clone(),
        server_name: server_name.clone(),
        tx,
    });

    if ws
        .send(Message::Text(protocol::ready().into()))
        .await
        .is_ok()
    {
        loop {
            tokio::select! {
                incoming = ws.next() => match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMsg>(&text) {
                            Ok(ClientMsg::Bindings { bindings }) => {
                                let _ = hub.send(HubMsg::Bindings { id, bindings });
                            }
                            _ => log_debug!("{server_name} sent a message that was ignored"),
                        }
                    }
                    Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                    Some(Ok(_)) => {}
                },
                outgoing = rx.recv() => match outgoing {
                    Some(text) => {
                        if ws.send(Message::Text(text.into())).await.is_err() {
                            break;
                        }
                    }
                    // the hub dropped us: another tab of this server took over
                    None => break,
                },
            }
        }
    }

    let _ = ws.close(None).await;
    let _ = hub.send(HubMsg::Closed { id });

    log_info!("disconnected: {}", server_label(&server_name, &server_id));
}
