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

//! Castling rights management.

use super::Color;

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXorAssign, Not};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// A simple struct to store a piece's castling rights.
///
/// Internally, a castling rights is represented as a packed bitmask.
/// The bits in this mask have the following assignments:
/// * `1 << 0` (LSB): White kingside castling.
/// * `1 << 1`: White queenside castling.
/// * `1 << 2`: Black kingside castling.
/// * `1 << 3`: Black queenside castling.
pub struct CastleRights(pub u8);

impl CastleRights {
    /// A `CastleRights` where all rights are available.
    pub const ALL: Self = Self(15);

    /// A `CastleRights` where no rights are available.
    pub const NONE: Self = Self(0);

    /// A `CastleRights` where White has both castling rights.
    pub const WHITE: Self = Self(3);

    /// A `CastleRights` where White has both castling rights.
    pub const BLACK: Self = Self(12);

    /// A `CastleRights` where the only right is White's kingside castle.
    pub const WHITE_KINGSIDE: Self = Self(1 << 0);

    /// A `CastleRights` where the only right is White's queenside castle.
    pub const WHITE_QUEENSIDE: Self = Self(1 << 1);

    /// A `CastleRights` where the only right is Black's kingside castle.
    pub const BLACK_KINGSIDE: Self = Self(1 << 2);

    /// A `CastleRights` where the only right is Black's queenside castle.
    pub const BLACK_QUEENSIDE: Self = Self(1 << 3);

    /// Can the given color legally castle kingside?
    pub fn kingside(self, color: Color) -> bool {
        self & match color {
            Color::White => Self::WHITE_KINGSIDE,
            Color::Black => Self::BLACK_KINGSIDE,
        } != Self::NONE
    }

    /// Can the given color legally castle kingside?
    pub fn queenside(self, color: Color) -> bool {
        self & match color {
            Color::White => Self::WHITE_QUEENSIDE,
            Color::Black => Self::BLACK_QUEENSIDE,
        } != Self::NONE
    }
}

impl BitOr<Self> for CastleRights {
    type Output = Self;
    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

impl BitOrAssign<Self> for CastleRights {
    fn bitor_assign(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl BitAnd<Self> for CastleRights {
    type Output = Self;
    fn bitand(self, other: Self) -> Self {
        Self(self.0 & other.0)
    }
}

impl BitAndAssign<Self> for CastleRights {
    fn bitand_assign(&mut self, other: Self) {
        self.0 &= other.0;
    }
}

impl Not for CastleRights {
    type Output = Self;
    fn not(self) -> Self {
        Self(self.0 ^ 15)
    }
}

impl BitXorAssign for CastleRights {
    fn bitxor_assign(&mut self, rhs: Self) {
        self.0 ^= rhs.0;
    }
}
