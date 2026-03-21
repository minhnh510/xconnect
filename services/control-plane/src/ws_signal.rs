use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    http::HeaderMap,
    response::IntoResponse,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::broadcast;
use tracing::{debug, warn};
use uuid::Uuid;
use xconnect_protocol::signal::{SignalEnvelope, SignalMessage};

use crate::{auth::extract_access_subject, errors::ApiError, AppState};

#[derive(Clone, Debug)]
struct HubMessage {
    sender: Uuid,
    payload: String,
}

#[derive(Clone, Default)]
pub struct WsHub {
    channels: Arc<DashMap<Uuid, broadcast::Sender<HubMessage>>>,
}

impl WsHub {
    fn sender_for(&self, session_id: Uuid) -> broadcast::Sender<HubMessage> {
        self.channels
            .entry(session_id)
            .or_insert_with(|| {
                let (tx, _rx) = broadcast::channel(256);
                tx
            })
            .clone()
    }
}

#[derive(Deserialize)]
pub struct SignalQuery {
    session_id: Uuid,
}

pub async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
    Query(query): Query<SignalQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let account_id = extract_access_subject(&headers, &state.token_service)?;
    let session = state
        .db
        .get_session(query.session_id)
        .ok_or(ApiError::NotFound)?;

    if session.account_id != account_id {
        return Err(ApiError::Forbidden);
    }

    state.metrics.inc_ws_connected();
    Ok(ws.on_upgrade(move |socket| handle_socket(state, socket, query.session_id)))
}

async fn handle_socket(state: Arc<AppState>, socket: WebSocket, session_id: Uuid) {
    let connection_id = Uuid::new_v4();
    let sender = state.ws_hub.sender_for(session_id);
    let mut receiver = sender.subscribe();

    let (mut ws_tx, mut ws_rx) = socket.split();

    let tx_task = tokio::spawn(async move {
        loop {
            match receiver.recv().await {
                Ok(msg) => {
                    if msg.sender == connection_id {
                        continue;
                    }
                    if ws_tx.send(Message::Text(msg.payload.into())).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let rx_sender = sender.clone();
    let rx_task = tokio::spawn(async move {
        while let Some(Ok(message)) = ws_rx.next().await {
            match message {
                Message::Text(payload) => {
                    let parsed = serde_json::from_str::<SignalEnvelope>(&payload);
                    if let Ok(envelope) = parsed {
                        if session_matches(session_id, &envelope.message) {
                            let _ = rx_sender.send(HubMessage {
                                sender: connection_id,
                                payload: payload.to_string(),
                            });
                        } else {
                            debug!(%session_id, "dropped signal payload with mismatched session");
                        }
                    } else {
                        debug!(%session_id, "dropped invalid signal envelope");
                    }
                }
                Message::Close(_) => break,
                Message::Binary(_) | Message::Ping(_) | Message::Pong(_) => {
                    debug!("ignored non-text ws message");
                }
            }
        }
    });

    let _ = tokio::join!(tx_task, rx_task);
    warn!(%session_id, %connection_id, "websocket disconnected");
}

fn session_matches(expected: Uuid, msg: &SignalMessage) -> bool {
    match msg {
        SignalMessage::Hello(_) => true,
        SignalMessage::SessionRequest(req) => req.session_id == expected,
        SignalMessage::SessionResponse(resp) => resp.session_id == expected,
        SignalMessage::SdpOffer(sdp) => sdp.session_id == expected,
        SignalMessage::SdpAnswer(sdp) => sdp.session_id == expected,
        SignalMessage::IceCandidate(candidate) => candidate.session_id == expected,
        SignalMessage::SessionState(state) => state.session_id == expected,
        SignalMessage::SessionEnd(end) => end.session_id == expected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use xconnect_protocol::signal::{IceCandidate, SdpPayload, SessionEnd};

    #[test]
    fn session_match_accepts_same_id() {
        let sid = Uuid::new_v4();
        let ok = session_matches(
            sid,
            &SignalMessage::SdpOffer(SdpPayload {
                session_id: sid,
                sdp: "offer".to_string(),
            }),
        );
        assert!(ok);
    }

    #[test]
    fn session_match_rejects_other_id() {
        let sid = Uuid::new_v4();
        let other = Uuid::new_v4();
        assert!(!session_matches(
            sid,
            &SignalMessage::IceCandidate(IceCandidate {
                session_id: other,
                candidate: "cand".to_string(),
                sdp_mid: None,
                sdp_mline_index: None,
            })
        ));
        assert!(!session_matches(
            sid,
            &SignalMessage::SessionEnd(SessionEnd {
                session_id: other,
                reason: None
            })
        ));
    }
}
