use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, NO_TYPE, PIECE_TYPES};
use crate::r#move::Move;
use crate::square::{Square, BAD_SQUARE};
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
    pub fn new() -> Board {
        Board {
            sides: [
                Bitboard(0x000000000000FFFF), //white
                Bitboard(0xFFFF000000000000), //black
            ],
            pieces: [
                Bitboard(0x00FF00000000FF00), //pawn
                Bitboard(0x4200000000000042), //knight
                Bitboard(0x2400000000000024), //bishop
                Bitboard(0x8100000000000081), //rook
                Bitboard(0x0800000000000008), //queen
                Bitboard(0x1000000000000010), //king
            ],
            en_passant_square: BAD_SQUARE,
            player_to_move: WHITE,
        }
    }

    #[inline]
    pub fn get_occupancy(&self) -> Bitboard {
        self.sides[WHITE] | self.sides[BLACK]
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

    pub fn type_at_square(&self, sq: Square) -> PieceType {
        let sq_bb = Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            if (self.pieces[i] & sq_bb) != BB_EMPTY {
                return PieceType(i as u8);
            }
        }
        return NO_TYPE;
    }

    pub fn color_at_square(&self, sq: Square) -> Color {
        match Bitboard::from(sq) & self.sides[WHITE] {
            BB_EMPTY => BLACK,
            _ => WHITE,
        }
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

    pub fn make_move(&mut self, m: Move) {
        //TODO handle castle rights
        //TODO handle en passant
        //TODO handle castling
        //TODO handle promotion
        //TODO handle turn timer
        let from_sq = m.from_square();
        let to_sq = m.to_square();
        let mover_type = self.type_at_square(from_sq);
        self.remove_piece(from_sq);
        self.add_piece(to_sq, mover_type, self.player_to_move);
    }

    //Remove the piece at sq from the board.
    fn remove_piece(&mut self, sq: Square) {
        let mask = !Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            self.pieces[i] &= mask;
        }
        self.sides[BLACK] &= mask;
        self.sides[WHITE] &= mask;
    }

    fn add_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        let mask = Bitboard::from(sq);
        self.pieces[pt.0 as usize] |= mask;
        self.sides[color] |= mask;
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square(i);
                let sq_bb = Bitboard::from(current_square);

                if (sq_bb & self.get_occupancy()) != BB_EMPTY {
                    //TODO capitalize if white pieces
                    let _is_white = (sq_bb & self.get_color_occupancy(WHITE)) != BB_EMPTY;
                    //find the type of this piece
                    for pt in PIECE_TYPES {
                        let pt_bb = self.get_pieces_of_type(pt);
                        if (pt_bb & sq_bb) != BB_EMPTY {
                            if let Err(e) = write!(f, "{}", pt) {
                                println!("Error {} while trying to write board!", e.to_string());
                            }
                            break;
                        }
                    }
                } else if let Err(e) = write!(f, " ") {
                    println!("Error {} while trying to write board!", e.to_string());
                }

                if c == 7 {
                    if let Err(e) = write!(f, "\n") {
                        println!("Error {} while trying to write board!", e.to_string());
                    }
                }
            }
        }
        write!(f, "")
    }
}
