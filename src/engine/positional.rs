use crate::engine::greedy::greedy_evaluate;
use crate::engine::Eval;
use crate::Game;
use crate::MoveGenerator;
use crate::piece::{PieceType, NUM_PIECE_TYPES, PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING};
use crate::constants::{WHITE, BLACK};
use crate::Square;

type ValueTable = [f64; 64];

const KING_VALUES: ValueTable = [
    0.1, 0.1, 0.2, 0.0, 0.0, 0.0, 0.2, 0.1, //rank 1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 2
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 3
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 4
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 5
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 6
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 8
];

const QUEEN_VALUES: ValueTable = [0.0; 64];

const PAWN_VALUES: ValueTable = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 2
    0.0, 0.0, 0.0, 0.0, 0.0, -0.1, 0.0, 0.0, //rank 3
    0.0, 0.0, 0.05, 0.12, 0.12, 0.0, 0.0, 0.0, //rank 4
    0.0, 0.0, 0.08, 0.1, 0.1, 0.08, 0.0, 0.0, //rank 5
    0.2, 0.5, 0.5, 0.5, 0.5, 0.5, 0.5, 0.2, //rank 6
    0.5, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 0.5, //rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 8
];

const KNIGHT_VALUES: ValueTable = [
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 1
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 2
    0.0, 0.0, 0.15, 0.15, 0.15, 0.15, 0.0, 0.0, //rank 3
    0.0, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.0, //rank 4
    0.0, 0.15, 0.15, 0.15, 0.15, 0.15, 0.15, 0.0, //rank 5
    0.0, 0.0, 0.15, 0.15, 0.15, 0.15, 0.0, 0.0, //rank 6
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 8
];

const ROOK_VALUES: ValueTable = [0.0; 64];

const BISHOP_VALUES: ValueTable = [
    0.3, 0.1, 0.0, 0.0, 0.0, 0.0, 0.1, 0.3,
    0.1, 0.3, 0.1, 0.0, 0.0, 0.1, 0.3, 0.1,
    0.0, 0.1, 0.3, 0.1, 0.1, 0.3, 0.1, 0.0,
    0.0, 0.0, 0.1, 0.3, 0.3, 0.1, 0.0, 0.0,
    0.0, 0.0, 0.1, 0.3, 0.3, 0.1, 0.0, 0.0,
    0.0, 0.1, 0.3, 0.1, 0.1, 0.3, 0.1, 0.0,
    0.1, 0.3, 0.1, 0.0, 0.0, 0.1, 0.3, 0.1,
    0.3, 0.1, 0.0, 0.0, 0.0, 0.0, 0.1, 0.3,
];

const DEFAULT_VALUES: ValueTable = [0.0; 64];

#[allow(dead_code)]
/**
 * Evaluate a position by both its material and the positional value of the
 * position.
 */
pub fn positional_evaluate(g: &mut Game, mgen: &MoveGenerator) -> Eval {
    let starting_eval = greedy_evaluate(g, mgen);
    if starting_eval.is_mate() {
        return starting_eval;
    }

    let mut positional_eval = Eval(0);

    for i in 0..NUM_PIECE_TYPES {
        let pt = PieceType(i as u8);

        let val_table = match pt {
            PAWN => &PAWN_VALUES,
            KNIGHT => &KNIGHT_VALUES,
            BISHOP => &BISHOP_VALUES,
            ROOK => &ROOK_VALUES,
            KING => &KING_VALUES,
            QUEEN => &QUEEN_VALUES,
            _ => &DEFAULT_VALUES,
        };

        for sq in g.get_board().get_pieces_of_type_and_color(pt, WHITE) {
            positional_eval += Eval::pawns(val_table[sq.0 as usize]);
        }
        for sq in g.get_board().get_pieces_of_type_and_color(pt, BLACK) {
            //Invert the square that Black is on, since positional values are
            //flipped (as pawns move the other way, etc)
            let alt_sq = Square::new(9 - sq.rank(), sq.file());
            positional_eval -= Eval::pawns(val_table[alt_sq.0 as usize]);
        }
    }

    return starting_eval + positional_eval;
}
