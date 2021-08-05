// rightmost 3 bits are an integer representing the type of a piece
#[derive(Copy, Clone)]
pub struct PieceType(pub u8);

#[allow(dead_code)]
pub const NO_TYPE: PieceType = PieceType(7);

//these piece types should match that of the indices in board::bb_indices
#[allow(dead_code)]
pub const PAWN: PieceType = PieceType(0);
#[allow(dead_code)]
pub const KNIGHT: PieceType = PieceType(1);
#[allow(dead_code)]
pub const BISHOP: PieceType = PieceType(2);
#[allow(dead_code)]
pub const ROOK: PieceType = PieceType(3);
#[allow(dead_code)]
pub const QUEEN: PieceType = PieceType(4);
#[allow(dead_code)]
pub const KING: PieceType = PieceType(5);

pub const PIECE_TYPES: [PieceType; 6] = [PAWN, KNIGHT, BISHOP, ROOK, QUEEN, KING];
