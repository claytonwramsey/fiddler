use crate::base::Bitboard;
use crate::base::Direction;
use std::ops::Not;

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Color {
    #[inline]
    /// Get the direction that a pawn of the given color normally moves.
    pub const fn pawn_direction(&self) -> Direction {
        match self {
            Color::White => Direction::NORTH,
            Color::Black => Direction::SOUTH,
        }
    }

    #[inline]
    /// Get the promotion rank of a given color.
    pub const fn pawn_promote_rank(&self) -> Bitboard {
        match self {
            Color::White => Bitboard(0xFF00000000000000),
            Color::Black => Bitboard(0x00000000000000FF),
        }
    }

    #[inline]
    /// Get a `Bitboard` with 1's on the start rank of the pawn of the given
    /// color.
    pub const fn pawn_start_rank(&self) -> Bitboard {
        match self {
            Color::White => Bitboard(0x000000000000FF00),
            Color::Black => Bitboard(0x00FF000000000000),
        }
    }
}

impl Not for Color {
    type Output = Self;
    #[inline]
    fn not(self) -> Color {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that the opposite color of `Color::White` is `Color::Black`, and
    /// vice versa.
    fn test_opposite_color() {
        assert_eq!(Color::White, !Color::Black);
        assert_eq!(Color::Black, !Color::White);
    }

    #[test]
    /// Test that the direction for White pawns is north and the direction for
    /// Black pawns is south.
    fn test_directions() {
        assert_eq!(Color::White.pawn_direction(), Direction::NORTH);
        assert_eq!(Color::Black.pawn_direction(), Direction::SOUTH);
    }

    #[test]
    /// Test that the pawn promotion rank bitboards are correct.
    fn test_pawn_promote_rank() {
        assert_eq!(
            Bitboard(0xFF00000000000000),
            Color::White.pawn_promote_rank()
        );
        assert_eq!(
            Bitboard(0x00000000000000FF),
            Color::Black.pawn_promote_rank()
        );
    }

    #[test]
    /// Test that the start ranks for pawns are correct.
    fn test_pawn_start_rank() {
        assert_eq!(Color::White.pawn_start_rank(), Bitboard(0x000000000000FF00));
        assert_eq!(Color::Black.pawn_start_rank(), Bitboard(0x00FF000000000000));
    }
}
