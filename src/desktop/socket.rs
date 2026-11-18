use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::{
    io,
    net::{Ipv4Addr, Ipv6Addr, TcpListener},
    sync::Arc,
    thread,
    time::Duration,
};
use tokio::sync::{Semaphore, broadcast, oneshot, watch};
use tokio_tungstenite::{
    accept_hdr_async_with_config,
    tungstenite::{
        Message,
        handshake::server::{ErrorResponse, Request as HandshakeRequest},
        http::{HeaderValue, StatusCode, header::ORIGIN, uri::Authority},
        protocol::WebSocketConfig,
    },
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
        let listener = {
            let _guard = runtime.enter();
            tokio::net::TcpListener::from_std(listener)?
        };
        let (state, current) = watch::channel("idle");
        let (changes, _) = broadcast::channel(32);
        let (stop, mut stopped) = oneshot::channel();
        let updates = changes.clone();
        thread::Builder::new()
            .name("pomotroid-websocket".into())
            .spawn(move || {
                runtime.block_on(async move {
                    let limit = Arc::new(Semaphore::new(32));
                    loop {
                        tokio::select! {
                            biased;
                            _ = &mut stopped => break,
                            result = listener.accept() => {
                                let Ok((stream, _)) = result else {
                                    tokio::select! {
                                        _ = &mut stopped => break,
                                        _ = tokio::time::sleep(Duration::from_millis(100)) => continue,
                                    }
                                };
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

fn is_loopback_origin(origin: &HeaderValue) -> bool {
    let Some((scheme, address)) = origin
        .to_str()
        .ok()
        .and_then(|value| value.split_once("://"))
    else {
        return false;
    };
    if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
        || address.contains('@')
    {
        return false;
    }
    let Ok(authority) = address.parse::<Authority>() else {
        return false;
    };
    let host = authority.host();
    if address != host
        && !address
            .strip_prefix(host)
            .and_then(|suffix| suffix.strip_prefix(':'))
            .is_some_and(|port| {
                port.bytes().all(|digit| digit.is_ascii_digit()) && port.parse::<u16>().is_ok()
            })
    {
        return false;
    }
    if let Some(ip) = host
        .strip_prefix('[')
        .and_then(|host| host.strip_suffix(']'))
    {
        return ip.parse::<Ipv6Addr>().is_ok_and(|ip| ip.is_loopback());
    }
    host.eq_ignore_ascii_case("localhost")
        || host.parse::<Ipv4Addr>().is_ok_and(|ip| ip.is_loopback())
}

async fn client(
    stream: tokio::net::TcpStream,
    current: watch::Receiver<&'static str>,
    mut updates: broadcast::Receiver<&'static str>,
) -> Result<(), tokio_tungstenite::tungstenite::Error> {
    let config = WebSocketConfig::default()
        .max_message_size(Some(4096))
        .max_frame_size(Some(4096));
    #[expect(
        clippy::result_large_err,
        reason = "Tungstenite requires this handshake error type"
    )]
    let handshake = |request: &HandshakeRequest, response| {
        let mut origins = request.headers().get_all(ORIGIN).iter();
        match origins.next() {
            None => Ok(response),
            Some(origin) if is_loopback_origin(origin) && origins.next().is_none() => Ok(response),
            Some(_) => {
                let mut rejection = ErrorResponse::new(None);
                *rejection.status_mut() = StatusCode::FORBIDDEN;
                Err(rejection)
            }
        }
    };
    let Ok(connection) = tokio::time::timeout(
        Duration::from_secs(5),
        accept_hdr_async_with_config(stream, handshake, Some(config)),
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
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Error, client::IntoClientRequest},
    };

    #[tokio::test]
    async fn browser_connections_require_a_single_loopback_origin() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let _server = StateServer::start(listener).unwrap();

        tokio::time::timeout(Duration::from_secs(5), async {
            for origin in [
                "http://localhost",
                "https://LOCALHOST:8443",
                "http://127.0.0.1:3000",
                "http://127.23.45.67",
                "https://[::1]:8443",
            ] {
                let mut request = format!("ws://{address}").into_client_request().unwrap();
                request.headers_mut().insert(ORIGIN, origin.parse().unwrap());
                connect_async(request).await.unwrap_or_else(|error| {
                    panic!("loopback origin {origin} was rejected: {error}")
                });
            }

            for origins in [
                &["https://example.test"][..],
                &["http://localhost.example.test"],
                &["http://localhost@example.test"],
                &["http://example.test@localhost"],
                &["http://192.0.2.1"],
                &["https://[2001:db8::1]"],
                &["http://localhost:"],
                &["http://localhost:65536"],
                &["http://localhost:+80"],
                &["http://localhost/page"],
                &["http://localhost?query"],
                &["http://localhost#fragment"],
                &["ftp://localhost"],
                &["null"],
                &[""],
                &["http://localhost", "https://example.test"],
            ] {
                let mut request = format!("ws://{address}").into_client_request().unwrap();
                for origin in origins {
                    request.headers_mut().append(ORIGIN, HeaderValue::from_static(origin));
                }
                assert!(
                    matches!(connect_async(request).await, Err(Error::Http(response)) if response.status() == StatusCode::FORBIDDEN),
                    "expected HTTP 403 for {origins:?}"
                );
            }
        })
        .await
        .unwrap();
    }

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
