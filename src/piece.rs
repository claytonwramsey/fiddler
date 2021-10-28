use std::fmt::{Display, Formatter, Result};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/**
 * A struct representing the type of a piece. Only the rightmost 3 bits are used.
 */
pub struct PieceType(pub u8);

impl PieceType {
    /**
     * Get the FEN code of this piece as an uppercase string.
     */
    pub fn get_code(self) -> &'static str {
        match self {
            NO_TYPE => "_",
            PAWN => "P",
            KNIGHT => "N",
            BISHOP => "B",
            ROOK => "R",
            QUEEN => "Q",
            KING => "K",
            _ => "?",
        }
    }

    /**
     * Given a FEN character, convert it to a piece type. Must be uppercase.
     */
    pub fn from_code(c: char) -> PieceType {
        match c {
            'P' => PAWN,
            'N' => KNIGHT,
            'B' => BISHOP,
            'R' => ROOK,
            'Q' => QUEEN,
            'K' => KING,
            _ => NO_TYPE,
        }
    }
}

impl Display for PieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.get_code())
    }
}

#[allow(dead_code)]
pub const NO_TYPE: PieceType = PieceType(7);

//these piece types should match that of the indices in board::bb_indices
#[allow(dead_code)]
pub const PAWN: PieceType = PieceType(0);
#[allow(dead_code)]
pub const KNIGHT: PieceType = PieceType(1);
#[allow(dead_code)]
pub const BISHOP: PieceType = PieceType(2);
#[allow(dead_code)]
pub const ROOK: PieceType = PieceType(3);
#[allow(dead_code)]
pub const QUEEN: PieceType = PieceType(4);
#[allow(dead_code)]
pub const KING: PieceType = PieceType(5);

pub const PIECE_TYPES: [PieceType; 6] = [PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING];

pub const PROMOTE_TYPES: [PieceType; 4] = [KNIGHT, BISHOP, ROOK, QUEEN];
