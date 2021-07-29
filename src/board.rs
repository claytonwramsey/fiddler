use crate::constants::NUM_PIECE_TYPES;
use crate::piece::{PieceType, Color};
use crate::bitboard::Bitboard;

mod bb_indices {
    pub const WHITE: usize = 0;
    pub const BLACK: usize = 1;
    pub const PAWN: usize = 0;
    pub const KNIGHT: usize = 1;
    pub const BISHOP: usize = 2;
    pub const ROOK: usize = 3;
    pub const QUEEN: usize = 4;
    pub const KING: usize = 5;
}

#[derive(Copy, Clone)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    sides: [Bitboard; 2],
    pieces: [Bitboard; NUM_PIECE_TYPES]
}

impl Board {
    pub fn getOccupancy(self) -> Bitboard {
        self.sides[bb_indices::WHITE] & self.sides[bb_indices::BLACK]
    }

    pub fn getColorOccupancy(self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    pub fn getPiecesOfType(self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    pub fn getPiecesOfTypeAndColor(self, pt: PieceType, color: Color) -> Bitboard {
        self.getPiecesOfType(pt) & self.getColorOccupancy(color)
    }
}

