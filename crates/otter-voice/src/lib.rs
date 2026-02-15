//! # Otter Voice
//!
//! 1-to-1 voice calling module for the Otter decentralized chat platform.
//!
//! This crate provides:
//! - WebRTC-based audio streaming
//! - Integration with Otter's secure signaling protocol
//! - Mono audio with fixed bitrate
//! - Simple call management (call, answer, hangup)
//!
//! ## Example
//!
//! ```no_run
//! use otter_voice::{VoiceManager, CallConfig};
//!
//! # async fn example() -> anyhow::Result<()> {
//! let mut voice_manager = VoiceManager::new()?;
//! 
//! // Start a call
//! voice_manager.initiate_call("peer_id", CallConfig::default()).await?;
//! # Ok(())
//! # }
//! ```

use anyhow::Result;
use otter_protocol::{MediaType, SignalingMessage};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, info, warn};
use uuid::Uuid;
use webrtc::api::media_engine::MediaEngine;
use webrtc::api::APIBuilder;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::rtp_transceiver::rtp_codec::{RTCRtpCodecCapability, RTCRtpCodecParameters, RTPCodecType};
use webrtc::track::track_local::track_local_static_rtp::TrackLocalStaticRTP;
use webrtc::track::track_local::TrackLocal;

#[derive(Error, Debug)]
pub enum VoiceError {
    #[error("WebRTC error: {0}")]
    WebRtc(String),
    #[error("No active call")]
    NoActiveCall,
    #[error("Call already active with peer: {0}")]
    CallAlreadyActive(String),
    #[error("Invalid SDP: {0}")]
    InvalidSdp(String),
    #[error("Invalid peer ID: {0}")]
    InvalidPeerId(String),
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Audio error: {0}")]
    AudioError(String),
}

/// Call configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallConfig {
    /// Sample rate in Hz (default: 48000)
    pub sample_rate: u32,
    /// Channels: 1 for mono, 2 for stereo (default: 1 for mono)
    pub channels: u8,
    /// Bitrate in bps (default: 64000)
    pub bitrate: u32,
    /// STUN server URLs for NAT traversal
    pub stun_servers: Vec<String>,
    /// TURN server URLs for relay (if needed)
    pub turn_servers: Vec<String>,
}

impl Default for CallConfig {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 1, // Mono
            bitrate: 64000, // 64 kbps
            stun_servers: vec![
                "stun:stun.l.google.com:19302".to_string(),
                "stun:stun1.l.google.com:19302".to_string(),
            ],
            turn_servers: Vec::new(),
        }
    }
}

/// Call state
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallState {
    /// No active call
    Idle,
    /// Outgoing call initiated, waiting for answer
    Calling,
    /// Incoming call, waiting for user to answer
    Ringing,
    /// Call is being connected (ICE negotiation)
    Connecting,
    /// Call is active
    Connected,
    /// Call ended
    Ended,
}

/// Active call information
#[derive(Debug)]
pub struct CallSession {
    /// Session ID
    pub session_id: String,
    /// Peer ID
    pub peer_id: String,
    /// Current call state
    pub state: CallState,
    /// WebRTC peer connection
    peer_connection: Arc<RTCPeerConnection>,
    /// Audio track for sending
    audio_track: Option<Arc<TrackLocalStaticRTP>>,
    /// Whether this peer initiated the call
    is_initiator: bool,
    /// ICE candidates collected before connection
    pending_ice_candidates: Vec<String>,
}

/// Voice manager for handling WebRTC voice calls
pub struct VoiceManager {
    /// Current active call (only one call at a time)
    active_call: Arc<RwLock<Option<CallSession>>>,
    /// Call configuration
    config: CallConfig,
    /// Channel for outgoing signaling messages
    signaling_tx: Option<mpsc::UnboundedSender<(String, SignalingMessage)>>,
    /// WebRTC API
    api: Arc<webrtc::api::API>,
}

