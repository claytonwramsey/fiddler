//! Evaluation of positions based on the mobility of pieces.

use fiddler_base::{
    movegen::{KING_MOVES, KNIGHT_MOVES, PAWN_ATTACKS},
    Board, Color, Piece, MAGIC,
};

use super::{Eval, Score};

/// The maximum number of squares that any piece can attack.
pub const MAX_MOBILITY: usize = 27;

/// The value of having a piece have a certain number of squares attacked.
pub const ATTACKS_VALUE: [[Score; MAX_MOBILITY]; Piece::NUM] = expand_attacks(&[
    [
        // N
        (-1, -4),
        (2, -4),
        (7, -2),
        (12, -1),
        (14, -1),
        (16, -6),
        (15, -7),
        (21, -10),
        (17, -11),
        (6, -6),
        (6, -6),
        (3, -2),
        (3, -2),
        (2, -2),
        (0, -1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // B
        (-8, -3),
        (-1, -4),
        (6, -4),
        (6, -7),
        (12, -4),
        (15, -8),
        (15, -7),
        (19, -5),
        (19, -10),
        (17, -9),
        (19, -12),
        (15, -7),
        (14, -10),
        (10, -8),
        (10, -6),
        (6, -5),
        (4, -4),
        (3, -4),
        (2, -2),
        (0, -2),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // R
        (-4, -7),
        (1, 0),
        (4, -1),
        (7, 0),
        (11, -3),
        (13, -5),
        (15, -6),
        (14, -7),
        (15, -9),
        (14, -8),
        (11, -5),
        (13, -6),
        (11, -7),
        (8, -7),
        (6, -5),
        (2, -2),
        (2, -3),
        (2, -2),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // Q
        (-6, 7),
        (3, 1),
        (2, 2),
        (6, 5),
        (1, 2),
        (0, 0),
        (4, 5),
        (2, 0),
        (2, 1),
        (0, 0),
        (0, 0),
        (0, 0),
        (-2, -2),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (1, 0),
        (0, 2),
        (0, 2),
        (0, 2),
        (0, 3),
        (0, 2),
        (0, 1),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // P
        (0, 0),
        (0, -2),
        (0, -2),
        (0, -1),
        (1, -4),
        (0, -4),
        (0, -4),
        (2, -5),
        (2, -1),
        (-2, 0),
        (1, 0),
        (1, 4),
        (3, 1),
        (2, 1),
        (3, 1),
        (2, 0),
        (3, 0),
        (1, 0),
        (4, 0),
        (7, 0),
        (3, 0),
        (2, 0),
        (0, 0),
        (2, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
    [
        // K
        (8, -1),
        (0, 6),
        (-1, 7),
        (7, 7),
        (6, 6),
        (4, 3),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ],
]);

/// Helper function to make the process of writing down attack values more easy.
const fn expand_attacks(vals: &[[(i16, i16); 27]; Piece::NUM]) -> [[Score; 27]; Piece::NUM] {
    let mut out = [[Score::DRAW; 27]; 6];

    let mut pt_idx = 0;
    while pt_idx < Piece::NUM {
        let mut mobility_idx = 0;
        while mobility_idx < MAX_MOBILITY {
            let (mg, eg) = vals[pt_idx][mobility_idx];
            out[pt_idx][mobility_idx] = Score::new(Eval(mg), Eval(eg));
            mobility_idx += 1;
        }
        pt_idx += 1;
    }

    out
}

#[must_use]
/// Get the mobility evaluation of a board.
pub fn evaluate(b: &Board) -> Score {
    let counts = count_attacks(b);
    let mut total = Score::DRAW;
    for pt in Piece::ALL {
        let count = counts[pt as usize];
        #[allow(clippy::cast_sign_loss)]
        if count < 0 {
            total -= ATTACKS_VALUE[pt as usize][(-count) as usize];
        } else {
            total += ATTACKS_VALUE[pt as usize][count as usize];
        }
    }
    total
}

#[must_use]
/// Count the net mobility of all pieces on the board. Each index corresponds to
/// one piece type.
pub fn count_attacks(b: &Board) -> [i16; Piece::NUM] {
    let white = b[Color::White];
    let black = b[Color::Black];
    let not_white = !white;
    let not_black = !black;
    let occupancy = white | black;

    // count knight moves
    let mut net_knight = 0;
    let knights = b[Piece::Knight];
    for sq in knights & white {
        net_knight += i16::from((KNIGHT_MOVES[sq as usize] & not_white).len());
    }
    for sq in knights & black {
        net_knight -= i16::from((KNIGHT_MOVES[sq as usize] & not_black).len());
    }

    // count bishop moves
    let mut net_bishop = 0;
    let bishops = b[Piece::Bishop];
    for sq in bishops & white {
        net_bishop += i16::from((MAGIC.bishop_attacks(occupancy, sq) & not_white).len());
    }
    for sq in bishops & black {
        net_bishop -= i16::from((MAGIC.bishop_attacks(occupancy, sq) & not_black).len());
    }

    // count rook moves
    let mut net_rook = 0;
    let rooks = b[Piece::Rook];
    for sq in rooks & white {
        net_rook += i16::from((MAGIC.rook_attacks(occupancy, sq) & not_white).len());
    }
    for sq in rooks & black {
        net_rook -= i16::from((MAGIC.rook_attacks(occupancy, sq) & not_black).len());
    }

    // count queen moves
    let mut net_queen = 0;
    let queens = b[Piece::Queen];
    for sq in queens & white {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        net_queen += i16::from((attacks & not_white).len());
    }
    for sq in rooks & black {
        let attacks = MAGIC.rook_attacks(occupancy, sq) | MAGIC.bishop_attacks(occupancy, sq);
        net_queen -= i16::from((attacks & not_black).len());
    }

    // count net pawn moves
    // pawns can't capture by pushing, so we only examine their capture squares
    let mut net_pawn = 0;
    let pawns = b[Piece::Pawn];
    for sq in pawns & white {
        net_pawn += i16::from((PAWN_ATTACKS[Color::White as usize][sq as usize] & not_white).len());
    }
    for sq in pawns & black {
        net_pawn += i16::from((PAWN_ATTACKS[Color::Black as usize][sq as usize] & not_black).len());
    }

    let white_king =
        i16::from((KING_MOVES[b.king_sqs[Color::White as usize] as usize] & not_white).len());
    let black_king =
        i16::from((KING_MOVES[b.king_sqs[Color::Black as usize] as usize] & not_black).len());
    let net_king = white_king - black_king;

    [
        net_knight, net_bishop, net_rook, net_queen, net_pawn, net_king,
    ]
}
