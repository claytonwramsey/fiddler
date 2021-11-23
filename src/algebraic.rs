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

        if is_unclear {
            if !is_unclear_file {
                //we can specify the mover by its file
                s += constants::FILE_NAMES[from_sq.file()];
            } else if !is_unclear_rank {
                //we can specify the mover by its rank
                s += constants::RANK_NAMES[from_sq.rank()];
            } else {
                //we need the complete square to specify the location of the mover
                s += &from_sq.to_string();
            }
        }

        if is_move_capture {
            s += "x";
        }

        s += &m.to_square().to_string();

        //TODO checks + mates here
    }

    return s;
}

/**
 * Given the string of an algebraic-notation move, get the `Move` which can be 
 * played. Will return Err if the string is invalid.
 */
pub fn move_from_algebraic(s: &str, b: &Board, mgen: &MoveGenerator) -> Result<Move, &'static str> {
    let moves = mgen.get_moves(b);
    for m in moves {
        let algebraic_str = algebraic_from_move(m, b, mgen);
        if algebraic_str.as_str() == s {
            return Ok(m);
        }
    }
    return Err("not a legal algebraic move");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::square::*;
    use crate::fens::*;
    #[test]
    fn test_e4_to_algebraic() {
        let b = Board::default();
        let mgen = MoveGenerator::new();
        let m = Move::new(E2, E4, PieceType::NO_TYPE);

        assert_eq!(String::from("e4"), algebraic_from_move(m, &b, &mgen));
    }

    
    #[test]
    fn test_mate() {
        let b = Board::from_fen(MATE_IN_1_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let m = Move::new(B6, B8, PieceType::NO_TYPE);

        println!("{}", b);
        assert_eq!(String::from("Rb8#"), algebraic_from_move(m, &b, &mgen));
        
    }

    #[test]
    /**
     * Test that the opening move e4 can be converted from a string to a move.
     */
    fn test_move_from_e4() {
        
        let b = Board::default();
        let mgen = MoveGenerator::new();
        let m = Move::new(E2, E4, PieceType::NO_TYPE);
        let s = "e4";

        assert_eq!(move_from_algebraic(s, &b, &mgen), Ok(m));
    }


    #[test]
    /**
     * Test that you get an error out when you give it a bad string.
     */
    fn test_bad_algebraic() {
        
        let b = Board::default();
        let mgen = MoveGenerator::new();
        let m = Move::new(E2, E4, PieceType::NO_TYPE);
        let s = "garbage";

        assert!(move_from_algebraic(s, &b, &mgen).is_err());
    }
}