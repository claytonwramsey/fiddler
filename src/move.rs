use crate::square::Square;
use crate::piece::PieceType;
//Left to right:
//2 bits: unused
//2 bits: promote type
//6 bits: from square
//6 bits: to square
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
        Square((self.0 & 63u16) as u8)
    }

    #[allow(dead_code)]
    pub fn from_square(self) -> Square {
        Square(((self.0 >> 6) & 63u16) as u8)
    }

    #[allow(dead_code)]
    pub fn promote_type(self) -> PieceType {
        PieceType(((self.0 >> 12) & 3u16) as u8)
    }
}