use crate::constants::NUM_PIECE_TYPES;
use crate::piece::{PieceType};
use crate::constants::{Color, WHITE, BLACK};
use crate::bitboard::Bitboard;

#[derive(Copy, Clone)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    sides: [Bitboard; 2],
    pieces: [Bitboard; NUM_PIECE_TYPES]
}

impl Board {
    pub fn get_occupancy(self) -> Bitboard {
        self.sides[WHITE] & self.sides[BLACK]
    }

    pub fn get_color_occupancy(self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    pub fn get_pieces_of_type(self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    pub fn get_pieces_of_type_ang_color(self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }
}

