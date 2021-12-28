use crate::base::constants::Color;
use crate::base::Bitboard;
use crate::base::Direction;

#[inline]
///
/// Get the opposite color of the given `Color`. Assumes the given color is
/// either `WHITE` or `BLACK`.
///
pub const fn opposite_color(color: Color) -> Color {
    // black = 0 -> 1 - black = 1 = white
    // white = 1 -> 1 - white = 0 = black
    1 - color
}

#[inline]
///
/// Get the direction that a pawn of the given color normally moves. Assumes
/// the given color is either `WHITE` or `BLACK`.
///
pub const fn pawn_direction(color: Color) -> Direction {
    // black = 0, south = -8 -> 16 * 0 - 8 = -8 = south
    // white = 1, north = 8  -> 16 * 1 - 8 =  8 = north
    Direction(8 - 16 * (color as i8))
}

#[inline]
///
/// Get the promotion rank of a given color. Assumes the given color is either
/// `WHITE` or `BLACK`.
///
pub const fn pawn_promote_rank(color: Color) -> Bitboard {
    // White: 0xFF00000000000000
    // Black: 0x00000000000000FF
    Bitboard((0xFF << 56) - ((0xFF << 56) - 0xFF) * (color as u64))
}

#[inline]
///
/// Get a `Bitboard` with 1's on the start rank of the pawn of the given color.
///
pub const fn pawn_start_rank(color: Color) -> Bitboard {
    // White: 0x000000000000FF00
    // Black: 0x00FF000000000000
    Bitboard((0xFF << 8) + ((0xFF << 48) - (0xFF << 8)) * (color as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::constants::{BLACK, WHITE};

    #[test]
    ///
    /// Test that the opposite color of `WHITE` is `BLACK`, and vice versa.
    ///
    fn test_opposite_color() {
        assert_eq!(WHITE, opposite_color(BLACK));
        assert_eq!(BLACK, opposite_color(WHITE));
    }

    #[test]
    ///
    /// Test that the direction for White pawns is north and the direction for
    /// Black pawns is south.
    ///
    fn test_directions() {
        assert_eq!(pawn_direction(WHITE), Direction::NORTH);
        assert_eq!(pawn_direction(BLACK), Direction::SOUTH);
    }

    #[test]
    ///
    /// Test that the pawn promotion rank bitboards are correct.
    ///
    fn test_pawn_promote_rank() {
        assert_eq!(Bitboard(0xFF00000000000000), pawn_promote_rank(WHITE));
        assert_eq!(Bitboard(0x00000000000000FF), pawn_promote_rank(BLACK));
    }

    #[test]
    ///
    /// Test that the start ranks for pawns are correct.
    ///
    fn test_pawn_start_rank() {
        assert_eq!(pawn_start_rank(WHITE), Bitboard(0x000000000000FF00));
        assert_eq!(pawn_start_rank(BLACK), Bitboard(0x00FF000000000000));
    }
}
