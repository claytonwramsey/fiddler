//! A module containing the information for Piece-Square Tables (PSTs). A PST
//! is given for both the early and endgame.

use crate::base::Eval;
use crate::base::Piece;

/// A lookup table for piece values. The outer index is the type of the piece
/// (in order of Pawn, Knight, Bishop, Rook, Queen, and King)
/// and the inner index is the square of the piece (from White's point of view)
/// , starting with A1 as the first index, then continuing on to B1, C1, and so
/// on until H8 as index 63.
type Pst = [[Eval; 64]; Piece::NUM_TYPES];

/// A PST which is given in millipawns.
type MilliPst = [[i32; 64]; Piece::NUM_TYPES];

/// A function used for ergonomics to convert from a table of millipawn values
/// to a table of `Eval`s.
const fn expand_table(milli_table: MilliPst) -> Pst {
    let mut table = [[Eval::DRAW; 64]; Piece::NUM_TYPES];
    let mut piece_idx = 0;
    // I would use for-loops here, but those are unsupported in const fns.
    while piece_idx < Piece::NUM_TYPES {
        let mut sq_idx = 0;
        while sq_idx < 64 {
            table[piece_idx][sq_idx] = Eval::millipawns(milli_table[piece_idx][sq_idx]);
            sq_idx += 1;
        }
        piece_idx += 1;
    }
    table
}

/* For now, we use the values from Stockfish. */

/// A PST for the value of pawns in the middlegame.
pub const MIDGAME_VALUE: Pst = expand_table([
    [
        // pawns. ranks 1 and 8 are inconsequential
        0, 0, 0, 0, 0, 0, 0, 0, // rank 1
        20, 40, 110, 180, 160, 210, 90, -30, // rank 2
        -90, -150, 110, 150, 310, 230, 60, -200, // rank 3
        -30, -200, 80, 190, 390, 170, 20, -50, // rank 4
        110, -40, -110, 20, 110, 0, -120, 50, // rank 5
        30, -110, -60, 220, -80, -50, -140, -110, // rank 6
        -70, 60, -20, -110, 40, -140, 10, -90, // rank 7
        0, 0, 0, 0, 0, 0, 0, 0, // rank 8
    ],
    [
        // knights
        -1750, -920, -740, -730, -730, -740, -920, -1750, // rank 1
        -770, -410, -270, -150, -150, -270, -410, -770, // rank 2
        -610, -170, 60, 120, 120, 60, -170, -610, // rank 3
        -350, 80, 400, 490, 490, 400, 80, -350, // rank 4
        -340, 130, 440, 510, 510, 440, 130, -340, // rank 5
        -90, 330, 580, 530, 530, 580, 330, -90, // rank 6
        -670, -270, 40, 370, 370, 40, -270, -670, // rank 7
        -2010, -830, -560, -260, -260, -560, -830, -2010, // rank 8
    ],
    [
        // bishops
        -370, -40, -60, -160, -160, -60, -40, -370, // rank 1
        -110, 60, 130, 30, 30, 130, 60, -110, // rank 2
        -50, 150, -40, 120, 120, -40, 150, -50, // rank 3
        -40, 80, 180, 270, 270, 180, 80, -40, // rank 4
        -80, 200, 50, 220, 220, 50, 200, -80, // rank 5
        -110, 40, 10, 80, 80, 10, 40, -110, // rank 6
        -120, -100, 40, 0, 0, 40, -100, -120, // rank 7
        -340, -10, -10, -160, -160, -10, -10, -340, // rank 8
    ],
    [
        // rooks
        -310, -200, -140, -50, -50, -140, -200, -310, // rank 1
        -210, -130, -80, 60, 60, -80, -130, -210, // rank 2
        -250, -110, -10, 30, 30, -10, -110, -250, // rank 3
        -130, -50, -40, -60, -60, -40, -50, -130, // rank 4
        -270, -150, -40, 30, 30, -40, -150, -270, // rank 5
        -220, -20, 60, 120, 120, 60, -20, -220, // rank 6
        -20, 120, 160, 180, 180, 160, 120, -20, // rank 7
        -170, -190, -10, 90, 90, -10, -190, -170, // rank 8
    ],
    [
        // queens
        30, -50, -50, 40, 40, -50, -50, 30, // rank 1
        -30, 50, 80, 120, 120, 80, 50, -30, // rank 2
        -30, 60, 130, 70, 70, 130, 60, -30, // rank 3
        40, 50, 90, 80, 80, 90, 50, 40, // rank 4
        0, 140, 120, 50, 50, 120, 140, 0, // rank 5
        -40, 100, 60, 80, 80, 60, 100, -40, // rank 6
        -50, 60, 100, 80, 80, 100, 60, -50, // rank 7
        -20, -20, 10, -20, -20, 10, -20, -20, // rank 8
    ],
    [
        // kings
        2710, 3270, 2710, 1980, 1980, 2710, 3270, 2710, // rank 1
        2780, 3030, 2340, 1790, 1790, 2340, 3030, 2780, // rank 2
        1950, 2580, 1690, 1200, 1200, 1690, 2580, 1950, // rank 3
        1640, 1900, 1380, 980, 980, 1380, 1900, 1640, // rank 4
        1540, 1790, 1050, 700, 700, 1050, 1790, 1540, // rank 5
        1230, 1450, 810, 310, 310, 810, 1450, 1230, // rank 6
        880, 1200, 650, 330, 330, 650, 1200, 880, // rank 7
        590, 890, 450, -10, -10, 450, 890, 590, // rank 8
    ],
]);

