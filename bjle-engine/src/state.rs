use crate::utils::{calculate_score, generate_deck};
use bjle_shared::{Card, GameEvent, GamePhase, Player, PlayerId};

pub struct State {
    pub phase: GamePhase,
    pub deck: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub players: Vec<Player>,
    /// Seed rivelati finora: (player_id, seed). Popolato durante Revealing.
    revealed_seeds: Vec<(PlayerId, [u8; 16])>,
    /// player_id locale — serve per decidere se rispondere a RequestSync.
    pub local_player_id: PlayerId,
}

impl State {
    pub fn new(local_player_id: PlayerId) -> Self {
        Self {
            phase: GamePhase::Lobby,
            deck: Vec::new(),
            dealer_hand: Vec::new(),
            players: Vec::new(),
            revealed_seeds: Vec::new(),
            local_player_id,
        }
    }

    /// Applica un evento allo stato.
    /// Ok(Some(event)) → trasmetti questo evento sulla mesh.
    /// Ok(None)        → nessuna risposta da trasmettere.
    /// Err(msg)        → evento invalido, ignora.
    pub fn apply(&mut self, event: GameEvent) -> Result<Option<GameEvent>, &'static str> {
        match (&self.phase, event) {
            // ── LOBBY ────────────────────────────────────────────────────────
            (
                GamePhase::Lobby,
                GameEvent::Join {
                    player_id, hash, ..
                },
            ) => {
                if self.players.iter().any(|p| p.id == player_id) {
                    return Err("player già presente");
                }
                self.players.push(Player::new_from_hash(player_id, hash));
                Ok(None)
            }
            (GamePhase::Lobby, GameEvent::Leave { player_id }) => {
                self.players.retain(|p| p.id != player_id);
                Ok(None)
            }
            // Late joiner chiede la lista. Risponde solo il nodo con id minimo,
            // così un solo SyncResponse arriva al richiedente (no flood).
            (GamePhase::Lobby, GameEvent::RequestSync { .. }) => {
                let min_id = self.players.iter().map(|p| p.id).min();
                if min_id == Some(self.local_player_id) {
                    let players = self.players.iter().map(|p| (p.id, p.hash)).collect();
                    Ok(Some(GameEvent::SyncResponse { players }))
                } else {
                    Ok(None)
                }
            }
            // Il late joiner riceve la lista e popola i player mancanti.
            (GamePhase::Lobby, GameEvent::SyncResponse { players }) => {
                for (id, hash) in players {
                    if !self.players.iter().any(|p| p.id == id) {
                        self.players.push(Player::new_from_hash(id, hash));
                    }
                }
                Ok(None)
            }
            // Il creatore chiude la lobby → tutti devono rivelare il seed.
            (GamePhase::Lobby, GameEvent::StartGame) => {
                if self.players.len() < 2 {
                    return Err("servono almeno 2 giocatori");
                }
                self.phase = GamePhase::Revealing;
                Ok(None)
            }

            // ── REVEALING ────────────────────────────────────────────────────
            (GamePhase::Revealing, GameEvent::RevealSeed { player_id, seed }) => {
                let player = self
                    .players
                    .iter()
                    .find(|p| p.id == player_id)
                    .ok_or("player sconosciuto")?;

                if !Player::verify_hash(seed, player.hash) {
                    return Err("seed non corrisponde all'hash committato");
                }
                if self.revealed_seeds.iter().any(|(id, _)| *id == player_id) {
                    return Err("seed già rivelato");
                }

                self.revealed_seeds.push((player_id, seed));

                // Tutti hanno rivelato → genera il mazzo e passa a Betting.
                if self.revealed_seeds.len() == self.players.len() {
                    self.init_deck();
                    self.phase = GamePhase::Betting;
                }
                Ok(None)
            }

            // ── BETTING ──────────────────────────────────────────────────────
            (GamePhase::Betting, GameEvent::TurnReady { player_id }) => {
                let player = self
                    .players
                    .iter_mut()
                    .find(|p| p.id == player_id)
                    .ok_or("player sconosciuto")?;
                player.ready = true;

                if self.players.iter().all(|p| p.ready) {
                    self.start();
                    self.phase = GamePhase::Playing;
                }
                Ok(None)
            }

            // ── PLAYING ──────────────────────────────────────────────────────
            (GamePhase::Playing, GameEvent::PlayerAction { player_id, action }) => {
                use bjle_shared::Action;
                let player = self
                    .players
                    .iter_mut()
                    .find(|p| p.id == player_id)
                    .ok_or("player sconosciuto")?;
                match action {
                    Action::Hit => {
                        let card = self.deck.remove(0);
                        player.add_card(card, false);
                        // bust: turno finisce automaticamente
                    }
                    Action::Stand => {
                        player.stood = true;
                    }
                    _ => {} // ponytail: Double/Split, aggiungere quando serve
                }

                if self
                    .players
                    .iter()
                    .all(|p| p.stood || calculate_score(&p.hand) > 21)
                {
                    self.play_dealer_turn();
                    self.phase = GamePhase::Done;
                }
                Ok(None)
            }
            (GamePhase::Playing, GameEvent::Leave { player_id }) => {
                self.players.retain(|p| p.id != player_id);
                Ok(None)
            }

            _ => Err("evento non valido per la fase corrente"),
        }
    }

    // ── Helpers privati ──────────────────────────────────────────────────────

    fn init_deck(&mut self) {
        // Ordina per id prima dell'XOR: stesso ordine su tutti i nodi
        // indipendentemente dall'ordine di arrivo dei Join.
        self.players.sort_by_key(|p| p.id);
        self.revealed_seeds.sort_by_key(|(id, _)| *id);

        let mut final_seed = [0u8; 32];
        for (_, s) in &self.revealed_seeds {
            for i in 0..16 {
                final_seed[i] ^= s[i];
                final_seed[i + 16] ^= s[i];
            }
        }
        self.deck = generate_deck(final_seed);
    }

    fn start(&mut self) {
        for _ in 0..2 {
            for p in self.players.iter_mut() {
                let card = self.deck.remove(0);
                p.add_card(card, false);
            }
        }
        // Dealer: una carta scoperta all'inizio
        self.dealer_hand.push(self.deck.remove(0));
    }

    fn play_dealer_turn(&mut self) {
        // Seconda carta del dealer (era coperta)
        self.dealer_hand.push(self.deck.remove(0));
        while calculate_score(&self.dealer_hand) < 17 {
            self.dealer_hand.push(self.deck.remove(0));
        }
    }
}

pub struct StateView {}
