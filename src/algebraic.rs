use crate::constants;
use crate::Board;
use crate::Move;
use crate::MoveGenerator;
use crate::PieceType;

#[allow(dead_code)]
/**
 * Given a `Move` and the `Board` it was played on, construct the
 * algebraic-notation version of the move. Assumes the move was legal.
 */
pub fn algebraic_from_move(m: Move, b: &Board, mgen: &MoveGenerator) -> String {
    //longest possible algebraic string would be something along the lines of
    //Qe4xd4# (7 chars)
    //e8=Q#
    //O-O-O+
    let mut s = String::with_capacity(7);

    if b.is_move_castle(m) {
        if m.to_square().file() > m.from_square().file() {
            //moving right, must be O-O
            s += "O-O";
        } else {
            s += "O-O-O";
        }
    } else {
        let mover_type = b.type_at_square(m.from_square());
        let is_move_capture =
            b.get_occupancy().is_square_occupied(m.to_square()) || b.is_move_en_passant(m);
        let other_moves = mgen.get_moves(b);
        let from_sq = m.from_square();

        // Type of the piece moving
        if mover_type != PieceType::PAWN || is_move_capture {
            s += mover_type.get_code();
        }

        // Resolution of un-clarity on mover location
        let mut is_unclear = false;
        let mut is_unclear_rank = false;
        let mut is_unclear_file = false;
        for other_move in other_moves {
            if other_move != m
                && other_move.to_square() == m.to_square()
                && b.type_at_square(other_move.from_square()) == mover_type
            {
                is_unclear = true;
                if other_move.from_square().rank() == from_sq.rank() {
                    is_unclear_rank = true;
                }
                if other_move.from_square().file() == from_sq.file() {
                    is_unclear_file = true;
                }
            }
        }

        if is_unclear && !is_unclear_file {
            //we can specify the mover by its file
            s += constants::FILE_NAMES[from_sq.file()];
        } else if !is_unclear_rank {
            //we can specify the mover by its rank
            s += constants::RANK_NAMES[from_sq.rank()];
        } else {
            //we need the complete square to specify the location of the mover
            s += &from_sq.to_string();
        }

        if is_move_capture {
            s += "x";
        }

        s += &m.to_square().to_string();

        //TODO checks + mates here
    }

    return s;
}
