use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::board::Board;
use crate::constants::Color;
use crate::direction::{
    Direction, EAST, NEE, NNE, NNW, NORTH, NORTHEAST, NORTHWEST, NWW, SEE, SOUTH, SOUTHEAST,
    SOUTHWEST, SSE, SSW, SWW, WEST,
};
use crate::magic::{get_bishop_attacks, get_rook_attacks, MagicTable};
use crate::piece::{
    PieceType, BISHOP, KING, KNIGHT, NO_TYPE, PAWN, PIECE_TYPES, PROMOTE_TYPES, QUEEN, ROOK,
};
use crate::r#move::Move;
use crate::square::Square;
use crate::util::{opposite_color, pawn_direction, pawn_promote_rank, pawn_start_rank};

//All the saved data necessary to generate moves
#[allow(dead_code)]
pub struct MoveGenerator {
    mtable: MagicTable,
    pawn_attacks: [Bitboard; 64], //for now unused, will be used later
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

impl MoveGenerator {
    #[allow(dead_code)]
    pub fn new() -> MoveGenerator {
        MoveGenerator {
            mtable: MagicTable::load(),
            pawn_attacks: create_step_attacks(&vec![NORTHEAST, NORTHWEST], 1),
            king_moves: create_step_attacks(&get_king_steps(), 1),
            knight_moves: create_step_attacks(&get_knight_steps(), 2),
        }
    }

    #[allow(dead_code)]
    pub fn get_moves(&self, board: &Board) -> Vec<Move> {
        let moves = self.get_pseudolegal_moves(board, board.player_to_move);
        let mut legal_moves = Vec::<Move>::new();
        for m in moves {
            if !self.is_move_self_check(board, m) {
                legal_moves.push(m);
            }
        }
        return legal_moves;
    }

    //This enumerates pseudolegal moves if the player of a given color were playing.
    pub fn get_pseudolegal_moves(&self, board: &Board, color: Color) -> Vec<Move> {
        let mut moves = Vec::new();
        //iterate through all the pieces of this color and enumerate their moves
        for pt in PIECE_TYPES {
            let mut pieces_to_move = board.get_pieces_of_type_and_color(pt, color);
            while pieces_to_move != BB_EMPTY {
                //square of next piece to move
                let sq = Square::from(pieces_to_move);
                //remove that square
                pieces_to_move &= !Bitboard::from(sq);
                moves.extend(self.sq_pseudolegal_moves(board, sq, pt));
            }
        }
        return moves;
    }

    //In a given board state, is a move illegal because it would result in a self-check?
    pub fn is_move_self_check(&self, board: &Board, m: Move) -> bool {
        let mut newboard = *board;
        let player = board.color_at_square(m.from_square());
        newboard.make_move(m);
        let player_king_bb = newboard.get_pieces_of_type_and_color(KING, player);
        let player_king_square = Square::from(player_king_bb);
        self.is_square_attacked_by(&newboard, player_king_square, opposite_color(player))
    }

    //In a given board state, is a square attacked by the given color?
    pub fn is_square_attacked_by(&self, board: &Board, sq: Square, color: Color) -> bool {
        let moves = self.get_pseudolegal_moves(board, color);
        for m in moves {
            if m.to_square() == sq {
                return true;
            }
        }
        return false;
    }

    //Enumerate all the pseudolegal moves made by a certain type at a certain
    //square in this position.
    #[inline]
    fn sq_pseudolegal_moves(&self, board: &Board, sq: Square, pt: PieceType) -> Vec<Move> {
        match pt {
            PAWN => self.pawn_moves(board, sq),
            KNIGHT => self.knight_moves(board, sq),
            KING => self.king_moves(board, sq),
            BISHOP => self.bishop_moves(board, sq),
            ROOK => self.rook_moves(board, sq),
            QUEEN => self.queen_moves(board, sq),
            //bad type gets empty vector of moves
            _ => Vec::new(),
        }
    }

    #[inline]
    //bob seger
    fn knight_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        let moves_bb = self.knight_moves[sq.0 as usize]
            & !board.get_color_occupancy(board.color_at_square(sq));
        return bitboard_to_moves(sq, moves_bb);
    }

    #[inline]
    fn king_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        let moves_bb =
            self.king_moves[sq.0 as usize] & !board.get_color_occupancy(board.color_at_square(sq));
        #[allow(unused_mut)]
        let mut moves = bitboard_to_moves(sq, moves_bb);
        //TODO add castling moves

