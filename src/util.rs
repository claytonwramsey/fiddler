use crate::constants::{Color, BLACK, WHITE};

#[inline]
pub const fn opposite_color(color: Color) -> Color {
    match color {
        WHITE => BLACK,
        BLACK => WHITE,
    }
}
