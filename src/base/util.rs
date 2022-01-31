use crate::base::Bitboard;
use crate::base::Color;
use crate::base::Direction;

#[inline]
///
/// Get the direction that a pawn of the given color normally moves. Assumes
/// the given color is either `Color::White` or `Color::Black`.
///
pub const fn pawn_direction(color: Color) -> Direction {
    match color {
        Color::White => Direction::NORTH,
        Color::Black => Direction::SOUTH,
    }
}

#[inline]
///
/// Get the promotion rank of a given color. Assumes the given color is either
/// `Color::White` or `Color::Black`.
///
pub const fn pawn_promote_rank(color: Color) -> Bitboard {
    // White: 0xFF00000000000000
    // Black: 0x00000000000000FF
    match color {
        Color::White => Bitboard(0xFF00000000000000),
        Color::Black => Bitboard(0x00000000000000FF),
    }
}

#[inline]
///
/// Get a `Bitboard` with 1's on the start rank of the pawn of the given color.
/// Assumes the given color is either `Color::White` or `Color::Black`.
///
pub const fn pawn_start_rank(color: Color) -> Bitboard {
    // White: 0x000000000000FF00
    // Black: 0x00FF000000000000
    Bitboard((0xFF << 8) + ((0xFF << 48) - (0xFF << 8)) * (color as u64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    ///
    /// Test that the direction for White pawns is north and the direction for
    /// Black pawns is south.
    ///
    fn test_directions() {
        assert_eq!(pawn_direction(Color::White), Direction::NORTH);
        assert_eq!(pawn_direction(Color::Black), Direction::SOUTH);
    }

    #[test]
    ///
    /// Test that the pawn promotion rank bitboards are correct.
    ///
    fn test_pawn_promote_rank() {
        assert_eq!(
            Bitboard(0xFF00000000000000),
            pawn_promote_rank(Color::White)
        );
        assert_eq!(
            Bitboard(0x00000000000000FF),
            pawn_promote_rank(Color::Black)
        );
    }

    #[test]
    ///
    /// Test that the start ranks for pawns are correct.
    ///
    fn test_pawn_start_rank() {
        assert_eq!(pawn_start_rank(Color::White), Bitboard(0x000000000000FF00));
        assert_eq!(pawn_start_rank(Color::Black), Bitboard(0x00FF000000000000));
    }
}
