
// rightmost 3 bits are an integer representing the type of a piece
#[derive(Copy, Clone)]
pub struct PieceType(u8);

pub const NO_TYPE: PieceType = PieceType(0);
pub const PAWN: PieceType = PieceType(1);
pub const KNIGHT: PieceType = PieceType(2);
pub const BISHOP: PieceType = PieceType(3);
pub const ROOK: PieceType = PieceType(4);
pub const QUEEN: PieceType = PieceType(5);
pub const KING: PieceType = PieceType(6);

