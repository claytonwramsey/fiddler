/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! Castling rights management.

use super::Color;

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// A simple struct to store a piece's castling rights.
/// The internal bits are used to represent castling rights.
/// From MSB to LSB:
/// * 4 unused bits
/// * Black queenside castling
/// * Black kingside castling
/// * White queenside castling
/// * White kingside castling
pub struct CastleRights(pub u8);

impl CastleRights {
    /// A `CastleRights` where all rights are available.
    pub const ALL_RIGHTS: CastleRights = CastleRights(15);

    /// A `CastleRights` where no rights are available.
    pub const NO_RIGHTS: CastleRights = CastleRights(0);

    /// Create a `CastleRights` for kingside castling on one side.
    #[inline(always)]
    pub const fn king_castle(color: Color) -> CastleRights {
        match color {
            Color::White => CastleRights(1),
            Color::Black => CastleRights(4),
        }
    }

    /// Create a `CastleRights` for queenside castling on one side.
    #[inline(always)]
    pub const fn queen_castle(color: Color) -> CastleRights {
        match color {
            Color::White => CastleRights(2),
            Color::Black => CastleRights(8),
        }
    }

    /// Get the full rights for one color.
    pub const fn color_rights(color: Color) -> CastleRights {
        match color {
            Color::White => CastleRights(3),
            Color::Black => CastleRights(12),
        }
    }

    #[inline(always)]
    /// Can the given color legally castle kingside?
    pub fn is_kingside_castle_legal(&self, color: Color) -> bool {
        *self & CastleRights::king_castle(color) != CastleRights::NO_RIGHTS
    }

    #[inline(always)]
    /// Can the given color legally castle kingside?
    pub fn is_queenside_castle_legal(&self, color: Color) -> bool {
        *self & CastleRights::queen_castle(color) != CastleRights::NO_RIGHTS
    }
}

impl BitOr<CastleRights> for CastleRights {
    type Output = CastleRights;
    #[inline(always)]
    fn bitor(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 | other.0)
    }
}

impl BitOrAssign<CastleRights> for CastleRights {
    #[inline(always)]
    fn bitor_assign(&mut self, other: CastleRights) {
        self.0 |= other.0;
    }
}

impl BitAnd<CastleRights> for CastleRights {
    type Output = CastleRights;
    #[inline(always)]
    fn bitand(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 & other.0)
    }
}

impl BitAndAssign<CastleRights> for CastleRights {
    #[inline(always)]
    fn bitand_assign(&mut self, other: CastleRights) {
        self.0 &= other.0;
    }
}

impl Not for CastleRights {
    type Output = CastleRights;
    #[inline(always)]
    fn not(self) -> CastleRights {
        CastleRights(self.0 ^ 15)
    }
}
