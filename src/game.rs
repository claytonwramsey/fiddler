use crate::board::Board;
use crate::r#move::Move;

use std::collections::HashMap;
use std::default::Default;

/**
 * A struct containing game information, which unlike a `Board`, knows about 
 * its history and can do things like repetition timing.
 */
pub struct Game {
    /**
     * The last element in `history` is the current state of the board. The 
     * first element should be the starting position of the game, and in 
     * between are sequential board states from the entire game.
     */
    history: Vec<Board>,
    /**
     * `moves` is the list, in order, of all moves made in the game. They   
     * should all be valid moves. The length of `moves` should always be one 
     * less than the length of `history`.
     */
    moves: Vec<Move>,
    /**
     * Stores the number of times a position has been reached in the course of 
     * this game. It is used for three-move-rule draws.
     */
    repetitions: HashMap<Board, u8>,
    //TODO figure out how to implement fifty-move rule here.
}

impl Game {
}

impl Default for Game {
    fn default() -> Self {
        Game {
            history: vec![Board::default()],
            moves: Vec::new(),
            repetitions: HashMap::from([
                (Board::default(), 1),
            ]),
        }
    }
}