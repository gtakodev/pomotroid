use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::{io, net::TcpListener, sync::Arc, thread, time::Duration};
use tokio::sync::{Semaphore, broadcast, oneshot, watch};
use tokio_tungstenite::{
    accept_async_with_config,
    tungstenite::{Message, protocol::WebSocketConfig},
};

pub(super) struct StateServer {
    state: watch::Sender<&'static str>,
    changes: broadcast::Sender<&'static str>,
    _shutdown: oneshot::Sender<()>,
}

impl StateServer {
    pub(super) fn start(listener: TcpListener) -> io::Result<Self> {
        listener.set_nonblocking(true)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        let (state, current) = watch::channel("idle");
        let (changes, _) = broadcast::channel(32);
        let (stop, mut stopped) = oneshot::channel();
        let updates = changes.clone();
        thread::Builder::new()
            .name("pomotroid-websocket".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let Ok(listener) = tokio::net::TcpListener::from_std(listener) else {
                        return;
                    };
                    let limit = Arc::new(Semaphore::new(32));
                    loop {
                        tokio::select! {
                            _ = &mut stopped => break,
                            result = listener.accept() => {
                                let Ok((stream, _)) = result else { break };
                                if let Ok(slot) = limit.clone().try_acquire_owned() {
                                    let current = current.clone();
                                    let events = updates.subscribe();
                                    tokio::spawn(async move {
                                        let _slot = slot;
                                        let _ = client(stream, current, events).await;
                                    });
                                }
                            }
                        }
                    }
                });
            })?;
        Ok(Self {
            state,
            changes,
            _shutdown: stop,
        })
    }

    pub(super) fn publish(&self, state: &'static str) {
        self.state.send_replace(state);
        let _ = self.changes.send(state);
    }
}

fn packet(event: &str, state: &str) -> Message {
    Message::text(json!({"event":event, "data":{"state":state}}).to_string())
}

#[derive(Deserialize)]
struct Request<'a> {
    event: &'a str,
}

async fn client(
    stream: tokio::net::TcpStream,
    current: watch::Receiver<&'static str>,
    mut updates: broadcast::Receiver<&'static str>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(4096))
        .max_frame_size(Some(4096));
    let Ok(connection) = tokio::time::timeout(
        Duration::from_secs(5),
        accept_async_with_config(stream, Some(config)),
    )
    .await
    else {
        return Ok(());
    };
    let mut socket = connection?;
    loop {
        let response = tokio::select! {
            message = socket.next() => match message.transpose()? {
                Some(Message::Text(text)) => {
                    if !matches!(serde_json::from_str::<Request>(&text), Ok(Request { event: "getState" })) {
                        continue;
                    }
                    packet("getState", &current.borrow())
                }
                Some(Message::Close(_)) | None => return Ok(()),
                Some(Message::Ping(_)) => { socket.flush().await?; continue; }
                _ => continue,
            },
            change = updates.recv() => match change {
                Ok(state) => packet("roundChange", state),
                Err(broadcast::error::RecvError::Lagged(_)) => return Ok(()),
                Err(broadcast::error::RecvError::Closed) => return Ok(()),
            }
        };
        socket.send(response).await?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::connect_async;

    #[tokio::test]
    async fn clients_can_query_and_subscribe_to_the_timer() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        let server = StateServer::start(listener).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            let (mut first, _) = connect_async(format!("ws://{addr}")).await.unwrap();
            let (mut second, _) = connect_async(format!("ws://{addr}")).await.unwrap();
            first.send(Message::text("invalid")).await.unwrap();
            first
                .send(Message::text(r#"{"event":"getState"}"#))
                .await
                .unwrap();
            assert_eq!(
                first.next().await.unwrap().unwrap(),
                packet("getState", "idle")
            );
            server.publish("work");
            for client in [&mut first, &mut second] {
                assert_eq!(
                    client.next().await.unwrap().unwrap(),
                    packet("roundChange", "work")
                );
            }
            second
                .send(Message::text(r#"{"event":"getState"}"#))
                .await
                .unwrap();
            assert_eq!(
                second.next().await.unwrap().unwrap(),
                packet("getState", "work")
            );
        })
        .await
        .unwrap();
    }
}
