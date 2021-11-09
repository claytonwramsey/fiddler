use crate::constants::{BLACK, WHITE};
use crate::engine::Eval;
use crate::piece::*;
use crate::util::opposite_color;
use crate::Game;
use crate::MoveGenerator;
use crate::Square;

/**
 * Get the value of one piece by its type.
 */
pub fn piece_value(pt: PieceType) -> Eval {
    Eval::pawns(match pt {
        PAWN => 1.0,
        KNIGHT => 3.0,
        BISHOP => 3.0,
        ROOK => 5.0,
        QUEEN => 9.0,
        _ => 0.0,
    })
}

/**
 * Evaluate a position solely by the amount of material available.
 */
pub fn greedy_evaluate(g: &mut Game, mgen: &MoveGenerator) -> Eval {
    let mut eval = Eval(0);
    let b = g.get_board();
    let player = b.player_to_move;
    let king_sq = Square::from(b.get_pieces_of_type_and_color(KING, player));

    if g.is_game_over(mgen) {
        if mgen.is_square_attacked_by(b, king_sq, opposite_color(player)) {
            return match b.player_to_move {
                WHITE => Eval::BLACK_MATE,
                BLACK => Eval::WHITE_MATE,
                _ => Eval(0),
            };
        }
        return Eval(0);
    }

    for i in 0..NUM_PIECE_TYPES {
        let pt = PieceType(i as u8);
        eval += piece_value(pt) * b.get_pieces_of_type_and_color(pt, WHITE).0.count_ones();
        eval -= piece_value(pt) * b.get_pieces_of_type_and_color(pt, BLACK).0.count_ones();
    }
    return eval;
}
