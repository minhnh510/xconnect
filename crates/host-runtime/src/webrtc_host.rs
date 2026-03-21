use std::{sync::Arc, time::Duration};

use bytes::Bytes;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::http::Request, tungstenite::Message};
use uuid::Uuid;
use webrtc::{
    api::{
        interceptor_registry::register_default_interceptors,
        media_engine::{MediaEngine, MIME_TYPE_H264},
        APIBuilder,
    },
    data_channel::RTCDataChannel,
    ice_transport::{ice_candidate::RTCIceCandidateInit, ice_server::RTCIceServer},
    interceptor::registry::Registry,
    media::Sample,
    peer_connection::{
        configuration::RTCConfiguration, sdp::session_description::RTCSessionDescription,
        RTCPeerConnection,
    },
    rtp_transceiver::rtp_codec::RTCRtpCodecCapability,
    track::track_local::{track_local_static_sample::TrackLocalStaticSample, TrackLocal},
};
use xconnect_protocol::{
    signal::{Hello, IceCandidate, SdpPayload, SessionEnd, SignalEnvelope, SignalMessage},
    ClipboardEvent, InputEvent,
};

use crate::{clipboard::ClipboardSync, input_apply::apply_input_event, HostRuntimeError};

#[derive(Debug, Clone)]
pub struct HostPeerConfig {
    pub session_id: Uuid,
    pub account_id: Uuid,
    pub device_id: Uuid,
    pub signaling_url: String,
    pub access_token: String,
    pub turn_uris: Vec<String>,
}

pub struct ConnectedHostPeer {
    session_id: Uuid,
    peer_connection: Arc<RTCPeerConnection>,
    video_track: Arc<TrackLocalStaticSample>,
    signal_tx: mpsc::UnboundedSender<SignalEnvelope>,
    clipboard_dc: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    _signal_writer_task: tokio::task::JoinHandle<()>,
    _signal_reader_task: tokio::task::JoinHandle<()>,
}

pub struct HostPeer {
    config: HostPeerConfig,
}

impl HostPeer {
    pub fn new(config: HostPeerConfig) -> Self {
        Self { config }
    }

