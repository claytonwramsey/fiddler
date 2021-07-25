use crate::constants::NUM_PIECE_TYPES;
use crate::bitboard::Bitboard;

mod BbIndices {
    pub const WHITE: usize = 0;
    pub const BLACK: usize = 1;
    pub const PAWN: usize = 2;
    pub const KNIGHT: usize = 3;
    pub const BISHOP: usize = 4;
    pub const ROOK: usize = 5;
    pub const QUEEN: usize = 6;
    pub const KING: usize = 7;
}

pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    bitboards: [Bitboard; NUM_PIECE_TYPES + 2]
}

impl Board {
    pub fn getOccupancy(self) -> Bitboard {
        return self.bitboards[BbIndices::WHITE] & self.bitboards[BbIndices::BLACK]
    }
}

