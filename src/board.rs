use crate::bitboard::Bitboard;
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, PIECE_TYPES};
use crate::r#move::Move;
use crate::square::Square;

#[derive(Copy, Clone)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    sides: [Bitboard; 2],
    pieces: [Bitboard; NUM_PIECE_TYPES],
    player_to_move: Color,
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

    pub fn get_pieces_of_type_and_color(self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }

    //Enumerate pseudo-legal moves in the current position
    pub fn get_pseudolegal_moves(self) -> Vec<Move> {
        let mut moves = Vec::new();
        let side_to_move = self.sides[self.player_to_move];
        for pt in PIECE_TYPES {
            let mut pieces_to_move = side_to_move & self.pieces[pt.0 as usize];
            while pieces_to_move != Bitboard(0) {
                //square of next piece to move
                let sq = Square(pieces_to_move.0.trailing_zeros() as u8);
                //remove that square
                pieces_to_move &= !Bitboard(1 << sq.0);
                moves.extend(self.sq_pseudolegal_moves(sq, pt));
            }
        }
        return moves;
    }

    //Enumerate all the pseudolegal moves made by a certain type at a certain
    //square in this position.
    fn sq_pseudolegal_moves(self, sq: Square, pt: PieceType) -> Vec<Move> {
        match pt {
            PAWN => self.pawn_moves(sq),
            KNIGHT => self.knight_moves(sq),
            KING => self.king_moves(sq),
            BISHOP => self.bishop_moves(sq),
            ROOK => self.rook_moves(sq),
            QUEEN => self.queen_moves(sq),
            //bad type gets empty vector of moves
            _ => Vec::new(),
        }
    }

    fn rook_moves(self, sq) -> Vec<Move> {
        
    }

    //Enumerating pseudolegal moves for each piece type
    fn queen_moves(self, sq) -> Vec<Move> {
        self.rook_moves(sq) | self.bishop_moves(sq)
    }
}

fn bitboard_to_moves(sq: Square, bb: Bitboard) -> Bitboard {

}
