use crate::piece::{PieceType, NO_TYPE};
use crate::square::Square;
use std::fmt::{Display, Formatter, Result};
//Left to right:
//1 bit: unused
//3 bits: promote type
//6 bits: from square
//6 bits: to square
#[derive(Debug, Clone, Copy)]
pub struct Move(pub u16);

impl Move {
    // Make a Move for a piece which does have a promote type
    // assumes that the squares and promote types are all valid
    #[allow(dead_code)]
    pub fn new(from_square: Square, to_square: Square, promote_type: PieceType) -> Move {
        let from_square_bits = from_square.0 as u16;
        let to_square_bits = (to_square.0 as u16) << 6;
        let promote_type_bits = (promote_type.0 as u16) << 12;
        let my_value = from_square_bits | to_square_bits | promote_type_bits;
        return Move(my_value);
    }

    
    #[allow(dead_code)]
    pub fn to_square(self) -> Square {
        Square(((self.0 >> 6) & 63u16) as u8)
    }

    #[allow(dead_code)]
    pub fn from_square(self) -> Square {
        Square((self.0 & 63u16) as u8)
    }


    #[allow(dead_code)]
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
