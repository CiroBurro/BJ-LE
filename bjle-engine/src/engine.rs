use crate::state::{State, StateView};
use bjle_shared::{GameEvent, RoomId, UserAction, player::PlayerId};
use tokio::sync::mpsc::{self, Receiver, Sender};

pub struct Engine {
    state: State,
    local_seed: [u8; 16],
    is_creator: bool,
    current_room: Option<RoomId>,
    tx_eng_to_mesh: Sender<GameEvent>,
    rx_mesh_to_eng: Receiver<GameEvent>,
    tx_eng_to_tui: Sender<StateView>,
    rx_tui_to_eng: Receiver<UserAction>,
}

impl Engine {
    pub fn new(
        id: PlayerId,
        tx_eng_to_mesh: Sender<GameEvent>,
        rx_mesh_to_eng: Receiver<GameEvent>,
        tx_eng_to_tui: Sender<StateView>,
        rx_tui_to_eng: Receiver<UserAction>,
    ) -> Self {
        let (player, local_seed) = bjle_shared::player::Player::new(id);
        let mut state = State::new(id);
        // Inserisce subito il player locale nello stato.
        state.players.push(player);

        Self {
            state,
            local_seed,
            is_creator: false,
            current_room: None,
            tx_eng_to_mesh,
            rx_mesh_to_eng,
            tx_eng_to_tui,
            rx_tui_to_eng,
        }
    }

    pub async fn run(mut self) -> Result<(), mpsc::error::SendError<GameEvent>> {
        loop {
            tokio::select! {
                Some(event)  = self.rx_mesh_to_eng.recv() => self.handle_mesh_event(event).await?,
                Some(action) = self.rx_tui_to_eng.recv()  => self.handle_tui_action(action).await?,
            }
        }
    }

    // ── Mesh → Engine ────────────────────────────────────────────────────────

    async fn handle_mesh_event(
        &mut self,
        event: GameEvent,
    ) -> Result<(), mpsc::error::SendError<GameEvent>> {
        // Fase prima dell'apply — serve per rilevare transizioni.
        let phase_before = self.state.phase.clone();

        match self.state.apply(event) {
            Ok(Some(reply)) => self.tx_eng_to_mesh.send(reply).await?,
            Ok(None) => {}
            Err(e) => eprintln!("[engine] evento mesh ignorato: {e}"),
        }

        // Se la fase è appena passata a Revealing (StartGame ricevuto dalla mesh),
        // il nodo locale deve rivelare il proprio seed.
        use bjle_shared::GamePhase;
        if phase_before != GamePhase::Revealing && self.state.phase == GamePhase::Revealing {
            self.reveal_seed().await?;
        }

        // Aggiorna la TUI dopo ogni cambiamento di stato (ignora se il canale è pieno).
        let _ = self.tx_eng_to_tui.try_send(self.state.view());
        Ok(())
    }

    // ── TUI → Engine ─────────────────────────────────────────────────────────

    async fn handle_tui_action(
        &mut self,
        action: UserAction,
    ) -> Result<(), mpsc::error::SendError<GameEvent>> {
        let id = self.state.local_player_id;

        match action {
            // ── Lobby ─────────────────────────────────────────────────────────
            UserAction::CreateRoom { room_id } => {
                self.is_creator = true;
                self.current_room = Some(room_id);
                // Annuncia sé stesso sulla mesh.
                let hash = self
                    .state
                    .players
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.hash)
                    .unwrap_or([0u8; 16]);
                let event = GameEvent::Join {
                    player_id: id,
                    room_id,
                    hash,
                };
                // apply locale (aggiunge il player allo stato) + trasmetti.
                let _ = self.state.apply(event.clone());
                self.tx_eng_to_mesh.send(event).await?;
            }

            UserAction::JoinRoom { room_id } => {
                self.is_creator = false;
                self.current_room = Some(room_id);
                let hash = self
                    .state
                    .players
                    .iter()
                    .find(|p| p.id == id)
                    .map(|p| p.hash)
                    .unwrap_or([0u8; 16]);
                let join = GameEvent::Join {
                    player_id: id,
                    room_id,
                    hash,
                };
                let _ = self.state.apply(join.clone());
                self.tx_eng_to_mesh.send(join).await?;
                // Chiede la lista dei peer già in lobby.
                self.tx_eng_to_mesh
                    .send(GameEvent::RequestSync { player_id: id })
                    .await?;
            }

            // Solo il creatore può avviare.
            UserAction::StartGame => {
                if !self.is_creator {
                    eprintln!("[engine] solo il creatore può avviare la partita");
                    return Ok(());
                }
                let event = GameEvent::StartGame;
                let _ = self.state.apply(event.clone());
                self.tx_eng_to_mesh.send(event).await?;
                // Il creatore rivela subito il suo seed.
                self.reveal_seed().await?;
            }

            // ── Betting ───────────────────────────────────────────────────────
            UserAction::PlaceBet { amount } => {
                // La puntata è locale per ora: aggiorna fishes e basta.
                if let Some(p) = self.state.players.iter_mut().find(|p| p.id == id) {
                    if p.fishes < amount {
                        eprintln!("[engine] fishes insufficienti");
                        return Ok(());
                    }
                    p.fishes -= amount;
                    p.bet = amount;
                }
            }

            UserAction::ConfirmReady => {
                let event = GameEvent::TurnReady { player_id: id };
                let _ = self.state.apply(event.clone());
                self.tx_eng_to_mesh.send(event).await?;
            }

            // ── Playing ───────────────────────────────────────────────────────
            UserAction::Play(action) => {
                let event = GameEvent::PlayerAction {
                    player_id: id,
                    action,
                };
                match self.state.apply(event.clone()) {
                    Ok(_) => self.tx_eng_to_mesh.send(event).await?,
                    Err(e) => eprintln!("[engine] mossa non valida: {e}"),
                }
            }

            // ── Globale ───────────────────────────────────────────────────────
            UserAction::Quit => {
                let event = GameEvent::Leave { player_id: id };
                let _ = self.state.apply(event.clone());
                self.tx_eng_to_mesh.send(event).await?;
                // Il chiamante si occupa di terminare il processo.
            }
        }

        let _ = self.tx_eng_to_tui.try_send(self.state.view());
        Ok(())
    }

    // ── Helpers ──────────────────────────────────────────────────────────────

    /// Rivela il seed locale sulla mesh e applica l'evento allo stato locale.
    async fn reveal_seed(&mut self) -> Result<(), mpsc::error::SendError<GameEvent>> {
        let event = GameEvent::RevealSeed {
            player_id: self.state.local_player_id,
            seed: self.local_seed,
        };
        let _ = self.state.apply(event.clone());
        self.tx_eng_to_mesh.send(event).await
    }
}
