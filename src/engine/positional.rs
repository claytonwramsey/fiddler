use crate::base::constants::{BLACK, WHITE};
use crate::base::piece::PieceType;
use crate::base::Bitboard;
use crate::base::Game;
use crate::base::MoveGenerator;
use crate::base::Square;
use crate::engine::greedy::greedy_evaluate;
use crate::engine::Eval;

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
    0.02, 0.02, 0.0, 0.0, 0.0, -0.1, 0.02, 0.02, //rank 3
    0.04, 0.04, 0.05, 0.15, 0.15, 0.0, 0.04, 0.04, //rank 4
    0.1, 0.1, 0.08, 0.15, 0.15, 0.08, 0.1, 0.1, //rank 5
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
    0.0, 0.0, 0.18, 0.18, 0.18, 0.18, 0.0, 0.0, //rank 6
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 7
    0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, //rank 8
];

const ROOK_VALUES: ValueTable = [0.0; 64];

const BISHOP_VALUES: ValueTable = [
    0.1, 0.05, 0.0, 0.0, 0.0, 0.0, 0.05, 0.1, //rank 1
    0.05, 0.05, 0.05, 0.0, 0.0, 0.05, 0.1, 0.05, //rank 2
    0.0, 0.05, 0.1, 0.05, 0.05, 0.1, 0.05, 0.0, //rank 3
    0.0, 0.0, 0.05, 0.1, 0.1, 0.05, 0.0, 0.0, //rank 4
    0.0, 0.0, 0.05, 0.1, 0.1, 0.05, 0.0, 0.0, //rank 5
    0.0, 0.05, 0.1, 0.05, 0.05, 0.1, 0.05, 0.0, //rank 6
    0.05, 0.1, 0.05, 0.0, 0.0, 0.05, 0.1, 0.05, //rank 7
    0.1, 0.05, 0.0, 0.0, 0.0, 0.0, 0.05, 0.1, //rank 8
];

const DEFAULT_VALUES: ValueTable = [0.0; 64];

///
/// The value of having an opponent's pawn doubled.
///
const DOUBLED_PAWN_VALUE: Eval = Eval(100);

///
/// Evaluate a position by both its material and the positional value of the/// position.
///
pub fn positional_evaluate(g: &mut Game, mgen: &MoveGenerator) -> Eval {
    let starting_eval = greedy_evaluate(g, mgen);
    if starting_eval.is_mate() {
        return starting_eval;
    }

    let mut positional_eval = Eval(0);

    let b = g.get_board();
    for pt in [PieceType::PAWN, PieceType::BISHOP, PieceType::KNIGHT, PieceType::KING] {
        for sq in b.get_type_and_color(pt, WHITE) {
            positional_eval += value_at_square(pt, sq);
        }
        for sq in b.get_type_and_color(pt, BLACK) {
            //Invert the square that Black is on, since positional values are
            //flipped (as pawns move the other way, etc)
            let alt_sq = Square::new(7 - sq.rank(), sq.file());
            positional_eval -= value_at_square(pt, alt_sq);
        }
    }

    // Add losses due to doubled pawns
    let white_occupancy = b.get_color_occupancy(WHITE);
    let pawns = b.get_type(PieceType::PAWN);
    let mut col_mask = Bitboard(0x0101010101010101);
    for _ in 0..8 {
        let col_pawns = pawns & col_mask;

        // all ones on the A column, shifted left by the col
        let num_black_doubled_pawns = match ((!white_occupancy) & col_pawns).0.count_ones() {
            0 => 0,
            x => x - 1,
        };
        let num_white_doubled_pawns = match (white_occupancy & col_pawns).0.count_ones() {
            0 => 0,
            x => x - 1,
        };

        positional_eval += DOUBLED_PAWN_VALUE * num_black_doubled_pawns;
        positional_eval -= DOUBLED_PAWN_VALUE * num_white_doubled_pawns;

        col_mask <<= 1;
    }

    return starting_eval + positional_eval;
}

#[inline]
///
/// Get the positional value of a piece at a square.
/// Requires that the square be a valid square.
/// 
pub fn value_at_square(pt: PieceType, sq: Square) -> Eval {
    let val_table = match pt {
        PieceType::PAWN => &PAWN_VALUES,
        PieceType::KNIGHT => &KNIGHT_VALUES,
        PieceType::BISHOP => &BISHOP_VALUES,
        PieceType::ROOK => &ROOK_VALUES,
        PieceType::KING => &KING_VALUES,
        PieceType::QUEEN => &QUEEN_VALUES,
        _ => &DEFAULT_VALUES,
    };

    Eval::pawns(unsafe {
        *val_table.get_unchecked(sq.0 as usize)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::square::*;
    use crate::base::{Game, Move};

    #[test]
    ///
    /// Test that at the start of the game, the positional value of all the
    /// pieces is equal.
    ///
    fn test_equal_start() {
        let mut g = Game::default();
        let mgen = MoveGenerator::new();
        assert_eq!(positional_evaluate(&mut g, &mgen), Eval(0));
    }

    #[test]
    ///
    /// Test that if White plays F3, the positional value of the position is
    /// better for Black.
    ///
    fn test_f3_bad() {
        let mut g = Game::default();
        let mgen = MoveGenerator::new();
        g.make_move(Move::new(F2, F3, PieceType::NO_TYPE));
        assert!(positional_evaluate(&mut g, &mgen) < Eval(0));
    }
}
