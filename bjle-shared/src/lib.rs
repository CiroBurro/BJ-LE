use rand::{Rng, rng};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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

impl Card {
    pub fn new(suit: Suit, value: u8) -> Self {
        Self { suit, value }
    }
}

// Azioni di gioco
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Hit,
    Stand,
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

pub type PlayerId = u8;
pub type RoomId = u8;

pub struct Player {
    pub id: PlayerId,
    pub hand: Vec<Card>,
    pub split_hand: Option<Vec<Card>>,
    pub fishes: u16,
    pub hash: [u8; 16],
    /// Giocatore ha piazzato la puntata ed è pronto.
    pub ready: bool,
    /// Giocatore ha fatto Stand (o bust).
    pub stood: bool,
}

impl Player {
    /// Crea il player locale: genera seed casuale, restituisce (Player, seed).
    /// Il seed va conservato localmente dall'engine — non entra nello stato.
    pub fn new(id: PlayerId) -> (Self, [u8; 16]) {
        let mut seed = [0u8; 16];
        rng().fill_bytes(seed.as_mut());

        let mut hasher = Sha256::new();
        hasher.update(seed);
        let full_hash = hasher.finalize();
        let mut hash = [0u8; 16];
        hash.copy_from_slice(&full_hash[0..16]);

        (
            Self {
                id,
                hand: Vec::new(),
                split_hand: None,
                fishes: 1000,
                hash,
                ready: false,
                stood: false,
            },
            seed,
        )
    }

    /// Costruisce un Player dai dati ricevuti via mesh (Join altrui).
    pub fn new_from_hash(id: PlayerId, hash: [u8; 16]) -> Self {
        Self {
            id,
            hand: Vec::new(),
            split_hand: None,
            fishes: 1000,
            hash,
            ready: false,
            stood: false,
        }
    }

    pub fn verify_hash(seed: [u8; 16], hash: [u8; 16]) -> bool {
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let full_hash = hasher.finalize();
        &full_hash[0..16] == hash
    }

    pub fn add_card(&mut self, card: Card, split_hand: bool) {
        if split_hand {
            self.split_hand.as_mut().unwrap().push(card);
        } else {
            self.hand.push(card);
        }
    }
}

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
