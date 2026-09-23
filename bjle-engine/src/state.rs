use bjle_shared::{Card, PlayerId};
use std::collections::HashMap;

pub struct State {
    pub deck: Vec<Card>,
    pub dealer_hand: Vec<Card>,
    pub player_hands: HashMap<PlayerId, Vec<Card>>,
    pub deck_index: usize,
}
