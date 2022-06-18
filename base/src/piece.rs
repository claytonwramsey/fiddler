//! Pieces, which contain no information about their color or current square.

use std::fmt::{Display, Formatter, Result};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
/// The type of a piece. This contains no information about the location of a
/// piece, or of its color.
///
/// The ordering of elements of this enumeration is highly intentional. The
/// first four pieces (knight, bishop, rook, and queen) are generally
/// well-behaved and subject to the same rules, and are all valid promotion
/// types. However, pawns and kings have no such obligations. Having the
/// well-behaved types as the lower integers allows them to be more efficiently
/// packed as promotion types and generally reduces hassle.
pub enum Piece {
    Knight = 0,
    Bishop,
    Rook,
    Queen,
    Pawn,
    King,
}

impl Piece {
    /// Total number of piece types.
    pub const NUM_TYPES: usize = 6;

    /// Array containing all piece types.
    pub const ALL_TYPES: [Piece; Piece::NUM_TYPES] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::Pawn,
        Piece::King,
    ];

    /// Array containing piece types which are not pawns.
    pub const NON_PAWN_TYPES: [Piece; Piece::NUM_TYPES - 1] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    /// Array containing piece types which are not kings.
    pub const NON_KING_TYPES: [Piece; Piece::NUM_TYPES - 1] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::Pawn,
    ];

    /// Get the FEN code of this piece as an uppercase string.
    pub const fn code(self) -> &'static str {
        match self {
            Piece::Knight => "N",
            Piece::Bishop => "B",
            Piece::Rook => "R",
            Piece::Queen => "Q",
            Piece::Pawn => "P",
            Piece::King => "K",
        }
    }

    /// Given a FEN character, convert it to a piece type. Must be uppercase.
    pub const fn from_code(c: char) -> Option<Piece> {
        match c {
            'N' => Some(Piece::Knight),
            'B' => Some(Piece::Bishop),
            'R' => Some(Piece::Rook),
            'Q' => Some(Piece::Queen),
            'P' => Some(Piece::Pawn),
            'K' => Some(Piece::King),
            _ => None,
        }
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.code())
    }
}
