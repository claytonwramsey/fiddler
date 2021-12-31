use crate::base::constants::{BLACK, WHITE};
use crate::base::util::opposite_color;
use crate::base::Game;
use crate::base::MoveGenerator;
use crate::base::PieceType;
use crate::base::Square;
use crate::base::Move;
use crate::engine::Eval;

///
/// Get the value of one piece by its type.
///
pub fn piece_value(pt: PieceType) -> Eval {
    Eval::pawns(match pt {
        PieceType::PAWN => 1.0,
        PieceType::KNIGHT => 3.0,
        PieceType::BISHOP => 3.0,
        PieceType::ROOK => 5.0,
        PieceType::QUEEN => 9.0,
        _ => 0.0,
    })
}

///
/// Evaluate a position solely by the amount of material available.
///
pub fn greedy_evaluate(g: &mut Game, moves: &[Move], mgen: &MoveGenerator) -> Eval {
    let mut eval = Eval(0);
    let b = g.get_board();
    let player = b.player_to_move;
    let king_sq = Square::from(b.get_type_and_color(PieceType::KING, player));

    if moves.len() == 0 {
        if mgen.is_square_attacked_by(b, king_sq, opposite_color(player)) {
            return match b.player_to_move {
                WHITE => Eval::BLACK_MATE,
                BLACK => Eval::WHITE_MATE,
                _ => Eval(0),
            };
        }
        return Eval(0);
    }

    for pt in PieceType::ALL_TYPES {
        eval += piece_value(pt) * b.get_type_and_color(pt, WHITE).0.count_ones();
        eval -= piece_value(pt) * b.get_type_and_color(pt, BLACK).0.count_ones();
    }
    return eval;
}
