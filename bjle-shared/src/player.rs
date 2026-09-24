use crate::Card;
use crate::utils::calculate_score;
use rand::{Rng, rng};
use sha2::{Digest, Sha256};

pub type PlayerId = u8;

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

pub struct PlayerView {
    pub id: PlayerId,
    pub hand: Vec<Card>,
    pub score: u8,
    pub fishes: u16,
    pub stood: bool,
    pub is_bust: bool,
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

    pub fn view(&self) -> PlayerView {
        let score = calculate_score(&self.hand);
        PlayerView {
            id: self.id,
            hand: self.hand.clone(),
            score,
            fishes: self.fishes,
            stood: self.stood,
            is_bust: score > 21,
        }
    }
}
