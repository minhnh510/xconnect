use std::sync::Arc;

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::http::Request, tungstenite::Message};
use uuid::Uuid;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors, media_engine::MediaEngine, APIBuilder,
    },
    data_channel::RTCDataChannel,
    ice_transport::{ice_candidate::RTCIceCandidateInit, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::{rtp_receiver::RTCRtpReceiver, RTCRtpTransceiver},
    track::track_remote::TrackRemote,
};
use xconnect_protocol::{
    signal::{Hello, IceCandidate, SdpPayload, SessionEnd, SignalEnvelope, SignalMessage},
    ClipboardEvent, InputEvent,
};

use crate::{clipboard::ClipboardSync, ViewerRuntimeError};

#[derive(Debug, Clone)]
pub struct ViewerPeerConfig {
    pub session_id: Uuid,
    pub account_id: Uuid,
    pub device_id: Uuid,
    pub signaling_url: String,
    pub access_token: String,
    pub turn_uris: Vec<String>,
}

pub struct ConnectedViewerPeer {
    session_id: Uuid,
    peer_connection: Arc<RTCPeerConnection>,
    signal_tx: mpsc::UnboundedSender<SignalEnvelope>,
    input_dc: Arc<RTCDataChannel>,
    clipboard_dc: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    video_rx: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    _signal_writer_task: tokio::task::JoinHandle<()>,
    _signal_reader_task: tokio::task::JoinHandle<()>,
}

pub struct ViewerPeer {
    config: ViewerPeerConfig,
}

impl ViewerPeer {
    pub fn new(config: ViewerPeerConfig) -> Self {
        Self { config }
    }

    pub async fn connect(&self) -> Result<ConnectedViewerPeer, ViewerRuntimeError> {
        let peer_connection = build_peer_connection(&self.config.turn_uris).await?;
        let clipboard_sync = ClipboardSync::default();

        let input_dc = peer_connection
            .create_data_channel("input", None)
            .await
            .map_err(map_webrtc("create input data channel"))?;

        let (video_tx, video_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        peer_connection.on_track(Box::new(
            move |track: Arc<TrackRemote>,
                  _receiver: Arc<RTCRtpReceiver>,
                  _transceiver: Arc<RTCRtpTransceiver>| {
                let video_tx = video_tx.clone();
                Box::pin(async move {
                    while let Ok((packet, _)) = track.read_rtp().await {
                        let _ = video_tx.send(packet.payload.to_vec());
                    }
                })
            },
        ));

        let clipboard_dc = Arc::new(Mutex::new(None));
        {
            let clipboard_dc_slot = clipboard_dc.clone();
            let clipboard_sync = clipboard_sync.clone();
            peer_connection.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
                let clipboard_dc_slot = clipboard_dc_slot.clone();
                let clipboard_sync = clipboard_sync.clone();
                Box::pin(async move {
                    if dc.label() == "clipboard" {
                        {
                            let mut guard = clipboard_dc_slot.lock().await;
                            *guard = Some(dc.clone());
                        }

                        dc.on_message(Box::new(move |message| {
                            let clipboard_sync = clipboard_sync.clone();
                            Box::pin(async move {
                                if let Ok(event) =
                                    serde_json::from_slice::<ClipboardEvent>(&message.data)
                                {
                                    let _ = clipboard_sync.apply_remote(event);
                                }
                            })
                        }));
                    }
                })
            }));
        }

        let ws_url = format!(
            "{}?session_id={}",
            self.config.signaling_url, self.config.session_id
        );
        let request = Request::builder()
            .uri(ws_url)
            .header(
                "Authorization",
                format!("Bearer {}", self.config.access_token),
            )
            .body(())
            .map_err(|err| {
                ViewerRuntimeError::Runtime(format!("build ws request failed: {err}"))
            })?;

        let (stream, _) = connect_async(request).await.map_err(|err| {
            ViewerRuntimeError::Runtime(format!("signal ws connect failed: {err}"))
        })?;
        let (mut ws_write, mut ws_read) = stream.split();

        let (signal_tx, mut signal_rx) = mpsc::unbounded_channel::<SignalEnvelope>();

        let writer_task = tokio::spawn(async move {
            while let Some(envelope) = signal_rx.recv().await {
                if let Ok(raw) = serde_json::to_string(&envelope) {
                    if ws_write.send(Message::Text(raw.into())).await.is_err() {
                        break;
                    }
                }
            }
        });

        {
            let signal_tx = signal_tx.clone();
            let session_id = self.config.session_id;
            peer_connection.on_ice_candidate(Box::new(move |candidate| {
                let signal_tx = signal_tx.clone();
                Box::pin(async move {
                    if let Some(candidate) = candidate {
                        if let Ok(candidate_json) = candidate.to_json() {
                            let envelope = SignalEnvelope::new(
                                Uuid::new_v4(),
                                SignalMessage::IceCandidate(IceCandidate {
                                    session_id,
                                    candidate: candidate_json.candidate,
                                    sdp_mid: candidate_json.sdp_mid,
                                    sdp_mline_index: candidate_json.sdp_mline_index,
                                }),
                            );
                            let _ = signal_tx.send(envelope);
                        }
                    }
                })
            }));
        }

        let session_id = self.config.session_id;
        let pc_for_read = peer_connection.clone();
        let signal_tx_for_read = signal_tx.clone();
        let reader_task = tokio::spawn(async move {
            while let Some(next) = ws_read.next().await {
                let Ok(msg) = next else { break };
                let Message::Text(raw) = msg else { continue };
                let raw_text = raw.to_string();
                let Ok(envelope) = serde_json::from_str::<SignalEnvelope>(&raw_text) else {
                    continue;
                };

                match envelope.message {
                    SignalMessage::SdpOffer(offer) if offer.session_id == session_id => {
                        if let Ok(desc) = RTCSessionDescription::offer(offer.sdp) {
                            if pc_for_read.set_remote_description(desc).await.is_ok() {
                                if let Ok(answer) = pc_for_read.create_answer(None).await {
                                    if pc_for_read
                                        .set_local_description(answer.clone())
                                        .await
                                        .is_ok()
                                    {
                                        let _ = signal_tx_for_read.send(SignalEnvelope::new(
                                            Uuid::new_v4(),
                                            SignalMessage::SdpAnswer(SdpPayload {
                                                session_id,
                                                sdp: answer.sdp,
                                            }),
                                        ));
                                    }
                                }
                            }
                        }
                    }
                    SignalMessage::IceCandidate(ice) if ice.session_id == session_id => {
                        let candidate = RTCIceCandidateInit {
                            candidate: ice.candidate,
                            sdp_mid: ice.sdp_mid,
                            sdp_mline_index: ice.sdp_mline_index,
                            username_fragment: None,
                        };
                        let _ = pc_for_read.add_ice_candidate(candidate).await;
                    }
                    SignalMessage::SessionEnd(SessionEnd { .. }) => {
                        let _ = pc_for_read.close().await;
                        break;
                    }
                    _ => {}
                }
            }
        });

        signal_tx
            .send(SignalEnvelope::new(
                Uuid::new_v4(),
                SignalMessage::Hello(Hello {
                    account_id: self.config.account_id,
                    device_id: self.config.device_id,
                }),
            ))
            .map_err(|_| ViewerRuntimeError::Runtime("signal channel closed".to_string()))?;

        Ok(ConnectedViewerPeer {
            session_id: self.config.session_id,
            peer_connection,
            signal_tx,
            input_dc,
            clipboard_dc,
            video_rx: Arc::new(Mutex::new(video_rx)),
            _signal_writer_task: writer_task,
            _signal_reader_task: reader_task,
        })
    }
}

