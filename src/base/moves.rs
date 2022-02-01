use crate::base::square::Square;
use crate::base::Piece;
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
pub struct Move(u16);

impl Move {
    ///
    /// A sentinel value for a move which is illegal, or otherwise
    /// inexpressible.
    ///
    pub const BAD_MOVE: Move = Move(0xFFFF);

    ///
    /// The bits representing a non-promoting piece in the promote type field.
    ///
    const NO_PROMOTE: u16 = Piece::NUM_TYPES as u16;

    ///
    /// Make a new `Move` for a piece. Assumes that all the inputs are valid.
    ///
    pub fn new(from_square: Square, to_square: Square, promote_type: Option<Piece>) -> Move {
        let from_square_bits = from_square.0 as u16;
        let to_square_bits = (to_square.0 as u16) << 6;
        let promote_type_bits = match promote_type {
            Some(p) => p as u16,
            None => Move::NO_PROMOTE,
        } << 12;

        Move(from_square_bits | to_square_bits | promote_type_bits)
    }

    #[inline]
    ///
    /// Create a `Move` with no promotion type.
    ///
    pub fn normal(from_square: Square, to_square: Square) -> Move {
        Move::new(from_square, to_square, None)
    }

    #[inline]
    ///
    /// Create a `Move` with the given promotion type.
    ///
    pub fn promoting(from_square: Square, to_square: Square, promote_type: Piece) -> Move {
        Move::new(from_square, to_square, Some(promote_type))
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
    pub fn promote_type(self) -> Option<Piece> {
        let promote_bits = (self.0 >> 12) & 7u16;
        // Justification for the transmutation here:
        // We know that from the creation of a Move its promotion type must
        // always have been valid.
        match promote_bits {
            Move::NO_PROMOTE => None,
            x => Some(unsafe { std::mem::transmute(x as u8) }),
        }
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
            let pt = Piece::from_code(charcode.to_ascii_uppercase());
            if pt == None {
                return Err("invalid promote type given");
            }
            pt
        } else {
            None
        };

        Ok(Move::new(from_sq, to_sq, promote_type))
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.promote_type() {
            None => write!(f, "{} -> {}", self.from_square(), self.to_square()),
            Some(p) => write!(f, "{} -> {} ={}", self.from_square(), self.to_square(), p),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::square::*;

    #[test]
    fn test_uci_move_normal() {
        assert_eq!(Move::from_uci("e2e4").unwrap(), Move::new(E2, E4, None));
    }

    #[test]
    fn test_uci_move_promotion() {
        assert_eq!(
            Move::from_uci("b7b8q").unwrap(),
            Move::new(B7, B8, Some(Piece::Queen)),
        );
    }
}
