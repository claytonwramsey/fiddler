use crate::piece::{PieceType, NO_TYPE};
use crate::square::Square;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
/**
 * The information of one move, containing its from- and to-squares, as well as
 * its promote type, in one integer.
 *
 * From MSB to LSB:
 * 1 bit: unused
 * 3 bits: promote type
 * 6 bits: from-square
 * 6 bits: to-square
 */
pub struct Move(pub u16);

impl Move {
    #[allow(dead_code)]
    /**
     * Make a new `Move` for a piece. Assumes that all the inputs are valid.
     */
    pub fn new(from_square: Square, to_square: Square, promote_type: PieceType) -> Move {
        let from_square_bits = from_square.0 as u16;
        let to_square_bits = (to_square.0 as u16) << 6;
        let promote_type_bits = (promote_type.0 as u16) << 12;
        let my_value = from_square_bits | to_square_bits | promote_type_bits;
        return Move(my_value);
    }

    #[allow(dead_code)]
    #[inline]
    /**
     * Get the target square of this move.
     */
    pub fn to_square(self) -> Square {
        Square(((self.0 >> 6) & 63u16) as u8)
    }

    #[allow(dead_code)]
    #[inline]
    /**
     * Get the square that a piece moves from to execute this move.
     */
    pub fn from_square(self) -> Square {
        Square((self.0 & 63u16) as u8)
    }

    #[allow(dead_code)]
    #[inline]
    /**
     * Get the promotion type of this move.
     */
    pub fn promote_type(self) -> PieceType {
        PieceType(((self.0 >> 12) & 7u16) as u8)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if self.promote_type() == NO_TYPE {
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

#[allow(dead_code)]
pub const BAD_MOVE: Move = Move(0xFFFF);
