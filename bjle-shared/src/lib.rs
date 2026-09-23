use serde::{Deserialize, Serialize};

pub enum Suit {
    Spades,
    Clubs,
    Diamonds,
    Hearts,
}

pub struct Card {
    pub suit: Suit,
    pub value: u8,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Hit,
    Stand,
    Double,
    Split,
}

pub type PlayerId = u8;
pub type RoomId = u8;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum GameEvent {
    Join {
        player_id: PlayerId,
        room_id: RoomId,
    },
    Leave {
        player_id: PlayerId,
    },
    PlayerAction {
        player_id: PlayerId,
        action: Action,
    },
    CommitSeed {
        hash: [u8; 16],
    },
    RevealSeed {
        seed: [u8; 16],
    },
    TurnReady {
        player_id: PlayerId,
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
