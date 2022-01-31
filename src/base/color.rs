use std::ops::Not;

#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Color {
    White = 0,
    Black = 1,
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
    ///
    /// Test that the opposite color of `Color::White` is `Color::Black`, and
    /// vice versa.
    ///
    fn test_opposite_color() {
        assert_eq!(Color::White, !Color::Black);
        assert_eq!(Color::Black, !Color::White);
    }
}
