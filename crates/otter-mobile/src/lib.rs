//! # Otter Mobile
//!
//! FFI layer for mobile platforms (iOS/Android)
//! Exposes P2P network core to Flutter/native mobile apps via C ABI
//!
//! Architecture:
//! - Core P2P network (libp2p, DHT, Gossipsub) - SHARED with desktop
//! - Async Tokio runtime for network operations
//! - FFI callbacks for UI events
//! - JSON serialization for cross-language communication

use std::os::raw::c_char;
use std::sync::{Arc, Mutex, OnceLock};
use std::ffi::{CStr, CString};
use serde::{Serialize, Deserialize};
use base64::Engine;

use otter_identity::Identity;
use otter_network::{Network, NetworkEvent};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

/// Global Tokio runtime for async operations
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

/// Global network instance
static NETWORK: OnceLock<Arc<Mutex<Option<Network>>>> = OnceLock::new();

/// Global event receiver
static EVENT_RX: OnceLock<Arc<Mutex<Option<mpsc::Receiver<NetworkEvent>>>>> = OnceLock::new();

/// Global event callback
static EVENT_CALLBACK: OnceLock<Arc<Mutex<Option<NetworkEventCallback>>>> = OnceLock::new();

/// FFI callback type for network events
pub type NetworkEventCallback = extern "C" fn(*const c_char);

/// Serializable event for FFI
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MobileNetworkEvent {
    pub event_type: String,
    pub data: serde_json::Value,
}

/// Get or initialize runtime
fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        Runtime::new().expect("Failed to create Tokio runtime")
    })
}

/// Get network instance
fn get_network() -> Arc<Mutex<Option<Network>>> {
    NETWORK.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

/// Get event receiver
fn get_event_rx() -> Arc<Mutex<Option<mpsc::Receiver<NetworkEvent>>>> {
    EVENT_RX.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

/// Get event callback
fn get_event_callback() -> Arc<Mutex<Option<NetworkEventCallback>>> {
    EVENT_CALLBACK.get_or_init(|| Arc::new(Mutex::new(None))).clone()
}

/// Send event to callback if registered
fn send_event(event_type: &str, data: serde_json::Value) {
    let callback_lock = get_event_callback();
    let callback_guard = callback_lock.lock();
    
    if let Ok(callback_opt) = callback_guard {
        if let Some(callback) = *callback_opt {
            let event = MobileNetworkEvent {
                event_type: event_type.to_string(),
                data,
            };
            if let Ok(json) = serde_json::to_string(&event) {
                if let Ok(c_str) = CString::new(json) {
                    callback(c_str.as_ptr());
                }
            }
        }
    }
}

/// Generate new identity
/// Returns: JSON with peer_id, public_key, etc.
#[no_mangle]
pub extern "C" fn otter_mobile_generate_identity() -> *const c_char {
    match Identity::generate() {
        Ok(identity) => {
            let response = serde_json::json!({
                "peer_id": identity.peer_id().to_string(),
                "success": true,
            });
            std::ffi::CString::new(response.to_string())
                .unwrap()
                .into_raw()
        }
        Err(e) => {
            let error = format!("Identity generation error: {}", e);
            std::ffi::CString::new(error).unwrap().into_raw()
        }
    }
}

/// Free C string allocated by Rust
#[no_mangle]
pub extern "C" fn otter_mobile_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe {
            let _ = std::ffi::CString::from_raw(ptr);
        }
    }
}

/// Get network version
/// Returns: JSON with version info
#[no_mangle]
pub extern "C" fn otter_mobile_get_version() -> *const c_char {
    let version = serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": "Otter Mobile",
    });
    CString::new(version.to_string())
        .unwrap()
        .into_raw()
}

