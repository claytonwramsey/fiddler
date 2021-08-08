use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, PIECE_TYPES};
use crate::square::Square;
use std::fmt::{Display, Formatter, Result};

#[derive(Copy, Clone, Debug)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    pub sides: [Bitboard; 2],
    pub pieces: [Bitboard; NUM_PIECE_TYPES],
    pub player_to_move: Color,
    pub en_passant_square: Square,
}

impl Board {
    #[inline]
    pub fn get_occupancy(&self) -> Bitboard {
        self.sides[WHITE] & self.sides[BLACK]
    }

    #[inline]
    pub fn get_color_occupancy(&self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    #[inline]
    pub fn get_pieces_of_type(&self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    #[inline]
    pub fn get_pieces_of_type_and_color(&self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }

    //Check through the state of this board and return false if this is an
    //invalid board state
    pub fn is_valid(&self) -> bool {
        let mut sides_checksum = BB_EMPTY;
        let mut sides_checkor = BB_EMPTY;
        let mut pieces_checksum = BB_EMPTY;
        let mut pieces_checkor = BB_EMPTY;
        for bb in self.sides {
            sides_checksum += bb;
            sides_checkor |= bb;
        }
        for bb in self.pieces {
            pieces_checksum += bb;
            pieces_checkor |= bb;
        }
        if sides_checksum != sides_checkor {
            return false;
        }
        if pieces_checksum != pieces_checkor {
            return false;
        }
        if sides_checksum != pieces_checksum {
            return false;
        }
        return true;
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result{
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square(i);
                let sq_bb = Bitboard::from(current_square);
                
                if (sq_bb & self.get_occupancy()) != BB_EMPTY {
                    let is_white = (sq_bb & self.get_color_occupancy(WHITE)) != BB_EMPTY;
                    //find the type of this piece
                    for pt in PIECE_TYPES {
                        let pt_bb = self.get_pieces_of_type(pt);
                        if (pt_bb & sq_bb) != BB_EMPTY {
                            write!(f, "{}", pt);
                            break;
                        }
                    }
                }
                else {
                    write!(f, " ");
                }

                if c == 7 {
                    write!(f, "\n");
                }
            }
        }
        write!(f, "")
    }
}