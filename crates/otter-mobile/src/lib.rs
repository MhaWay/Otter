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
use serde::{Serialize, Deserialize};

use otter_identity::Identity;

/// FFI callback type for network events
pub type NetworkEventCallback = extern "C" fn(*const c_char);

/// Serializable event for FFI
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MobileNetworkEvent {
    pub event_type: String,
    pub data: serde_json::Value,
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
    std::ffi::CString::new(version.to_string())
        .unwrap()
        .into_raw()
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
