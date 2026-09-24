use crate::state::{State, StateView};
use bjle_shared::{GameEvent, UserAction, player::PlayerId};
use tokio::sync::mpsc::{Receiver, Sender};

pub struct Engine {
    state: State,
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
        let state = State::new(id);

        Self {
            state,
            tx_eng_to_mesh,
            rx_mesh_to_eng,
            tx_eng_to_tui,
            rx_tui_to_eng,
        }
    }
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.rx_mesh_to_eng.recv() => todo!(),
                action = self.rx_tui_to_eng.recv() => todo!(),
            }
        }
    }
}
