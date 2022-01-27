use crate::base::square::Square;
use crate::base::PieceType;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
///
/// The information of one move, containing its from- and to-squares, as well as
/// its promote type, in one integer.
/// From MSB to LSB:
/// * 1 bit: unused
/// * 3 bits: promote type
/// * 6 bits: from-square
/// 6 bits: to-square
///
pub struct Move(pub u16);

impl Move {
    ///
    /// A sentinel value for a move which is illegal, or otherwise inexpressible.
    ///
    pub const BAD_MOVE: Move = Move(0xFFFF);

    ///
    /// Make a new `Move` for a piece. Assumes that all the inputs are valid.
    ///
    pub fn new(from_square: Square, to_square: Square, promote_type: PieceType) -> Move {
        let from_square_bits = from_square.0 as u16;
        let to_square_bits = (to_square.0 as u16) << 6;
        let promote_type_bits = (promote_type.0 as u16) << 12;

        Move(from_square_bits | to_square_bits | promote_type_bits)
    }

    #[inline]
    ///
    /// Get the target square of this move.
    ///
    pub fn to_square(self) -> Square {
        Square(((self.0 >> 6) & 63u16) as u8)
    }

    #[inline]
    ///
    /// Get the square that a piece moves from to execute this move.
    ///
    pub fn from_square(self) -> Square {
        Square((self.0 & 63u16) as u8)
    }

    #[inline]
    ///
    /// Get the promotion type of this move.
    ///
    pub fn promote_type(self) -> PieceType {
        PieceType(((self.0 >> 12) & 7u16) as u8)
    }

    ///
    /// Convert a move from its UCI representation.
    ///
    pub fn from_uci(s: &str) -> Result<Move, &'static str> {
        if !(s.len() == 4 || s.len() == 5) {
            return Err("string was neither a normal move or a promotion");
        }
        let from_sq = Square::from_algebraic(&s[0..2])?;
        let to_sq = Square::from_algebraic(&s[2..4])?;
        let promote_type = if s.len() == 5 {
            // this is valid because we already checked the length of s
            let charcode = s.chars().nth(4).unwrap();
            let pt = PieceType::from_code(charcode.to_ascii_uppercase());
            if pt == PieceType::NO_TYPE {
                return Err("invalid promote type given");
            }
            pt
        } else {
            PieceType::NO_TYPE
        };

        Ok(Move::new(from_sq, to_sq, promote_type))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.promote_type() == PieceType::NO_TYPE {
            return write!(f, "{} -> {}", self.from_square(), self.to_square());
        } else {
            return write!(
                f,
                "{} -> {} ={}",
                self.from_square(),
                self.to_square(),
                self.promote_type()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::square::*;

    #[test]
    fn test_uci_move_normal() {
        assert_eq!(
            Move::from_uci("e2e4").unwrap(),
            Move::new(E2, E4, PieceType::NO_TYPE)
        );
    }

    #[test]
    fn test_uci_move_promotion() {
        assert_eq!(
            Move::from_uci("b7b8q").unwrap(),
            Move::new(B7, B8, PieceType::QUEEN),
        );
    }
}
