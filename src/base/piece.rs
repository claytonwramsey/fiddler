/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Pieces, which contain no information about their color or current square.

use std::fmt::{Display, Formatter, Result};

#[derive(Copy, Clone, PartialEq, Eq, Debug, Hash)]
#[repr(u8)]
/// The type of a piece.
/// This contains no information about the location of a piece, or of its color.
///
/// The ordering of elements of this enumeration is highly intentional.
/// The first four pieces (knight, bishop, rook, and queen) are generally well-behaved and subject
/// to the same rules, and are all valid promotion types.
/// However, pawns and kings have no such obligations.
/// Having the  well-behaved types as the lower integers allows them to be more efficiently packed
/// as promotion types and generally reduces hassle.
pub enum Piece {
    /// A knight, which can move in an L-shape (two squares out, then one square sideways).
    Knight = 0,
    /// A bishop, which can move arbitrarily far diagonally.
    Bishop,
    /// A rook, which can move arbitrarily far horizontally or vertically.
    Rook,
    /// A queen, which can move like both a rook and a bishop.
    Queen,
    /// A pawn, which is an especially cheap piece with limited movement.
    Pawn,
    /// A king, which is the most valuable piece.
    King,
}

impl Piece {
    /// Total number of piece types.
    pub const NUM: usize = 6;

    /// Array containing all piece types.
    pub const ALL: [Piece; Piece::NUM] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::Pawn,
        Piece::King,
    ];

    /// Array containing piece types which are not pawns.
    pub const NON_PAWNS: [Piece; Piece::NUM - 1] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::King,
    ];

    /// Array containing piece types which are not kings.
    pub const NON_KING: [Piece; Piece::NUM - 1] = [
        Piece::Knight,
        Piece::Bishop,
        Piece::Rook,
        Piece::Queen,
        Piece::Pawn,
    ];

    /// The types of pieces that a pawn can be promted to.
    pub const PROMOTING: [Piece; 4] = [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen];

    #[must_use]
    /// Get the FEN code of this piece as an uppercase string.
    pub const fn code(self) -> char {
        match self {
            Piece::Knight => 'N',
            Piece::Bishop => 'B',
            Piece::Rook => 'R',
            Piece::Queen => 'Q',
            Piece::Pawn => 'P',
            Piece::King => 'K',
        }
    }

    #[must_use]
    /// Given a FEN character, convert it to a piece type.
    /// Must be uppercase.
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