/// Register event callback
/// Flutter will call this to receive network events
#[no_mangle]
pub extern "C" fn otter_mobile_register_callback(callback: NetworkEventCallback) -> *const c_char {
    let callback_lock = get_event_callback();
    if let Ok(mut cb) = callback_lock.lock() {
        *cb = Some(callback);
        let response = serde_json::json!({
            "success": true,
            "message": "Callback registered"
        });
        return CString::new(response.to_string()).unwrap().into_raw();
    }
    
    let error = serde_json::json!({
        "success": false,
        "error": "Failed to register callback"
    });
    CString::new(error.to_string()).unwrap().into_raw()
}

/// Start network with identity
/// Returns: JSON with status
#[no_mangle]
pub extern "C" fn otter_mobile_start_network(identity_json: *const c_char) -> *const c_char {
    if identity_json.is_null() {
        let error = serde_json::json!({
            "success": false,
            "error": "Null identity JSON pointer"
        });
        return CString::new(error.to_string()).unwrap().into_raw();
    }
    
    let identity_str = unsafe {
        match CStr::from_ptr(identity_json).to_str() {
            Ok(s) => s,
            Err(_) => {
                let error = serde_json::json!({
                    "success": false,
                    "error": "Invalid UTF-8 in identity JSON"
                });
                return CString::new(error.to_string()).unwrap().into_raw();
            }
        }
    };
    
    // Parse identity or generate new one
    let identity = match serde_json::from_str::<serde_json::Value>(identity_str) {
        Ok(json) => {
            if json.get("peer_id").and_then(|v| v.as_str()).is_some() {
                // TODO: Load existing identity from storage
                // For now, generate new
                match Identity::generate() {
                    Ok(id) => id,
                    Err(e) => {
                        let error = serde_json::json!({
                            "success": false,
                            "error": format!("Identity generation failed: {}", e)
                        });
                        return CString::new(error.to_string()).unwrap().into_raw();
                    }
                }
            } else {
                match Identity::generate() {
                    Ok(id) => id,
                    Err(e) => {
                        let error = serde_json::json!({
                            "success": false,
                            "error": format!("Identity generation failed: {}", e)
                        });
                        return CString::new(error.to_string()).unwrap().into_raw();
                    }
                }
            }
        }
        Err(_) => {
            match Identity::generate() {
                Ok(id) => id,
                Err(e) => {
                    let error = serde_json::json!({
                        "success": false,
                        "error": format!("Identity generation failed: {}", e)
                    });
                    return CString::new(error.to_string()).unwrap().into_raw();
                }
            }
        }
    };
    
    let peer_id = identity.peer_id();
    
    // Initialize network
    let runtime = get_runtime();
    let network_lock = get_network();
    let event_rx_lock = get_event_rx();
    
    let result = runtime.block_on(async {
        let (event_tx, event_rx) = mpsc::channel(1000);
        let (_command_tx, command_rx) = mpsc::channel(100);
        
        match Network::new(event_tx, command_rx) {
            Ok(network) => {
                let mut net = network_lock.lock().unwrap();
                *net = Some(network);
                
                let mut rx = event_rx_lock.lock().unwrap();
                *rx = Some(event_rx);
                
                // Send network started event
                send_event("network_started", serde_json::json!({
                    "peer_id": peer_id.to_string()
                }));
                
                Ok(peer_id.to_string())
            }
            Err(e) => Err(format!("Network init failed: {}", e))
        }
    });
    
    match result {
        Ok(peer_id_str) => {
            let runtime = get_runtime();
            let network_lock_clone = network_lock.clone();
            
            // Spawn network run task
            runtime.spawn(async move {
                let network_opt = {
                    let mut lock = network_lock_clone.lock().unwrap();
                    lock.take()
                };
                
                if let Some(network) = network_opt {
                    match network.run().await {
                        Ok(_) => println!("Network task completed"),
                        Err(e) => eprintln!("Network error: {}", e),
                    }
                }
            });
            
            // Spawn event listener
            runtime.spawn(async move {
                event_listener_task().await;
            });
            
            let response = serde_json::json!({
                "success": true,
                "peer_id": peer_id_str
            });
            CString::new(response.to_string()).unwrap().into_raw()
        }
        Err(e) => {
            let error = serde_json::json!({
                "success": false,
                "error": e
            });
            CString::new(error.to_string()).unwrap().into_raw()
        }
    }
}

