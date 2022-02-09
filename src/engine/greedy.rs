use crate::base::Color;
use crate::base::Game;
use crate::base::MoveGenerator;
use crate::base::Piece;
use crate::engine::Eval;

///
/// Get the value of one piece by its type.
///
pub fn piece_value(pt: Piece) -> Eval {
    Eval::pawns(match pt {
        Piece::Pawn => 1.0,
        Piece::Knight => 2.9,
        Piece::Bishop => 3.1,
        Piece::Rook => 5.0,
        Piece::Queen => 9.0,
        _ => 0.0,
    })
}

///
/// Evaluate a position solely by the amount of material available.
///
pub fn greedy_evaluate(g: &mut Game, _mgen: &MoveGenerator) -> Eval {
    let mut eval = Eval(0);
    let b = g.get_board();

    for pt in Piece::ALL_TYPES {
        eval += piece_value(pt) * b.get_type_and_color(pt, Color::White).0.count_ones();
        eval -= piece_value(pt) * b.get_type_and_color(pt, Color::Black).0.count_ones();
    }

    eval
}
