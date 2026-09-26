use player::PlayerId;
use serde::{Deserialize, Serialize};
pub mod player;
pub mod utils;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Suit {
    Spades,
    Clubs,
    Diamonds,
    Hearts,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    pub suit: Suit,
    pub value: u8,
}

impl Card {
    pub fn new(suit: Suit, value: u8) -> Self {
        Self { suit, value }
    }
}

// Azioni di gioco
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Hit { split: bool },
    Stand,
    StandSplit, // Stand sulla mano split, continua sulla mano principale
    Double,
    Split,
}

// Azioni nella UI
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum UserAction {
    CreateRoom { room_id: RoomId },
    JoinRoom { room_id: RoomId },
    StartGame,
    PlaceBet { amount: u16 },
    ConfirmReady,
    Quit,
    Play(Action),
}

pub type RoomId = u8;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GamePhase {
    Lobby,
    Revealing,
    Betting,
    Playing,
    Done,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    /// Peer entra nella room; porta subito l'hash del suo seed (commit implicito).
    Join {
        player_id: PlayerId,
        room_id: RoomId,
        hash: [u8; 16],
    },
    Leave {
        player_id: PlayerId,
    },
    /// Il creatore della room chiude la lobby e avvia la fase reveal.
    StartGame,
    /// Ogni peer rivela il seed dopo StartGame; gli altri verificano hash == SHA256(seed)[0..16].
    RevealSeed {
        player_id: PlayerId,
        seed: [u8; 16],
    },
    /// Late joiner chiede la lista dei player già in lobby.
    RequestSync {
        player_id: PlayerId,
    },
    /// Risposta al RequestSync: lista (id, hash) dei player presenti.
    /// Solo il nodo con player_id minimo risponde, per evitare flood.
    SyncResponse {
        players: Vec<(PlayerId, [u8; 16])>,
    },
    /// Peer ha piazzato la puntata ed è pronto a iniziare il turno.
    TurnReady {
        player_id: PlayerId,
    },
    PlayerAction {
        player_id: PlayerId,
        action: Action,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct MeshMessage {
    pub magic: u8,
    pub msg_id: u32,
    pub room_id: RoomId,
    pub ttl: u8,
    pub event: GameEvent,
}