/// Event listener task - forwards NetworkEvents to callback
async fn event_listener_task() {
    let event_rx_lock = get_event_rx();
    
    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        let mut rx_opt = event_rx_lock.lock().unwrap();
        if let Some(rx) = rx_opt.as_mut() {
            if let Ok(event) = rx.try_recv() {
                match event {
                    NetworkEvent::PeerDiscovered { peer_id, addresses } => {
                        send_event("peer_connected", serde_json::json!({
                            "peer_id": peer_id.to_string(),
                            "addresses": addresses
                        }));
                    }
                    NetworkEvent::PeerOffline { peer_id } => {
                        send_event("peer_disconnected", serde_json::json!({
                            "peer_id": peer_id.to_string()
                        }));
                    }
                    NetworkEvent::MessageReceived { from, data } => {
                        send_event("message", serde_json::json!({
                            "from": from.to_string(),
                            "topic": "otter-global",
                            "data": base64::engine::general_purpose::STANDARD.encode(&data)
                        }));
                    }
                    NetworkEvent::NetworkReady { mesh_peer_count } => {
                        send_event("network_ready", serde_json::json!({
                            "peer_count": mesh_peer_count
                        }));
                    }
                    NetworkEvent::PeerOnline { peer_id, nickname, .. } => {
                        send_event("peer_connected", serde_json::json!({
                            "peer_id": peer_id.to_string(),
                            "nickname": nickname.unwrap_or_else(|| "Unknown".to_string())
                        }));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Get connected peers
/// Returns: JSON array of peer info
#[no_mangle]
pub extern "C" fn otter_mobile_get_peers() -> *const c_char {
    let network_lock = get_network();
    let network_opt = network_lock.lock().unwrap();
    
    if let Some(_network) = network_opt.as_ref() {
        // For now, return simplified peer list
        // In future, implement proper peer tracking or query via command channel
        let peers: Vec<serde_json::Value> = vec![];
        
        let response = serde_json::json!({
            "success": true,
            "peers": peers
        });
        return CString::new(response.to_string()).unwrap().into_raw();
    }
    
    let error = serde_json::json!({
        "success": false,
        "error": "Network not started"
    });
    CString::new(error.to_string()).unwrap().into_raw()
}

/// Send message to topic
#[no_mangle]
pub extern "C" fn otter_mobile_send_message(_topic: *const c_char, _message: *const c_char) -> *const c_char {
    // TODO: Implement message sending via gossipsub
    // For now, just simulate success
    let response = serde_json::json!({
        "success": true,
        "note": "Message sending not yet fully implemented"
    });
    CString::new(response.to_string()).unwrap().into_raw()
}

/// Stop network
#[no_mangle]
pub extern "C" fn otter_mobile_stop_network() -> *const c_char {
    let network_lock = get_network();
    let mut network_opt = network_lock.lock().unwrap();
    *network_opt = None;
    
    send_event("network_stopped", serde_json::json!({}));
    
    let response = serde_json::json!({
        "success": true
    });
    CString::new(response.to_string()).unwrap().into_raw()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_generation() {
        let result = otter_mobile_generate_identity();
        assert!(!result.is_null());
        unsafe {
            let c_str = CStr::from_ptr(result);
            let json_str = c_str.to_str().unwrap();
            let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
            assert!(json["success"].as_bool().unwrap());
            assert!(!json["peer_id"].as_str().unwrap().is_empty());
            otter_mobile_free_string(result as *mut c_char);
        }
    }

    #[test]
    fn test_version() {
        let result = otter_mobile_get_version();
        assert!(!result.is_null());
        unsafe {
            let c_str = CStr::from_ptr(result);
            let json_str = c_str.to_str().unwrap();
            let json: serde_json::Value = serde_json::from_str(json_str).unwrap();
            assert!(!json["version"].as_str().unwrap().is_empty());
            otter_mobile_free_string(result as *mut c_char);
        }
    }
}