/// The PST for pieces in the endgame.
pub const ENDGAME_VALUE: Pst = expand_table([
    /* TODO update everything except kings */
    [
        // pawns. ranks 1 and 8 are inconsequential
        0, 0, 0, 0, 0, 0, 0, 0, // rank 1
        20, 40, 110, 180, 160, 210, 90, -30, // rank 2
        -90, -150, 110, 150, 310, 230, 60, -200, // rank 3
        -30, -200, 80, 190, 390, 170, 20, -50, // rank 4
        110, -40, -110, 20, 110, 0, -120, 50, // rank 5
        30, -110, -60, 220, -80, -50, -140, -110, // rank 6
        -70, 60, -20, -110, 40, -140, 10, -90, // rank 7
        0, 0, 0, 0, 0, 0, 0, 0, // rank 8
    ],
    [
        // knights
        -1750, -920, -740, -730, -730, -740, -920, -1750, // rank 1
        -770, -410, -270, -150, -150, -270, -410, -770, // rank 2
        -610, -170, 60, 120, 120, 60, -170, -610, // rank 3
        -350, 80, 400, 490, 490, 400, 80, -350, // rank 4
        -340, 130, 440, 510, 510, 440, 130, -340, // rank 5
        -90, 330, 580, 530, 530, 580, 330, -90, // rank 6
        -670, -270, 40, 370, 370, 40, -270, -670, // rank 7
        -2010, -830, -560, -260, -260, -560, -830, -2010, // rank 8
    ],
    [
        // bishops
        -370, -40, -60, -160, -160, -60, -40, -370, // rank 1
        -110, 60, 130, 30, 30, 130, 60, -110, // rank 2
        -50, 150, -40, 120, 120, -40, 150, -50, // rank 3
        -40, 80, 180, 270, 270, 180, 80, -40, // rank 4
        -80, 200, 50, 220, 220, 50, 200, -80, // rank 5
        -110, 40, 10, 80, 80, 10, 40, -110, // rank 6
        -120, -100, 40, 0, 0, 40, -100, -120, // rank 7
        -340, -10, -10, -160, -160, -10, -10, -340, // rank 8
    ],
    [
        // rooks
        -310, -200, -140, -50, -50, -140, -200, -310, // rank 1
        -210, -130, -80, 60, 60, -80, -130, -210, // rank 2
        -250, -110, -10, 30, 30, -10, -110, -250, // rank 3
        -130, -50, -40, -60, -60, -40, -50, -130, // rank 4
        -270, -150, -40, 30, 30, -40, -150, -270, // rank 5
        -220, -20, 60, 120, 120, 60, -20, -220, // rank 6
        -20, 120, 160, 180, 180, 160, 120, -20, // rank 7
        -170, -190, -10, 90, 90, -10, -190, -170, // rank 8
    ],
    [
        // queens
        30, -50, -50, 40, 40, -50, -50, 30, // rank 1
        -30, 50, 80, 120, 120, 80, 50, -30, // rank 2
        -30, 60, 130, 70, 70, 130, 60, -30, // rank 3
        40, 50, 90, 80, 80, 90, 50, 40, // rank 4
        0, 140, 120, 50, 50, 120, 140, 0, // rank 5
        -40, 100, 60, 80, 80, 60, 100, -40, // rank 6
        -50, 60, 100, 80, 80, 100, 60, -50, // rank 7
        -20, -20, 10, -20, -20, 10, -20, -20, // rank 8
    ],
    [
        // kings
        10, 450, 850, 760, 760, 850, 450, 10, // rank 1
        530, 1000, 1330, 1350, 1350, 1330, 1000, 530, // rank 2
        880, 1300, 1690, 1750, 1750, 1690, 1300, 880, // rank 3
        1030, 1560, 1720, 1720, 1720, 1720, 1560, 1030, // rank 4
        960, 1660, 1990, 1990, 1990, 1990, 1660, 960, // rank 5
        920, 1720, 1840, 1910, 1910, 1840, 1720, 920, // rank 6
        470, 1210, 1160, 1310, 1310, 1160, 1210, 470, // rank 7
        110, 590, 730, 780, 780, 730, 590, 110, // rank 8
    ],
]);

#[cfg(test)]
mod tests {

    use super::*;
    use crate::base::Bitboard;
    use crate::base::Square;

    #[test]
    /// Test that the PST value of the pieces has left-right symmetry.
    fn test_left_right_symmetry() {
        for pt in Piece::NON_PAWN_TYPES {
            for sq1 in Bitboard::ALL {
                let sq2 = Square::new(sq1.rank(), 7 - sq1.file()).unwrap();
                assert_eq!(
                    MIDGAME_VALUE[pt as usize][sq1 as usize],
                    MIDGAME_VALUE[pt as usize][sq2 as usize]
                );
                assert_eq!(
                    ENDGAME_VALUE[pt as usize][sq1 as usize],
                    ENDGAME_VALUE[pt as usize][sq2 as usize]
                );
            }
        }
    }
}
