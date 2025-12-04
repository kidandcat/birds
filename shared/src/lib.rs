use serde::{Deserialize, Serialize};

/// Bird type for network sync
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum NetBirdType {
    Sparrow,
    Hawk,
    Eagle,
    Albatross,
}

/// Player state sent over network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerState {
    /// Unique player ID (assigned by server)
    pub player_id: u32,
    /// Position in world space
    pub position: [f32; 3],
    /// Rotation as quaternion [x, y, z, w]
    pub rotation: [f32; 4],
    /// Current velocity
    pub velocity: [f32; 3],
    /// Bird type
    pub bird_type: NetBirdType,
    /// Is wings closed (space held)
    pub wings_closed: bool,
    /// Is on ground
    pub grounded: bool,
    /// Is walking
    pub is_walking: bool,
    /// Flap animation timer (for wing animation sync)
    pub flap_timer: f32,
}

/// Messages from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClientMessage {
    /// Request to join the game
    Join { name: String },
    /// Player state update
    StateUpdate(PlayerState),
    /// Player leaving
    Leave,
}

/// Messages from server to client
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServerMessage {
    /// Welcome message with assigned player ID
    Welcome { player_id: u32 },
    /// Another player's state (relayed)
    PlayerState(PlayerState),
    /// Player disconnected
    PlayerLeft { player_id: u32 },
}

/// Encode a message to bytes
pub fn encode<T: Serialize>(msg: &T) -> Vec<u8> {
    bincode::serialize(msg).expect("Failed to serialize message")
}

/// Decode a message from bytes
pub fn decode<'a, T: Deserialize<'a>>(bytes: &'a [u8]) -> Result<T, bincode::Error> {
    bincode::deserialize(bytes)
}