impl VoiceManager {
    /// Create a new voice manager
    pub fn new() -> Result<Self> {
        let mut media_engine = MediaEngine::default();
        
        // Register Opus codec for audio (standard for WebRTC voice)
        media_engine.register_codec(
            RTCRtpCodecParameters {
                capability: RTCRtpCodecCapability {
                    mime_type: "audio/opus".to_owned(),
                    clock_rate: 48000,
                    channels: 2, // Opus supports stereo, we'll use mono
                    sdp_fmtp_line: "".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 111,
                ..Default::default()
            },
            RTPCodecType::Audio,
        )?;
        
        // Build API with media engine (simpler version without interceptors for minimal PoC)
        let api = APIBuilder::new()
            .with_media_engine(media_engine)
            .build();
        
        Ok(Self {
            active_call: Arc::new(RwLock::new(None)),
            config: CallConfig::default(),
            signaling_tx: None,
            api: Arc::new(api),
        })
    }
    
    /// Set signaling channel for sending signaling messages
    pub fn set_signaling_channel(&mut self, tx: mpsc::UnboundedSender<(String, SignalingMessage)>) {
        self.signaling_tx = Some(tx);
    }
    
    /// Initiate a call to a peer
    pub async fn initiate_call(&mut self, peer_id: &str, config: CallConfig) -> Result<String> {
        // Check if there's already an active call
        {
            let call_lock = self.active_call.read().await;
            if call_lock.is_some() {
                return Err(VoiceError::CallAlreadyActive(peer_id.to_string()).into());
            }
        }
        
        self.config = config;
        let session_id = Uuid::new_v4().to_string();
        
        info!("Initiating call to peer {} with session {}", peer_id, session_id);
        
        // Create peer connection
        let peer_connection = self.create_peer_connection().await?;
        
        // Create audio track
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "audio/opus".to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            "audio".to_owned(),
            "otter-audio".to_owned(),
        ));
        
        // Add track to peer connection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;
        
