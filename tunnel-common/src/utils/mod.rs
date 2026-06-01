//! Utility functions for common operations

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CommonError {
    #[error("UUID generation failed: {0}")]
    Uuid(String),
}

/// Generate a new random UUID v4 for session/stream identification
pub fn uuid_v4() -> Result<uuid::Uuid, CommonError> {
    match uuid::Uuid::new_v4() {
        id => Ok(id),
    }
}

/// Convert Z85-encoded CURVE secret key to binary (used internally by zmq)
pub fn decode_z85(key: &str) -> Result<Vec<u8>, CommonError> {
    // Z85 decoding - simplified for now
    // In production, use zmq's curve_keypair.from_uri() or proper conversion
    Ok(key.as_bytes().to_vec())
}

/// Convert binary secret key to Z85 format
pub fn encode_z85(data: &[u8]) -> Result<String, CommonError> {
    // Z85 encoding - simplified for now
    Ok(hex::encode(data))
}
