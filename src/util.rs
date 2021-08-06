use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::constants::{Color, BLACK, WHITE, NO_COLOR};
use crate::direction::{Direction, NORTH, SOUTH, NODIR};

#[inline]
pub const fn opposite_color(color: Color) -> Color {
    match color {
        WHITE => BLACK,
        BLACK => WHITE,
        _ => NO_COLOR
    }
}

#[inline]
pub const fn pawn_direction(color: Color) -> Direction {
    match color {
        WHITE => NORTH,
        BLACK => SOUTH,
        _ => NODIR
    }
}

#[inline]
pub const fn pawn_promote_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0xFF00000000000000),
        BLACK => Bitboard(0x00000000000000FF),
        _ => BB_EMPTY
    }
}

#[inline]
pub const fn pawn_start_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0x000000000000FF00),
        BLACK => Bitboard(0x00FF000000000000),
        _ => BB_EMPTY
    }
}
