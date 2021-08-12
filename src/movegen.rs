use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::board::Board;
use crate::direction::{
    Direction, EAST, NEE, NNE, NNW, NORTH, NORTHEAST, NORTHWEST, NWW, SEE, SOUTH, SOUTHEAST,
    SOUTHWEST, SSE, SSW, SWW, WEST,
};
use crate::magic::{
    create_empty_magic, get_bishop_attacks, get_rook_attacks, load_magic, MagicTable,
};
use crate::piece::{
    PieceType, BISHOP, KING, KNIGHT, NO_TYPE, PAWN, PIECE_TYPES, PROMOTE_TYPES, QUEEN, ROOK,
};
use crate::r#move::Move;
use crate::square::{Square, B2, C3};
use crate::util::{opposite_color, pawn_direction, pawn_promote_rank, pawn_start_rank};

//square where the king would be to have moveset KING_MOVE_MASK
const KING_MOVE_SQ: Square = B2;
//bitboard of places king could go from square KING_MOVE_SQ
const KING_MOVE_MASK: Bitboard = Bitboard(0x70507);
const KING_STEPS: &[Direction] = &[
    NORTH, NORTHEAST, EAST, SOUTHEAST, SOUTH, SOUTHWEST, WEST, NORTHWEST,
];

//square where the knight would be to have moveset KNIGHT_MOVE_MASK
const KNIGHT_MOVE_SQ: Square = C3;
//bitboard of places knight could go from square KNIGHT_MOVE_SQ
const KNIGHT_MOVE_MASK: Bitboard = Bitboard(0xA1100110A);
const KNIGHT_STEPS: &[Direction] = &[NNW, NNE, NEE, SEE, SSE, SSW, SWW, NWW];

//All the saved data necessary to generate moves
pub struct MoveGenData {
    magic: MagicTable,
    pawn_attacks: [Bitboard; 64],
    pawn_moves: [Bitboard; 64],
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

/*
pub fn create_move_gen_data() -> MoveGenData {
    let mut mtable = create_empty_magic();
    load_magic(&mtable);
    MoveGenData{
        magic: mtable,
        pawn_attacks: create_pawn_attacks(),
        pawn_moves: create_pawn_moves(),
        king_moves: create_king_moves(),
        knight_moves: create_knight_moves()
    }
}*/

//Enumerate pseudo-legal moves in the current position
#[allow(dead_code)]
pub fn get_pseudolegal_moves(board: &Board, mtable: &MagicTable) -> Vec<Move> {
    let mut moves = Vec::new();
    for pt in PIECE_TYPES {
        let mut pieces_to_move = board.get_pieces_of_type_and_color(pt, board.player_to_move);
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
#[inline]
fn sq_pseudolegal_moves(
    board: &Board,
    sq: Square,
    pt: PieceType,
    mtable: &MagicTable,
) -> Vec<Move> {
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

#[inline]
//bob seger
fn knight_moves(board: &Board, sq: Square) -> Vec<Move> {
    step_moves(board, sq, KNIGHT_MOVE_MASK, KNIGHT_MOVE_SQ)
}

#[inline]
fn king_moves(board: &Board, sq: Square) -> Vec<Move> {
    step_moves(board, sq, KING_MOVE_MASK, KING_MOVE_SQ)
}

fn step_moves(board: &Board, sq: Square, mask: Bitboard, ref_sq: Square) -> Vec<Move> {
    //difference in position between the reference mask square and the current
    //square
    let shift = (ref_sq.0 as i8) - (sq.0 as i8);
    //bitboard of places this step piece can move to
    let move_bb = (mask >> shift) & !board.get_color_occupancy(board.player_to_move);
    return bitboard_to_moves(sq, move_bb);
}

//Generate pseudo-legal pawn moves for a from-square in a given position
fn pawn_moves(board: &Board, sq: Square) -> Vec<Move> {
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
fn bishop_moves(board: &Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
    bitboard_to_moves(
        sq,
        get_bishop_attacks(board.get_occupancy(), sq, mtable)
            & !board.get_color_occupancy(board.player_to_move),
    )
}

#[inline]
fn rook_moves(board: &Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
    bitboard_to_moves(
        sq,
        get_rook_attacks(board.get_occupancy(), sq, mtable)
            & !board.get_color_occupancy(board.player_to_move),
    )
}

//Enumerating pseudolegal moves for each piece type
fn queen_moves(board: &Board, sq: Square, mtable: &MagicTable) -> Vec<Move> {
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

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;
    use crate::constants::WHITE;
    use crate::magic::{create_empty_magic, load_magic};
    use crate::square::BAD_SQUARE;

    const STARTING_BOARD: &Board = &Board {
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
    };

    #[test]
    fn test_opening_moveset() {
        let mut mtable = create_empty_magic();
        load_magic(&mut mtable);
        let moves = get_pseudolegal_moves(STARTING_BOARD, &mtable);
        print!("{{");
        for m in moves.iter() {
            print!("{}, ", m);
        }
        print!("}}");
    }
}