        // Handle RTCP packets (for monitoring)
        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        });
        
        // Create and set local description (offer)
        let offer = peer_connection.create_offer(None).await?;
        peer_connection.set_local_description(offer.clone()).await?;
        
        let sdp = offer.sdp;
        
        // Create call session
        let call_session = CallSession {
            session_id: session_id.clone(),
            peer_id: peer_id.to_string(),
            state: CallState::Calling,
            peer_connection: Arc::clone(&peer_connection),
            audio_track: Some(audio_track),
            is_initiator: true,
            pending_ice_candidates: Vec::new(),
        };
        
        // Store active call
        {
            let mut call_lock = self.active_call.write().await;
            *call_lock = Some(call_session);
        }
        
        // Send offer via signaling channel
        if let Some(ref tx) = self.signaling_tx {
            let signaling_msg = SignalingMessage::Offer {
                sdp,
                media_type: MediaType::AudioOnly,
                session_id: session_id.clone(),
            };
            tx.send((peer_id.to_string(), signaling_msg))?;
            info!("Sent offer for session {}", session_id);
        }
        
        Ok(session_id)
    }
    
    /// Handle incoming signaling message
    pub async fn handle_signaling(&mut self, peer_id: &str, message: SignalingMessage) -> Result<()> {
        match message {
            SignalingMessage::Offer { sdp, media_type, session_id } => {
                info!("Received call offer from {} for session {}", peer_id, session_id);
                self.handle_offer(peer_id, &session_id, &sdp, media_type).await?;
            }
            SignalingMessage::Answer { sdp, session_id } => {
                info!("Received answer from {} for session {}", peer_id, session_id);
                self.handle_answer(&session_id, &sdp).await?;
            }
            SignalingMessage::IceCandidate { candidate, session_id, .. } => {
                debug!("Received ICE candidate from {} for session {}", peer_id, session_id);
                self.handle_ice_candidate(&session_id, &candidate).await?;
            }
            SignalingMessage::IceComplete { session_id } => {
                debug!("ICE gathering complete from {} for session {}", peer_id, session_id);
            }
            SignalingMessage::Hangup { session_id, reason } => {
                info!("Received hangup from {} for session {}: {:?}", peer_id, session_id, reason);
                self.hangup().await?;
            }
            _ => {}
        }
        Ok(())
    }
    
    /// Handle incoming offer
    async fn handle_offer(&mut self, peer_id: &str, session_id: &str, sdp: &str, _media_type: MediaType) -> Result<()> {
        // Check if there's already an active call
        {
            let call_lock = self.active_call.read().await;
            if call_lock.is_some() {
                warn!("Already in a call, rejecting incoming call from {}", peer_id);
                // TODO: Send reject message
                return Ok(());
            }
        }
        
        // Create peer connection
        let peer_connection = self.create_peer_connection().await?;
        
        // Create audio track
        let audio_track = Arc::new(TrackLocalStaticRTP::new(
            RTCRtpCodecCapability {
                mime_type: "audio/opus".to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "".to_owned(),
                rtcp_feedback: vec![],
            },
            "audio".to_owned(),
            "otter-audio".to_owned(),
        ));
        
        // Add track to peer connection
        let rtp_sender = peer_connection
            .add_track(Arc::clone(&audio_track) as Arc<dyn TrackLocal + Send + Sync>)
            .await?;
        
        // Handle RTCP packets
        tokio::spawn(async move {
            let mut rtcp_buf = vec![0u8; 1500];
            while let Ok((_, _)) = rtp_sender.read(&mut rtcp_buf).await {}
        });
        
        // Set remote description (offer)
        let offer = RTCSessionDescription::offer(sdp.to_string())?;
        peer_connection.set_remote_description(offer).await?;
        
        // Create and set local description (answer)
        let answer = peer_connection.create_answer(None).await?;
        peer_connection.set_local_description(answer.clone()).await?;
        
        let answer_sdp = answer.sdp;
        
        // Create call session
        let call_session = CallSession {
            session_id: session_id.to_string(),
            peer_id: peer_id.to_string(),
            state: CallState::Ringing,
            peer_connection: Arc::clone(&peer_connection),
            audio_track: Some(audio_track),
            is_initiator: false,
            pending_ice_candidates: Vec::new(),
        };
        
        // Store active call
        {
            let mut call_lock = self.active_call.write().await;
            *call_lock = Some(call_session);
        }
        
        // Send answer via signaling channel
        if let Some(ref tx) = self.signaling_tx {
            let signaling_msg = SignalingMessage::Answer {
                sdp: answer_sdp,
                session_id: session_id.to_string(),
            };
            tx.send((peer_id.to_string(), signaling_msg))?;
            info!("Sent answer for session {}", session_id);
        }
        
        // Update state to connecting
        {
            let mut call_lock = self.active_call.write().await;
            if let Some(ref mut call) = *call_lock {
                call.state = CallState::Connecting;
            }
        }
        
        Ok(())
    }
    
    /// Handle answer to our offer
    async fn handle_answer(&mut self, session_id: &str, sdp: &str) -> Result<()> {
        let mut call_lock = self.active_call.write().await;
        if let Some(ref mut call) = *call_lock {
            if call.session_id == session_id {
                let answer = RTCSessionDescription::answer(sdp.to_string())?;
                call.peer_connection.set_remote_description(answer).await?;
                call.state = CallState::Connecting;
                info!("Set remote description for session {}", session_id);
            }
        }
        Ok(())
    }
    
    /// Handle ICE candidate
    async fn handle_ice_candidate(&mut self, session_id: &str, candidate: &str) -> Result<()> {
        let call_lock = self.active_call.read().await;
        if let Some(ref call) = *call_lock {
            if call.session_id == session_id {
                // Parse and add ICE candidate
                let candidate_init = webrtc::ice_transport::ice_candidate::RTCIceCandidateInit {
                    candidate: candidate.to_string(),
                    ..Default::default()
                };
                
                if let Err(e) = call.peer_connection.add_ice_candidate(candidate_init).await {
                    warn!("Failed to add ICE candidate: {}", e);
                }
            }
        }
        Ok(())
    }
    
    /// Answer an incoming call
    pub async fn answer_call(&mut self) -> Result<()> {
        let mut call_lock = self.active_call.write().await;
        if let Some(ref mut call) = *call_lock {
            if call.state == CallState::Ringing {
                call.state = CallState::Connecting;
                info!("Answered call with peer {}", call.peer_id);
                return Ok(());
            }
        }
        Err(VoiceError::NoActiveCall.into())
    }
    
    /// Hang up the current call
    pub async fn hangup(&mut self) -> Result<()> {
        let mut call_lock = self.active_call.write().await;
        if let Some(call) = call_lock.take() {
            info!("Hanging up call with peer {}", call.peer_id);
            
            // Send hangup message
            if let Some(ref tx) = self.signaling_tx {
                let signaling_msg = SignalingMessage::Hangup {
                    session_id: call.session_id.clone(),
                    reason: Some("User hung up".to_string()),
                };
                let _ = tx.send((call.peer_id.clone(), signaling_msg));
            }
            
            // Close peer connection
            if let Err(e) = call.peer_connection.close().await {
                warn!("Error closing peer connection: {}", e);
            }
            
            Ok(())
        } else {
            Err(VoiceError::NoActiveCall.into())
        }
    }
    
    /// Get current call state
    pub async fn get_call_state(&self) -> CallState {
        let call_lock = self.active_call.read().await;
        call_lock.as_ref().map(|c| c.state.clone()).unwrap_or(CallState::Idle)
    }
    
    /// Check if there's an active call
    pub async fn has_active_call(&self) -> bool {
        let call_lock = self.active_call.read().await;
        call_lock.is_some()
    }
    
    /// Get current peer ID if in call
    pub async fn get_current_peer(&self) -> Option<String> {
        let call_lock = self.active_call.read().await;
        call_lock.as_ref().map(|c| c.peer_id.clone())
    }
    
    /// Create a new peer connection with configuration
    async fn create_peer_connection(&self) -> Result<Arc<RTCPeerConnection>> {
        let mut ice_servers = Vec::new();
        
        // Add STUN servers
        for stun_url in &self.config.stun_servers {
            ice_servers.push(RTCIceServer {
                urls: vec![stun_url.clone()],
                ..Default::default()
            });
        }
        
        // Add TURN servers (if configured)
        for turn_url in &self.config.turn_servers {
            ice_servers.push(RTCIceServer {
                urls: vec![turn_url.clone()],
                ..Default::default()
            });
        }
        
        let config = RTCConfiguration {
            ice_servers,
            ..Default::default()
        };
        
        let peer_connection = Arc::new(self.api.new_peer_connection(config).await?);
        
        // Set up ICE candidate handler
        let signaling_tx = self.signaling_tx.clone();
        let active_call = Arc::clone(&self.active_call);
        
        peer_connection.on_ice_candidate(Box::new(move |candidate| {
            let signaling_tx = signaling_tx.clone();
            let active_call = Arc::clone(&active_call);
            
            Box::pin(async move {
                if let Some(candidate) = candidate {
                    let call_lock = active_call.read().await;
                    if let Some(ref call) = *call_lock {
                        if let Some(ref tx) = signaling_tx {
                            // Serialize ICE candidate
                            let candidate_str = match candidate.to_json() {
                                Ok(init) => init.candidate,
                                Err(_) => return,
                            };
                            
                            let signaling_msg = SignalingMessage::IceCandidate {
                                candidate: candidate_str,
                                sdp_mid: None,
                                sdp_mline_index: None,
                                session_id: call.session_id.clone(),
                            };
                            let _ = tx.send((call.peer_id.clone(), signaling_msg));
                            debug!("Sent ICE candidate for session {}", call.session_id);
                        }
                    }
                } else {
                    // ICE gathering complete
                    let call_lock = active_call.read().await;
                    if let Some(ref call) = *call_lock {
                        if let Some(ref tx) = signaling_tx {
                            let signaling_msg = SignalingMessage::IceComplete {
                                session_id: call.session_id.clone(),
                            };
                            let _ = tx.send((call.peer_id.clone(), signaling_msg));
                            debug!("ICE gathering complete for session {}", call.session_id);
                        }
                    }
                }
            })
        }));
        
        // Set up connection state handler
        let active_call_clone = Arc::clone(&self.active_call);
        peer_connection.on_peer_connection_state_change(Box::new(move |state| {
            let active_call = Arc::clone(&active_call_clone);
            
            Box::pin(async move {
                info!("Peer connection state changed: {:?}", state);
                
                match state {
                    RTCPeerConnectionState::Connected => {
                        let mut call_lock = active_call.write().await;
                        if let Some(ref mut call) = *call_lock {
                            call.state = CallState::Connected;
                            info!("Call connected with peer {}", call.peer_id);
                        }
                    }
                    RTCPeerConnectionState::Disconnected | RTCPeerConnectionState::Failed | RTCPeerConnectionState::Closed => {
                        let mut call_lock = active_call.write().await;
                        if let Some(ref mut call) = *call_lock {
                            call.state = CallState::Ended;
                            info!("Call ended with peer {}", call.peer_id);
                        }
                    }
                    _ => {}
                }
            })
        }));
        
        // Set up track handler for incoming audio
        peer_connection.on_track(Box::new(move |track, _receiver, _transceiver| {
            Box::pin(async move {
                let codec = track.codec();
                info!("Received track: {} ({})", track.kind(), codec.capability.mime_type);
                
                // Spawn task to read and process incoming audio
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 1500];
                    while let Ok((n, _attr)) = track.read(&mut buf).await {
                        // Here you would process the audio data
                        // For now, just count bytes received
                        debug!("Received {} bytes of audio data", n);
                    }
                });
            })
        }));
        
        Ok(peer_connection)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_voice_manager_creation() {
        let manager = VoiceManager::new();
        assert!(manager.is_ok());
    }
    
    #[tokio::test]
    async fn test_call_config_default() {
        let config = CallConfig::default();
        assert_eq!(config.channels, 1); // Mono
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.bitrate, 64000);
        assert!(!config.stun_servers.is_empty());
    }
    
    #[tokio::test]
    async fn test_initial_state() {
        let manager = VoiceManager::new().unwrap();
        assert_eq!(manager.get_call_state().await, CallState::Idle);
        assert!(!manager.has_active_call().await);
        assert!(manager.get_current_peer().await.is_none());
    }
}
