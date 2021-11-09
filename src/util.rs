use crate::constants::{Color, BLACK, NO_COLOR, WHITE};
use crate::Bitboard;
use crate::Direction;

#[inline]
#[allow(dead_code)]
/**
 * Get the opposite color of the given `Color`.
 */
pub const fn opposite_color(color: Color) -> Color {
    match color {
        WHITE => BLACK,
        BLACK => WHITE,
        _ => NO_COLOR,
    }
}

#[inline]
/**
 * Get the direction that a pawn of the given color normally moves.
 */
pub const fn pawn_direction(color: Color) -> Direction {
    match color {
        WHITE => Direction::NORTH,
        BLACK => Direction::SOUTH,
        _ => Direction::NODIR,
    }
}

#[inline]
/**
 * Get the promotion rank of a given color.
 */
pub const fn pawn_promote_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0xFF00000000000000),
        BLACK => Bitboard(0x00000000000000FF),
        _ => Bitboard::EMPTY,
    }
}

#[inline]
/**
 * Get a `Bitboard` with 1's on the start rank of the pawn of the given color.
 */
pub const fn pawn_start_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0x000000000000FF00),
        BLACK => Bitboard(0x00FF000000000000),
        _ => Bitboard::EMPTY,
    }
}
