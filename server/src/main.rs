use flight_shared::{decode, encode, ClientMessage, PlayerState, ServerMessage};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;

const SERVER_PORT: u16 = 7777;
const MAX_PACKET_SIZE: usize = 1024;
const PLAYER_TIMEOUT_SECS: u64 = 5;

struct Player {
    addr: SocketAddr,
    name: String,
    last_state: Option<PlayerState>,
    last_activity: Instant,
}

struct GameState {
    players: HashMap<u32, Player>,
    next_player_id: u32,
    addr_to_id: HashMap<SocketAddr, u32>,
}

impl GameState {
    fn new() -> Self {
        Self {
            players: HashMap::new(),
            next_player_id: 1,
            addr_to_id: HashMap::new(),
        }
    }

    fn add_player(&mut self, addr: SocketAddr, name: String) -> u32 {
        let id = self.next_player_id;
        self.next_player_id += 1;
        self.players.insert(
            id,
            Player {
                addr,
                name: name.clone(),
                last_state: None,
                last_activity: Instant::now(),
            },
        );
        self.addr_to_id.insert(addr, id);
        println!("Player '{}' joined with ID {} from {}", name, id, addr);
        id
    }

    fn remove_player_by_id(&mut self, id: u32) -> Option<SocketAddr> {
        if let Some(player) = self.players.remove(&id) {
            self.addr_to_id.remove(&player.addr);
            println!("Player '{}' (ID {}) timed out", player.name, id);
            Some(player.addr)
        } else {
            None
        }
    }

    fn remove_player(&mut self, addr: &SocketAddr) -> Option<u32> {
        if let Some(id) = self.addr_to_id.remove(addr) {
            if let Some(player) = self.players.remove(&id) {
                println!("Player '{}' (ID {}) left", player.name, id);
            }
            Some(id)
        } else {
            None
        }
    }

    fn get_player_id(&self, addr: &SocketAddr) -> Option<u32> {
        self.addr_to_id.get(addr).copied()
    }

    fn update_player_state(&mut self, id: u32, state: PlayerState) {
        if let Some(player) = self.players.get_mut(&id) {
            player.last_state = Some(state);
            player.last_activity = Instant::now();
        }
    }

    fn touch_player(&mut self, id: u32) {
        if let Some(player) = self.players.get_mut(&id) {
            player.last_activity = Instant::now();
        }
    }

    fn get_other_players(&self, exclude_id: u32) -> Vec<(u32, SocketAddr)> {
        self.players
            .iter()
            .filter(|(&id, _)| id != exclude_id)
            .map(|(&id, p)| (id, p.addr))
            .collect()
    }

    fn get_all_player_addrs(&self) -> Vec<SocketAddr> {
        self.players.values().map(|p| p.addr).collect()
    }

    fn get_timed_out_players(&self, timeout: Duration) -> Vec<u32> {
        self.players
            .iter()
            .filter(|(_, p)| p.last_activity.elapsed() > timeout)
            .map(|(&id, _)| id)
            .collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // On Fly.io, we must bind to fly-global-services for UDP to work correctly
    // This ensures replies go out with the correct source address
    let bind_addr = if std::env::var("FLY_APP_NAME").is_ok() {
        format!("fly-global-services:{}", SERVER_PORT)
    } else {
        format!("0.0.0.0:{}", SERVER_PORT)
    };

    let socket = UdpSocket::bind(&bind_addr).await?;
    let socket = Arc::new(socket);
    let state = Arc::new(RwLock::new(GameState::new()));

    println!("Flight relay server listening on {}", bind_addr);

    // Spawn cleanup task
    let cleanup_socket = socket.clone();
    let cleanup_state = state.clone();
    tokio::spawn(async move {
        let timeout = Duration::from_secs(PLAYER_TIMEOUT_SECS);
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let mut state = cleanup_state.write().await;
            let timed_out = state.get_timed_out_players(timeout);

            for player_id in timed_out {
                if state.remove_player_by_id(player_id).is_some() {
                    // Notify all remaining players
                    let msg = ServerMessage::PlayerLeft { player_id };
                    let data = encode(&msg);

                    for addr in state.get_all_player_addrs() {
                        let _ = cleanup_socket.send_to(&data, addr).await;
                    }
                }
            }
        }
    });

    let mut buf = [0u8; MAX_PACKET_SIZE];

    loop {
        let (len, src) = socket.recv_from(&mut buf).await?;
        let data = &buf[..len];

        match decode::<ClientMessage>(data) {
            Ok(msg) => {
                handle_message(msg, src, &socket, &state).await;
            }
            Err(e) => {
                eprintln!("Failed to decode message from {}: {}", src, e);
            }
        }
    }
}

async fn handle_message(
    msg: ClientMessage,
    src: SocketAddr,
    socket: &Arc<UdpSocket>,
    state: &Arc<RwLock<GameState>>,
) {
    match msg {
        ClientMessage::Join { name } => {
            let mut state = state.write().await;

            // Check if already joined
            if let Some(id) = state.get_player_id(&src) {
                // Update activity time for existing player
                state.touch_player(id);
                return;
            }

            let player_id = state.add_player(src, name);

            // Send welcome message
            let welcome = ServerMessage::Welcome { player_id };
            let data = encode(&welcome);
            if let Err(e) = socket.send_to(&data, src).await {
                eprintln!("Failed to send welcome to {}: {}", src, e);
            }

            // Send existing players' states to new player
            for (_, player) in state.players.iter() {
                if let Some(ref player_state) = player.last_state {
                    let msg = ServerMessage::PlayerState(player_state.clone());
                    let data = encode(&msg);
                    let _ = socket.send_to(&data, src).await;
                }
            }
        }

        ClientMessage::StateUpdate(player_state) => {
            let mut state = state.write().await;

            if let Some(player_id) = state.get_player_id(&src) {
                // Update stored state
                let mut updated_state = player_state;
                updated_state.player_id = player_id;
                state.update_player_state(player_id, updated_state.clone());

                // Relay to all other players
                let others = state.get_other_players(player_id);
                let msg = ServerMessage::PlayerState(updated_state);
                let data = encode(&msg);

                for (_, addr) in others {
                    let _ = socket.send_to(&data, addr).await;
                }
            }
        }

        ClientMessage::Leave => {
            let mut state = state.write().await;

            if let Some(player_id) = state.remove_player(&src) {
                // Notify all remaining players
                let msg = ServerMessage::PlayerLeft { player_id };
                let data = encode(&msg);

                for (_, player) in state.players.iter() {
                    let _ = socket.send_to(&data, player.addr).await;
                }
            }
        }
    }
}
