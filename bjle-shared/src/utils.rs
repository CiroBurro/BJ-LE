<<<<<<< Updated upstream:bjle-shared/src/utils.rs
use crate::{Card, Suit};
=======
use bjle_shared::{Card, Suit};
>>>>>>> Stashed changes:bjle-engine/src/utils.rs
use rand::SeedableRng;
use rand::prelude::SliceRandom;
use rand_chacha::ChaCha8Rng;

pub fn calculate_score(hand: &Vec<Card>) -> u8 {
    let mut score = 0;
    let mut ace_counter = 0;
    for card in hand {
        if card.value == 1 {
            score += 11;
            ace_counter += 1;
<<<<<<< Updated upstream:bjle-shared/src/utils.rs
=======
        } else if card.value > 10 {
            score += 10;
>>>>>>> Stashed changes:bjle-engine/src/utils.rs
        } else {
            score += card.value;
        }
    }

    while score > 21 && ace_counter > 0 {
        score -= 10;
        ace_counter -= 1;
    }

    score
}

pub fn generate_deck(seed: [u8; 32]) -> Vec<Card> {
    let mut rng = ChaCha8Rng::from_seed(seed);

    let mut deck: Vec<Card> = Vec::with_capacity(52);

    for v in 1..14 {
        deck.push(Card::new(Suit::Spades, u8::from(v)));
        deck.push(Card::new(Suit::Clubs, u8::from(v)));
        deck.push(Card::new(Suit::Diamonds, u8::from(v)));
        deck.push(Card::new(Suit::Hearts, u8::from(v)));
    }

    deck.shuffle(&mut rng);
    deck
}
