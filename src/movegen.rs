use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::board::Board;
use crate::constants::{Color, BLACK, WHITE};
use crate::direction::{Direction, EAST, NORTH, SOUTH, WEST};
use crate::magic::{get_bishop_attacks, get_rook_attacks, MagicTable};
use crate::piece::{PieceType, NO_TYPE, PIECE_TYPES, PROMOTE_TYPES};
use crate::r#move::Move;
use crate::square::Square;
use crate::util::opposite_color;

//Enumerate pseudo-legal moves in the current position
pub fn get_pseudolegal_moves(board: Board, mtable: &MagicTable) -> Vec<Move> {
    let mut moves = Vec::new();
    let side_to_move = board.sides[board.player_to_move];
    for pt in PIECE_TYPES {
        let mut pieces_to_move = side_to_move & board.pieces[pt.0 as usize];
        while pieces_to_move != BB_EMPTY {
            //square of next piece to move
            let sq = Square::from(pieces_to_move);
            //remove that square
            pieces_to_move &= !Bitboard::from(sq);
            moves.extend(sq_pseudolegal_moves(board, sq, pt, mtable));
        }
    }
    return moves;
}

//Enumerate all the pseudolegal moves made by a certain type at a certain
//square in this position.
fn sq_pseudolegal_moves(board: Board, sq: Square, pt: PieceType, mtable: &MagicTable) -> Vec<Move> {
    match pt {
        PAWN => pawn_moves(board, sq),
        KNIGHT => knight_moves(board, sq),
        KING => king_moves(board, sq),
        BISHOP => bishop_moves(board, sq, mtable),
        ROOK => rook_moves(board, sq, mtable),
        QUEEN => queen_moves(board, sq, mtable),
        //bad type gets empty vector of moves
        _ => Vec::new(),
    }
}

//Generate pseudo-legal pawn moves for a from-square in a given position
fn pawn_moves(board: Board, sq: Square) -> Vec<Move> {
    let dir = pawn_direction(board.player_to_move);
    let start_rank = pawn_start_rank(board.player_to_move);
    let promote_rank = pawn_promote_rank(board.player_to_move);
    let from_bb = Bitboard::from(sq);
    let occupancy = board.get_occupancy();
    let capture_sqs = [sq + dir + EAST, sq + dir + WEST];
    let opponents = board.get_color_occupancy(opposite_color(board.player_to_move));
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
        if capture_sq.is_inbounds() {
            let capture_bb = Bitboard::from(capture_sq);
            target_squares |= capture_bb & (opponents | Bitboard::from(board.en_passant_square));
        }
    }
    let promotion_bb = target_squares & promote_rank;
    let not_promotion_bb = target_squares & !promote_rank;
    let mut moves = bitboard_to_moves(sq, not_promotion_bb);
    for promote_type in PROMOTE_TYPES {
        moves.extend(bitboard_to_promotions(sq, promotion_bb, promote_type));
    }
    return moves;
}

//Generate pseudo-legal bishop moves for a from-square in a given position
#[inline]
fn bishop_moves(board: Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
    bitboard_to_moves(sq, get_bishop_attacks(board.get_occupancy(), sq, mtable))
}

#[inline]
fn rook_moves(board: Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
    bitboard_to_moves(sq, get_rook_attacks(board.get_occupancy(), sq, mtable))
}

//Enumerating pseudolegal moves for each piece type
fn queen_moves(board: Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
    let mut moves = rook_moves(board, sq, mtable);
    moves.extend(bishop_moves(board, sq, mtable));
    return moves;
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

#[inline]
const fn pawn_direction(color: Color) -> Direction {
    match color {
        WHITE => NORTH,
        BLACK => SOUTH,
    }
}

#[inline]
const fn pawn_promote_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0xFF00000000000000),
        BLACK => Bitboard(0x00000000000000FF),
    }
}

#[inline]
const fn pawn_start_rank(color: Color) -> Bitboard {
    match color {
        WHITE => Bitboard(0x000000000000FF00),
        BLACK => Bitboard(0x00FF000000000000),
    }
}
