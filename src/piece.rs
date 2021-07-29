
// rightmost 3 bits are an integer representing the type of a piece
#[derive(Copy, Clone)]
pub struct PieceType(pub u8);

pub const NO_TYPE: PieceType = PieceType(7);

//these piece types should match that of the indices in board::bb_indices
pub const PAWN: PieceType = PieceType(0);
pub const KNIGHT: PieceType = PieceType(1);
pub const BISHOP: PieceType = PieceType(2);
pub const ROOK: PieceType = PieceType(3);
pub const QUEEN: PieceType = PieceType(4);
pub const KING: PieceType = PieceType(5);

pub type Color = usize;
//should match indices in board::bb_indices
pub const WHITE: Color = 0;
pub const BLACK: Color = 1;