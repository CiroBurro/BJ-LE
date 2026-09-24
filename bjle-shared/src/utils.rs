use crate::{Card, Suit};
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

    for v in 1..11 {
        deck.push(Card::new(Suit::Spades, u8::from(v)));
        deck.push(Card::new(Suit::Clubs, u8::from(v)));
        deck.push(Card::new(Suit::Diamonds, u8::from(v)));
        deck.push(Card::new(Suit::Hearts, u8::from(v)));
    }

    for _v in 1..4 {
        deck.push(Card::new(Suit::Spades, 10));
        deck.push(Card::new(Suit::Clubs, 10));
        deck.push(Card::new(Suit::Diamonds, 10));
        deck.push(Card::new(Suit::Hearts, 10));
    }

    deck.shuffle(&mut rng);
    deck
}
