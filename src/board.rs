use crate::bitboard::Bitboard;
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, PIECE_TYPES};
use crate::r#move::Move;
use crate::square::Square;

#[derive(Copy, Clone)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    pub sides: [Bitboard; 2],
    pub pieces: [Bitboard; NUM_PIECE_TYPES],
    pub player_to_move: Color,
    pub en_passant_square: Square,
}

impl Board {
    #[inline]
    pub fn get_occupancy(self) -> Bitboard {
        self.sides[WHITE] & self.sides[BLACK]
    }

    #[inline]
    pub fn get_color_occupancy(self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    #[inline]
    pub fn get_pieces_of_type(self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    #[inline]
    pub fn get_pieces_of_type_and_color(self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }
}