impl ConnectedViewerPeer {
    pub async fn send_input(&self, event: InputEvent) -> Result<(), ViewerRuntimeError> {
        let payload = serde_json::to_vec(&event)
            .map_err(|err| ViewerRuntimeError::Runtime(format!("encode input failed: {err}")))?;

        self.input_dc
            .send(&Bytes::from(payload))
            .await
            .map(|_| ())
            .map_err(map_webrtc("send input"))
    }

    pub async fn send_clipboard_text(&self, text: String) -> Result<(), ViewerRuntimeError> {
        let payload = serde_json::to_vec(&ClipboardEvent {
            text_utf8: text,
            ts_unix_ms: chrono::Utc::now().timestamp_millis(),
        })
        .map_err(|err| ViewerRuntimeError::Runtime(format!("encode clipboard failed: {err}")))?;

        let guard = self.clipboard_dc.lock().await;
        let Some(dc) = guard.as_ref() else {
            return Err(ViewerRuntimeError::Runtime(
                "clipboard data channel not ready".to_string(),
            ));
        };

        dc.send(&Bytes::from(payload))
            .await
            .map(|_| ())
            .map_err(map_webrtc("send clipboard"))
    }

    pub async fn end_session(&self) -> Result<(), ViewerRuntimeError> {
        let _ = self.signal_tx.send(SignalEnvelope::new(
            Uuid::new_v4(),
            SignalMessage::SessionEnd(SessionEnd {
                session_id: self.session_id,
                reason: Some("viewer_ended".to_string()),
            }),
        ));

        self.peer_connection
            .close()
            .await
            .map_err(map_webrtc("close peer"))
    }

    pub async fn recv_video_packet(&self) -> Option<Vec<u8>> {
        let mut guard = self.video_rx.lock().await;
        guard.recv().await
    }
}

async fn build_peer_connection(
    turn_uris: &[String],
) -> Result<Arc<RTCPeerConnection>, ViewerRuntimeError> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_default_codecs()
        .map_err(map_webrtc("register_default_codecs"))?;

    let mut registry = Registry::new();
    registry = register_default_interceptors(registry, &mut media_engine)
        .map_err(map_webrtc("register_default_interceptors"))?;

    let api = APIBuilder::new()
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .build();

    let mut ice_servers = vec![];
    if !turn_uris.is_empty() {
        ice_servers.push(RTCIceServer {
            urls: turn_uris.to_vec(),
            ..Default::default()
        });
    }

    let config = RTCConfiguration {
        ice_servers,
        ..Default::default()
    };

    let peer = api
        .new_peer_connection(config)
        .await
        .map_err(map_webrtc("new_peer_connection"))?;

    Ok(Arc::new(peer))
}

fn map_webrtc(action: &'static str) -> impl Fn(webrtc::Error) -> ViewerRuntimeError {
    move |err| ViewerRuntimeError::Runtime(format!("{action} failed: {err}"))
}
