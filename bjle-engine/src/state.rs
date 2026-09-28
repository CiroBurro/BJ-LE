use bjle_shared::{Action, Card, GameEvent, GamePhase, player::*, utils::*};

pub struct State {
    pub phase: GamePhase,
    pub deck: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub players: Vec<Player>,
    pub player_idx: usize,
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
            player_idx: 0,
            revealed_seeds: Vec::new(),
            local_player_id,
        }
    }

    pub fn view(&self) -> StateView {
        let dealer_score = calculate_score(&self.dealer_hand);
        StateView {
            phase: self.phase.clone(),
            local_player_id: self.local_player_id,
            players: self.players.iter().map(|p| p.view()).collect(),
            player_idx: self.player_idx,
            dealer_hand: self.dealer_hand.clone(),
            dealer_score,
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
            (GamePhase::Betting, GameEvent::PlaceBet { player_id, amount }) => {
                let player = self
                    .players
                    .iter_mut()
                    .find(|p| p.id == player_id)
                    .ok_or("player sconosciuto")?;

                if player.fishes < amount {
                    return Err("Fishes insufficienti per la puntata selezionata");
                }
                player.bet = amount;
                player.fishes -= amount;
                Ok(None)
            }
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
                // Valida che sia il turno del player corretto.
                if self.player_idx >= self.players.len() {
                    return Err("indice player fuori bounds");
                }
                if self.players[self.player_idx].id != player_id {
                    return Err("non è il tuo turno");
                }

                let player = self
                    .players
                    .iter_mut()
                    .find(|p| p.id == player_id)
                    .ok_or("player sconosciuto")?;

                match action {
                    Action::Hit { split } => {
                        if split {
                            if player.split_hand.is_none() {
                                return Err("nessuna mano split su cui pescare");
                            }
                            if player.split_stood {
                                return Err("mano split già conclusa");
                            }
                            let card = self.deck.remove(0);
                            player.add_card(card, true);
                            // bust sulla split: chiude la mano split automaticamente
                            if calculate_score(player.split_hand.as_ref().unwrap()) > 21 {
                                player.split_stood = true;
                            }
                        } else {
                            if player.stood {
                                return Err("mano principale già conclusa");
                            }
                            let card = self.deck.remove(0);
                            player.add_card(card, false);
                            // bust sulla principale: chiude la mano principale automaticamente
                            if calculate_score(&player.hand) > 21 {
                                player.stood = true;
                            }
                        }
                    }
                    Action::Stand => {
                        player.stood = true;
                        // Se non ha la split, anche split_stood = true così la
                        // condizione finale è uniforme.
                        if player.split_hand.is_none() {
                            player.split_stood = true;
                        }
                    }
                    Action::StandSplit => {
                        if player.split_hand.is_none() {
                            return Err("nessuna mano split");
                        }
                        player.split_stood = true;
                    }
                    Action::Double => {
                        if player.fishes < player.bet {
                            return Err("fishes insufficienti per raddoppiare");
                        }
                        if player.hand.len() != 2 {
                            return Err("double solo con 2 carte in mano");
                        }
                        player.fishes -= player.bet;
                        player.bet *= 2;
                        let card = self.deck.remove(0);
                        player.add_card(card, false);
                        // Double implica stand obbligatorio dopo la carta.
                        player.stood = true;
                        if player.split_hand.is_none() {
                            player.split_stood = true;
                        }
                    }
                    Action::Split => {
                        if player.hand.len() != 2 {
                            return Err("split solo con 2 carte in mano");
                        }
                        if player.hand[0].value != player.hand[1].value {
                            return Err("split solo con carte dello stesso valore");
                        }
                        if player.split_hand.is_some() {
                            return Err("split già effettuato");
                        }
                        if player.fishes < player.bet {
                            return Err("fishes insufficienti per lo split");
                        }
                        player.fishes -= player.bet; // seconda puntata uguale alla prima
                        player.bet *= 2;
                        // Sposta la seconda carta nella mano split.
                        let second = player.hand.pop().unwrap();
                        player.split_hand = Some(vec![second]);
                        // Pesca una carta per ciascuna mano.
                        let card_main = self.deck.remove(0);
                        player.add_card(card_main, false);
                        let card_split = self.deck.remove(0);
                        player.add_card(card_split, true);
                    }
                }

                // Il turno del giocatore è finito quando entrambe le mani
                // (o l'unica mano se non ha splittato) sono stood/bust.
                let player_done = |p: &Player| {
                    let main_done = p.stood || calculate_score(&p.hand) > 21;
                    let split_done = p.split_hand.is_none()
                        || p.split_stood
                        || calculate_score(p.split_hand.as_ref().unwrap()) > 21;
                    main_done && split_done
                };

                // Se il player corrente ha finito, avanza al prossimo.
                if player_done(&self.players[self.player_idx]) {
                    self.advance_turn();
                }

                // Se tutti hanno finito, dealer gioca e finisce la partita.
                if self.players.iter().all(|p| player_done(p)) {
                    self.play_dealer_turn();
                    self.phase = GamePhase::Done;
                }
                Ok(None)
            }
            (GamePhase::Playing, GameEvent::Leave { player_id }) => {
                self.players.retain(|p| p.id != player_id);
                Ok(None)
            }

            // ── DONE ──────────────────────────────────────────────────────
            (GamePhase::Done, _) => {
                self.payout();
                Ok(None)
            }

            _ => Err("evento non valido per la fase corrente"),
        }
    }

    pub fn payout(&mut self) {
        let dealer_score = calculate_score(&self.dealer_hand);
        let dealer_blackjack = self.dealer_hand.len() == 2 && dealer_score == 21;

        for p in self.players.iter_mut() {
            // Mano principale
            let main_score = calculate_score(&p.hand);
            let main_blackjack = p.hand.len() == 2 && main_score == 21;
            let bet_per_hand = p.bet / 2; // dopo split bet è doppio; se no split, bet/2 = bet_original
            let main_bet = if p.split_hand.is_some() { bet_per_hand } else { p.bet };

            p.fishes = p.fishes.saturating_add_signed(Self::hand_result(main_score, dealer_score, main_blackjack && !dealer_blackjack, main_bet));

            // Mano split (se presente)
            if let Some(ref split_hand) = p.split_hand {
                let split_score = calculate_score(split_hand);
                let split_blackjack = split_hand.len() == 2 && split_score == 21;
                // La seconda puntata è uguale alla prima
                p.fishes = p.fishes.saturating_add_signed(Self::hand_result(split_score, dealer_score, split_blackjack && !dealer_blackjack, bet_per_hand));
            }
        }
    }

    /// Calcola il delta fishes per una singola mano.
    /// Ritorna valore positivo (vincita netta), negativo (perdita), o 0 (push).
    fn hand_result(player_score: u8, dealer_score: u8, is_blackjack: bool, bet: u16) -> i16 {
        if player_score > 21 {
            return -(bet as i16); // bust
        }
        if dealer_score > 21 || dealer_score < player_score {
            if is_blackjack {
                return (bet as i16 * 3) / 2; // blackjack paga 3:2
            }
            return bet as i16; // win 1:1
        }
        if dealer_score == player_score {
            return 0; // push
        }
        -(bet as i16) // dealer vince
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

    /// Avanza al prossimo player che deve ancora giocare.
    /// Salta quelli già done (stood/bust).
    fn advance_turn(&mut self) {
        let player_done = |p: &Player| {
            let main_done = p.stood || calculate_score(&p.hand) > 21;
            let split_done = p.split_hand.is_none()
                || p.split_stood
                || calculate_score(p.split_hand.as_ref().unwrap()) > 21;
            main_done && split_done
        };

        let mut idx = self.player_idx + 1;
        while idx < self.players.len() && player_done(&self.players[idx]) {
            idx += 1;
        }
        self.player_idx = idx.min(self.players.len());
    }
}

/// Proiezione readonly dello State per la TUI.
/// Non espone deck, seed, hash — solo ciò che serve al rendering.
pub struct StateView {
    pub phase: GamePhase,
    pub local_player_id: PlayerId,
    pub players: Vec<PlayerView>,
    pub player_idx: usize,
    /// Carte visibili del dealer (la seconda è coperta durante Playing).
    pub dealer_hand: Vec<Card>,
    pub dealer_score: u8,
}
