use crate::square::Square;
use crate::piece::PieceType;
//Left to right:
//2 bits: unused
//2 bits: promote type
//6 bits: from square
//6 bits: to square
pub struct Move(u16);

impl Move {
    // Make a Move for a piece which does not need a promote type
    fn new(from_square: Square, to_square: Square) -> Move {
        Self::new(from_square, to_square, NO_TYPE);
    }

    // Make a Move for a piece which does have a promote type
    fn new(from_square: Square, to_square: Square, promote_type: PieceType) -> Move {
        let from_square_bits = from_square.0 as u16;
        let to_square_bits = (to_square.0 as u16) << 4;
        let promote_type_bits = promote_type.0
    }
}