    pub async fn connect(&self) -> Result<ConnectedHostPeer, HostRuntimeError> {
        let peer_connection = build_peer_connection(&self.config.turn_uris).await?;
        let clipboard_sync = ClipboardSync::default();

        let codec = RTCRtpCodecCapability {
            mime_type: MIME_TYPE_H264.to_string(),
            clock_rate: 90_000,
            channels: 0,
            sdp_fmtp_line: "level-asymmetry-allowed=1;packetization-mode=1;profile-level-id=42e01f"
                .to_string(),
            rtcp_feedback: vec![],
        };

        let video_track = Arc::new(TrackLocalStaticSample::new(
            codec,
            "video".to_string(),
            "xconnect-host".to_string(),
        ));
        let _rtp_sender = peer_connection
            .add_track(video_track.clone() as Arc<dyn TrackLocal + Send + Sync>)
            .await
            .map_err(map_webrtc("add_track"))?;

        let clipboard_dc = Arc::new(Mutex::new(None));
        let host_clipboard_dc = peer_connection
            .create_data_channel("clipboard", None)
            .await
            .map_err(map_webrtc("create clipboard data channel"))?;
        {
            let mut guard = clipboard_dc.lock().await;
            *guard = Some(host_clipboard_dc);
        }

        {
            let clipboard_sync = clipboard_sync.clone();
            peer_connection.on_data_channel(Box::new(move |dc: Arc<RTCDataChannel>| {
                let clipboard_sync = clipboard_sync.clone();
                Box::pin(async move {
                    let label = dc.label().to_string();
                    if label == "input" {
                        dc.on_message(Box::new(move |message| {
                            Box::pin(async move {
                                if let Ok(event) =
                                    serde_json::from_slice::<InputEvent>(&message.data)
                                {
                                    let _ = apply_input_event(&event);
                                }
                            })
                        }));
                    }

                    if label == "clipboard" {
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
            .map_err(|err| HostRuntimeError::Runtime(format!("build ws request failed: {err}")))?;

        let (stream, _) = connect_async(request)
            .await
            .map_err(|err| HostRuntimeError::Runtime(format!("signal ws connect failed: {err}")))?;
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

        let pc_for_read = peer_connection.clone();
        let session_id = self.config.session_id;
        let reader_task = tokio::spawn(async move {
            while let Some(next) = ws_read.next().await {
                let Ok(msg) = next else { break };
                let Message::Text(raw) = msg else { continue };
                let raw_text = raw.to_string();
                let Ok(envelope) = serde_json::from_str::<SignalEnvelope>(&raw_text) else {
                    continue;
                };

                match envelope.message {
                    SignalMessage::SdpAnswer(answer) if answer.session_id == session_id => {
                        if let Ok(desc) = RTCSessionDescription::answer(answer.sdp) {
                            let _ = pc_for_read.set_remote_description(desc).await;
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
            .map_err(|_| HostRuntimeError::Runtime("signal channel closed".to_string()))?;

        let offer = peer_connection
            .create_offer(None)
            .await
            .map_err(map_webrtc("create_offer"))?;
        peer_connection
            .set_local_description(offer.clone())
            .await
            .map_err(map_webrtc("set_local_description"))?;

        signal_tx
            .send(SignalEnvelope::new(
                Uuid::new_v4(),
                SignalMessage::SdpOffer(SdpPayload {
                    session_id: self.config.session_id,
                    sdp: offer.sdp,
                }),
            ))
            .map_err(|_| HostRuntimeError::Runtime("signal channel closed".to_string()))?;

        Ok(ConnectedHostPeer {
            session_id: self.config.session_id,
            peer_connection,
            video_track,
            signal_tx,
            clipboard_dc,
            _signal_writer_task: writer_task,
            _signal_reader_task: reader_task,
        })
    }
}

impl ConnectedHostPeer {
    pub async fn publish_h264_frame(
        &self,
        encoded_frame: Vec<u8>,
        duration_ms: u64,
    ) -> Result<(), HostRuntimeError> {
        if encoded_frame.is_empty() {
            return Err(HostRuntimeError::Runtime("empty encoded frame".to_string()));
        }

        let sample = Sample {
            data: Bytes::from(encoded_frame),
            duration: Duration::from_millis(duration_ms.max(1)),
            ..Default::default()
        };

        self.video_track
            .write_sample(&sample)
            .await
            .map_err(map_webrtc("write_sample"))
    }

    pub async fn send_clipboard_text(&self, text: String) -> Result<(), HostRuntimeError> {
        let payload = serde_json::to_vec(&ClipboardEvent {
            text_utf8: text,
            ts_unix_ms: chrono::Utc::now().timestamp_millis(),
        })
        .map_err(|err| HostRuntimeError::Runtime(format!("encode clipboard failed: {err}")))?;

        let guard = self.clipboard_dc.lock().await;
        let Some(dc) = guard.as_ref() else {
            return Err(HostRuntimeError::Runtime(
                "clipboard data channel not ready".to_string(),
            ));
        };

        dc.send(&Bytes::from(payload))
            .await
            .map(|_| ())
            .map_err(map_webrtc("send clipboard"))
    }

    pub async fn end_session(&self) -> Result<(), HostRuntimeError> {
        let _ = self.signal_tx.send(SignalEnvelope::new(
            Uuid::new_v4(),
            SignalMessage::SessionEnd(SessionEnd {
                session_id: self.session_id,
                reason: Some("host_ended".to_string()),
            }),
        ));

        self.peer_connection
            .close()
            .await
            .map_err(map_webrtc("close peer"))
    }
}

async fn build_peer_connection(
    turn_uris: &[String],
) -> Result<Arc<RTCPeerConnection>, HostRuntimeError> {
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

fn map_webrtc(action: &'static str) -> impl Fn(webrtc::Error) -> HostRuntimeError {
    move |err| HostRuntimeError::Runtime(format!("{action} failed: {err}"))
}
