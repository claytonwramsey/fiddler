use std::fmt::{Display, Formatter, Result};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
///
/// The type of a piece. Only the rightmost 3 bits of its internal
/// representation are used.
///
pub enum Piece {
    Pawn = 0,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl Piece {
    ///
    /// Total number of piece types.
    ///
    pub const NUM_TYPES: usize = 6;

    pub const ALL_TYPES: [Piece; Piece::NUM_TYPES] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    pub const NON_PAWN_TYPES: [Piece; Piece::NUM_TYPES - 1] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    pub const NON_KING_TYPES: [Piece; Piece::NUM_TYPES - 1] = [
        Piece::Pawn,
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
    ];

    pub const NUM_PROMOTE_TYPES: usize = 4;

    pub const PROMOTE_TYPES: [Piece; 4] = [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

    ///
    /// Get the FEN code of this piece as an uppercase string.
    ///
    pub const fn get_code(self) -> &'static str {
        match self {
            Piece::Pawn => "P",
            Piece::Knight => "N",
            Piece::Bishop => "B",
            Piece::Rook => "R",
            Piece::Queen => "Q",
            Piece::King => "K",
        }
    }

    ///
    /// Given a FEN character, convert it to a piece type. Must be uppercase.
    ///
    pub const fn from_code(c: char) -> Option<Piece> {
        match c {
            'P' => Some(Piece::Pawn),
            'N' => Some(Piece::Knight),
            'B' => Some(Piece::Bishop),
            'R' => Some(Piece::Rook),
            'Q' => Some(Piece::Queen),
            'K' => Some(Piece::King),
            _ => None,
        }
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.get_code())
    }
}
