use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::board::Board;
use crate::constants::Color;
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
use crate::square::Square;
use crate::util::{opposite_color, pawn_direction, pawn_promote_rank, pawn_start_rank};

//All the saved data necessary to generate moves
#[allow(dead_code)]
pub struct MoveGenData {
    mtable: MagicTable,
    pawn_attacks: [Bitboard; 64], //for now unused, will be used later
    king_moves: [Bitboard; 64],
    knight_moves: [Bitboard; 64],
}

#[allow(dead_code)]
pub fn create_move_gen_data() -> MoveGenData {
    let mut mtable = create_empty_magic();
    load_magic(&mut mtable);
    MoveGenData {
        mtable: mtable,
        pawn_attacks: create_step_attacks(&vec![NORTHEAST, NORTHWEST], 1),
        king_moves: create_step_attacks(&get_king_steps(), 1),
        knight_moves: create_step_attacks(&get_knight_steps(), 2),
    }
}

#[allow(dead_code)]
pub fn get_moves(board: &Board, mdata: &MoveGenData) -> Vec<Move> {
    let moves = get_pseudolegal_moves(board, mdata);
    let mut legal_moves = Vec::<Move>::new();
    for m in moves {
        if !is_move_self_check(board, m, mdata) {
            legal_moves.push(m);
        }
    }
    return legal_moves;
}

//Enumerate pseudo-legal moves in the current position
#[inline]
pub fn get_pseudolegal_moves(board: &Board, mdata: &MoveGenData) -> Vec<Move> {
    get_pseudolegal_moves_of_color(board, mdata, board.player_to_move)
}

//This enumerates pseudolegal moves if the player of a given color were playing.
pub fn get_pseudolegal_moves_of_color(
    board: &Board,
    mdata: &MoveGenData,
    color: Color,
) -> Vec<Move> {
    let mut moves = Vec::new();
    //iterate through all the pieces of this color and enumerate their moves
    for pt in PIECE_TYPES {
        let mut pieces_to_move = board.get_pieces_of_type_and_color(pt, color);
        while pieces_to_move != BB_EMPTY {
            //square of next piece to move
            let sq = Square::from(pieces_to_move);
            //remove that square
            pieces_to_move &= !Bitboard::from(sq);
            moves.extend(sq_pseudolegal_moves(board, sq, pt, mdata));
        }
    }
    return moves;
}

//In a given board state, is a move illegal because it would result in a self-check?
pub fn is_move_self_check(board: &Board, m: Move, mdata: &MoveGenData) -> bool {
    let mut newboard = *board;
    let player = board.color_at_square(m.from_square());
    newboard.make_move(m);
    let player_king_bb = newboard.get_pieces_of_type_and_color(KING, player);
    let player_king_square = Square::from(player_king_bb);
    is_square_attacked_by(&newboard, player_king_square, opposite_color(player), mdata)
}

//In a given board state, is a square attacked by the given color?
pub fn is_square_attacked_by(board: &Board, sq: Square, color: Color, mdata: &MoveGenData) -> bool {
    let moves = get_pseudolegal_moves_of_color(board, mdata, color);
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
fn sq_pseudolegal_moves(
    board: &Board,
    sq: Square,
    pt: PieceType,
    mdata: &MoveGenData,
) -> Vec<Move> {
    match pt {
        PAWN => pawn_moves(board, sq, mdata),
        KNIGHT => knight_moves(board, sq, mdata),
        KING => king_moves(board, sq, mdata),
        BISHOP => bishop_moves(board, sq, mdata),
        ROOK => rook_moves(board, sq, mdata),
        QUEEN => queen_moves(board, sq, mdata),
        //bad type gets empty vector of moves
        _ => Vec::new(),
    }
}

#[inline]
//bob seger
fn knight_moves(board: &Board, sq: Square, mdata: &MoveGenData) -> Vec<Move> {
    let moves_bb =
        mdata.knight_moves[sq.0 as usize] & !board.get_color_occupancy(board.color_at_square(sq));
    return bitboard_to_moves(sq, moves_bb);
}

#[inline]
fn king_moves(board: &Board, sq: Square, mdata: &MoveGenData) -> Vec<Move> {
    let moves_bb =
        mdata.king_moves[sq.0 as usize] & !board.get_color_occupancy(board.color_at_square(sq));
    #[allow(unused_mut)]
    let mut moves = bitboard_to_moves(sq, moves_bb);
    //TODO add castling moves

    return moves;
}

//Generate pseudo-legal pawn moves for a from-square in a given position
fn pawn_moves(board: &Board, sq: Square, _mdata: &MoveGenData) -> Vec<Move> {
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
fn bishop_moves(board: &Board, sq: Square, mdata: &MoveGenData) -> Vec<Move> {
    bitboard_to_moves(
        sq,
        get_bishop_attacks(board.get_occupancy(), sq, &mdata.mtable)
            & !board.get_color_occupancy(board.color_at_square(sq)),
    )
}

#[inline]
fn rook_moves(board: &Board, sq: Square, mdata: &MoveGenData) -> Vec<Move> {
    bitboard_to_moves(
        sq,
        get_rook_attacks(board.get_occupancy(), sq, &mdata.mtable)
            & !board.get_color_occupancy(board.color_at_square(sq)),
    )
}

//Enumerating pseudolegal moves for each piece type
fn queen_moves(board: &Board, sq: Square, mdata: &MoveGenData) -> Vec<Move> {
    let mut moves = rook_moves(board, sq, mdata);
    moves.extend(bishop_moves(board, sq, mdata));
    return moves;
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
    use crate::constants::WHITE;
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
        let mdata = create_move_gen_data();
        let moves = get_moves(STARTING_BOARD, &mdata);
        print!("{{");
        for m in moves.iter() {
            print!("{}, ", m);
        }
        print!("}}");
    }
}
