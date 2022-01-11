use crate::base::constants::Color;

use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
///
/// A simple struct to store a piece's castling rights.
/// The internal bits are used to represent castling rights.
/// From MSB to LSB:
/// * 4 unused bits
/// * Black queenside castling
/// * Black kingside castling
/// * White queenside castling
/// * White kingside castling
///
pub struct CastleRights(pub u8);

impl CastleRights {
    pub const ALL_RIGHTS: CastleRights = CastleRights(15);
    pub const NO_RIGHTS: CastleRights = CastleRights(0);
    pub const WHITE_RIGHTS: CastleRights = CastleRights(3);
    pub const BLACK_RIGHTS: CastleRights = CastleRights(12);

    ///
    /// Create a `CastleRights` for kingside castling on one side. `color` must
    /// be either `WHITE` or `BLACK`.
    ///
    #[inline]
    pub fn king_castle(color: Color) -> CastleRights {
        // White = 0 --> 1
        // Black = 1 --> 4
        CastleRights((1 + color * 3) as u8)
    }

    ///
    /// Create a `CastleRights` for queenside castling on one side. `color`
    /// must be either `WHITE` or `BLACK`.
    ///
    #[inline]
    pub fn queen_castle(color: Color) -> CastleRights {
        // White = 0 --> 2
        // Black = 1 --> 8
        CastleRights((2 + color * 6) as u8)
    }

    ///
    /// Get the full rights for one color. `color` must be either `WHITE` or
    /// `BLACK`.
    ///
    pub fn color_rights(color: Color) -> CastleRights {
        // White = 0 --> 3
        // Black = 1 --> 12
        CastleRights((3 + color * 9) as u8)
    }

    #[inline]
    ///
    /// Can the given color legally castle kingside?
    ///
    pub fn is_kingside_castle_legal(&self, color: Color) -> bool {
        *self & CastleRights::king_castle(color) != CastleRights::NO_RIGHTS
    }

    #[inline]
    ///
    /// Can the given color legally castle kingside?
    ///
    pub fn is_queenside_castle_legal(&self, color: Color) -> bool {
        *self & CastleRights::queen_castle(color) != CastleRights::NO_RIGHTS
    }
}

impl BitOr<CastleRights> for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn bitor(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 | other.0)
    }
}

impl BitOrAssign<CastleRights> for CastleRights {
    #[inline]
    fn bitor_assign(&mut self, other: CastleRights) {
        self.0 |= other.0;
    }
}

impl BitAnd<CastleRights> for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn bitand(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 & other.0)
    }
}

impl BitAndAssign<CastleRights> for CastleRights {
    #[inline]
    fn bitand_assign(&mut self, other: CastleRights) {
        self.0 &= other.0;
    }
}

impl Not for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn not(self) -> CastleRights {
        CastleRights(self.0 ^ 15)
    }
}