        return moves;
    }

    //Generate pseudo-legal pawn moves for a from-square in a given position
    fn pawn_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        let player_color = board.color_at_square(sq);
        let dir = pawn_direction(player_color);
        let start_rank = pawn_start_rank(player_color);
        let promote_rank = pawn_promote_rank(player_color);
        let from_bb = Bitboard::from(sq);
        let occupancy = board.get_occupancy();
        let capture_sqs = [sq + dir + EAST, sq + dir + WEST];
        let opponents = board.get_color_occupancy(board.color_at_square(sq));
        let mut target_squares = BB_EMPTY;
        //this will never be out of bounds because pawns don't live on promotion rank
        if !occupancy.is_square_occupied(sq + dir) {
            target_squares |= Bitboard::from(sq + dir);
            //pawn is on start rank and double-move square is not occupied
            if (start_rank & from_bb) != BB_EMPTY && !occupancy.is_square_occupied(sq + 2 * dir) {
                target_squares |= Bitboard::from(sq + 2 * dir);
            }
        }
        //captures
        for capture_sq in capture_sqs {
            if capture_sq.is_inbounds() && capture_sq.chebyshev_to(sq) < 2 {
                if capture_sq == board.en_passant_square {
                    target_squares |= Bitboard::from(capture_sq);
                }
                let capture_bb = Bitboard::from(capture_sq);
                target_squares |= capture_bb & opponents;
            }
        }
        let promotion_bb = target_squares & promote_rank;
        let not_promotion_bb = target_squares & !promote_rank;
        let mut moves = bitboard_to_moves(sq, not_promotion_bb);
        if promotion_bb != BB_EMPTY {
            for promote_type in PROMOTE_TYPES {
                moves.extend(bitboard_to_promotions(sq, promotion_bb, promote_type));
            }
        }
        return moves;
    }

    //Generate pseudo-legal bishop moves for a from-square in a given position
    #[inline]
    fn bishop_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        bitboard_to_moves(
            sq,
            get_bishop_attacks(board.get_occupancy(), sq, &self.mtable)
                & !board.get_color_occupancy(board.color_at_square(sq)),
        )
    }

    #[inline]
    fn rook_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        bitboard_to_moves(
            sq,
            get_rook_attacks(board.get_occupancy(), sq, &self.mtable)
                & !board.get_color_occupancy(board.color_at_square(sq)),
        )
    }

    //Enumerating pseudolegal moves for each piece type
    fn queen_moves(&self, board: &Board, sq: Square) -> Vec<Move> {
        let mut moves = self.rook_moves(board, sq);
        moves.extend(self.bishop_moves(board, sq));
        return moves;
    }
}

fn create_step_attacks(dirs: &Vec<Direction>, max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard(0); 64];
    for i in 0..64usize {
        for dir in dirs {
            let start_sq = Square(i as u8);
            let target_sq = start_sq + *dir;
            if target_sq.chebyshev_to(start_sq) <= max_dist {
                attacks[i] |= Bitboard::from(target_sq);
            }
        }
    }
    return attacks;
}

#[inline]
fn bitboard_to_moves(from_sq: Square, bb: Bitboard) -> Vec<Move> {
    bitboard_to_promotions(from_sq, bb, NO_TYPE)
}

fn bitboard_to_promotions(from_sq: Square, bb: Bitboard, promote_type: PieceType) -> Vec<Move> {
    let mut targets = bb;
    let mut moves = Vec::new();
    while targets != BB_EMPTY {
        let to_sq = Square::from(targets);
        moves.push(Move::new(from_sq, to_sq, promote_type));
        targets &= !Bitboard::from(to_sq);
    }
    return moves;
}

//get the steps a king can make
fn get_king_steps() -> Vec<Direction> {
    vec![
        NORTH, NORTHEAST, EAST, SOUTHEAST, SOUTH, SOUTHWEST, WEST, NORTHWEST,
    ]
}
//get the steps a knight can make
fn get_knight_steps() -> Vec<Direction> {
    vec![NNW, NNE, NEE, SEE, SSE, SSW, SWW, NWW]
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_opening_moveset() {
        let mg = MoveGenerator::new();
        let moves = mg.get_moves(&Board::new());
        print!("{{");
        for m in moves.iter() {
            print!("{}, ", m);
        }
        print!("}}");
    }
}
