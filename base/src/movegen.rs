/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Generation and verification of legal moves in a position.

use once_cell::sync::Lazy;

use crate::{
    game::{NoTag, Tagger},
    MAGIC,
};

use super::{moves::Move, Bitboard, Board, Color, Direction, Piece, Square};

use std::{convert::TryFrom, time::Instant};

/// A bitboard of all the squares a knight can move to if its position is
/// the index of the list.
static KNIGHT_MOVES: Lazy<[Bitboard; 64]> =
    Lazy::new(|| create_step_attacks(&Direction::KNIGHT_STEPS, 2));

/// A bitboard of all the squares a king can move to if his position is the
/// index in the list.
static KING_MOVES: Lazy<[Bitboard; 64]> =
    Lazy::new(|| create_step_attacks(&Direction::KING_STEPS, 1));

/// A bitboard of all the squares which a pawn on the given square can
/// attack. The first index is for White's pawn attacks, the second is for
/// Black's.
pub(crate) static PAWN_ATTACKS: Lazy<[[Bitboard; 64]; 2]> = Lazy::new(|| {
    [
        create_step_attacks(&[Direction::NORTHEAST, Direction::NORTHWEST], 1),
        create_step_attacks(&[Direction::SOUTHEAST, Direction::SOUTHWEST], 1),
    ]
});

/// The types of move generation. These are used in const generics, as enums are
/// not supported in const generics.
pub type GenMode = u8;

/// The mode identifier for `get_moves()` to generate all legal moves.
pub const ALL: GenMode = 0;
/// The mode identifier for `get_moves()` to generate captures only.
pub const CAPTURES: GenMode = 1;
/// The mode identifier for `get_moves()` to generate non-captures only.
pub const QUIETS: GenMode = 2;

#[must_use]
/// Determine whether any given move is legal, given a position in which it
/// could be played.
/// Requires that the move must have been legal on *some* board, but not
/// necessarily the given one.
///
/// # Panics
///
/// This function might panic, but this is only due to an internal error.
///
/// # Examples
///
/// ```
/// use fiddler_base::{Board, Move, movegen::is_legal, Square};
///
/// let board = Board::new();
/// assert!(is_legal(Move::normal(Square::E2, Square::E4), &board));
/// assert!(!is_legal(Move::normal(Square::E2, Square::D4), &board));
/// ```
pub fn is_legal(m: Move, b: &Board) -> bool {
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let from_bb = Bitboard::from(from_sq);
    let to_bb = Bitboard::from(to_sq);
    let player = b.player;
    let allies = b[player];
    let enemies = b[!player];
    let occupancy = allies | enemies;
    if allies.contains(to_sq) {
        // cannot move to square occupied by our piece
        return false;
    }
    if !allies.contains(from_sq) {
        return false;
    }
    match b.type_at_square(from_sq) {
        Some(Piece::King) => {
            if m.promote_type().is_some() {
                // cannot promote non-pawn
                return false;
            }

            if m.is_en_passant() {
                // king cannot en passant
                return false;
            }
            if m.is_castle() {
                // just generate moves, since castle is quite rare
                let mut move_buf = Vec::with_capacity(2);
                castles::<NoTag>(b, &(), &mut move_buf);
                return move_buf.contains(&(m, ()));
            }

            if !KING_MOVES[from_sq as usize].contains(to_sq) {
                return false;
            }

            // normal king moves can't step into check
            let new_occupancy = (b.occupancy() ^ from_bb) | to_bb;
            square_attackers_occupancy(b, to_sq, !b.player, new_occupancy).is_empty()
        }
        Some(pt) => {
            if b.checkers.more_than_one() {
                // non-kings can't get out of double check
                return false;
            }

            if pt != Piece::Pawn && m.is_promotion() {
                // cannot promote non-pawn
                return false;
            }

            if m.is_castle() {
                // only kings can castle
                return false;
            }

            let is_ep = m.is_en_passant();
            if is_ep {
                if pt != Piece::Pawn {
                    // only pawns can en passant
                    return false;
                }

                if b.en_passant_square != Some(to_sq) {
                    // en passant must target the en passant square
                    return false;
                }
            }

            // first, validate pseudolegality
            if !match pt {
                Piece::Pawn => {
                    let pawn_dir = player.pawn_direction();
                    let singlemove_sq = from_sq + pawn_dir;
                    let pattacks = PAWN_ATTACKS[player as usize][from_sq as usize];
                    (!occupancy.contains(singlemove_sq)
                        && (to_sq == singlemove_sq //singlemove
                        || (to_sq == singlemove_sq + pawn_dir //doublemove
                            && player.pawn_start_rank().contains(from_sq)
                            && !occupancy.contains(to_sq))))
                        || (is_ep && b.en_passant_square == Some(to_sq))
                        || (!is_ep && (pattacks & enemies).contains(m.to_square()))
                }
                Piece::Knight => KNIGHT_MOVES[from_sq as usize].contains(to_sq),
                Piece::Bishop => MAGIC
                    .bishop_attacks(allies | enemies, from_sq)
                    .contains(to_sq),
                Piece::Rook => MAGIC
                    .rook_attacks(allies | enemies, from_sq)
                    .contains(to_sq),
                Piece::Queen => {
                    let occupancy = allies | enemies;
                    (MAGIC.bishop_attacks(occupancy, from_sq)
                        | MAGIC.rook_attacks(occupancy, from_sq))
                    .contains(to_sq)
                }
                Piece::King => unreachable!(),
            } {
                return false;
            };

            // check that the move is not a self check
            if !b.checkers.is_empty() {
                // we already handled the two-checker case, so there is only one
                // checker
                let checker_sq = Square::try_from(b.checkers).unwrap();
                let player_idx = b.player as usize;
                let mut targets = Bitboard::between(b.king_sqs[player_idx], checker_sq)
                    | Bitboard::from(checker_sq);

                if let Some(ep_sq) = b.en_passant_square {
                    if pt == Piece::Pawn && (checker_sq == ep_sq - player.pawn_direction()) {
                        // allow en passants that let us escape check
                        targets.insert(ep_sq);
                    }
                }

                if !targets.contains(to_sq) {
                    return false;
                }
            };

            if is_ep {
                // en passants have their own weird effects
                let king_sq = b.king_sqs[b.player as usize];
                let capture_bb = match player {
                    Color::White => to_bb >> 8,
                    Color::Black => to_bb << 8,
                };

                let new_occupancy = b.occupancy() ^ from_bb ^ capture_bb ^ to_bb;

                return (MAGIC.rook_attacks(new_occupancy, king_sq)
                    & (b[Piece::Rook] | b[Piece::Queen])
                    & enemies)
                    .is_empty()
                    && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                        & (b[Piece::Bishop] | b[Piece::Queen])
                        & enemies)
                        .is_empty();
            }

            !b.pinned.contains(from_sq)
                || Square::aligned(from_sq, to_sq, b.king_sqs[player as usize])
        }
        None => false,
    }
}

#[inline(always)]
#[must_use]
/// Get the legal moves in a board.
///
/// `M` is the generation mode of move generation: it specifies which subset of
/// all legal moves to generate. There are currently 3 legal generation modes:
///
/// * `ALL` will generate all legal moves.
/// * `CAPTURES` will generate all captures, including en passant.
/// * `QUIETS` will generate all quiet (i.e. non-capture) moves.
///
/// `T` is a tagger for moves: it contains a callback function to tag moves as
/// they are generated so that the user can save on total heap allocations.
/// If no tag is needed, you can use `fiddler_base::game::NoTag` to avoid
/// wasting effort tagging each move.
///
/// # Examples
///
/// Generate all legal moves:
/// ```
/// use fiddler_base::{Board, game::NoTag, movegen::{ALL, is_legal, get_moves}};
///
/// let b = Board::new();
/// for (m, _) in get_moves::<ALL, NoTag>(&b, &()) {
///     assert!(is_legal(m, &b));
/// }
/// ```
///
/// Generate captures:
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>>{
/// use fiddler_base::{Board, game::NoTag, Move, movegen::{CAPTURES, is_legal, get_moves}, Square};
///
/// // Scandinavian defense. The only legal capture is exd5.
/// let b = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")?;
///
/// assert_eq!(
///     get_moves::<CAPTURES, NoTag>(&b, &()),
///     vec![(Move::normal(Square::E4, Square::D5), ())],
/// );
/// # Ok(())
/// # }
/// ```
///
/// Generate quiet moves:
///
/// ```
/// use fiddler_base::{Board, game::NoTag, movegen::{QUIETS, is_legal, get_moves}};
///
/// let b = Board::new();
/// for (m, _) in get_moves::<QUIETS, NoTag>(&b, &()) {
///     assert!(is_legal(m, &b));
///     assert!(!b.is_move_capture(m));
/// }
/// ```
pub fn get_moves<const M: GenMode, T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
) -> Vec<(Move, T::Tag)> {
    // prevent wonky generation modes
    debug_assert!(M == ALL || M == CAPTURES || M == QUIETS);

    if b.is_drawn() {
        return Vec::new();
    }

    let mut moves;
    let in_check = !b.checkers.is_empty();

    if in_check {
        // in the overwhelming majority of cases, there are 8 or fewer
        // legal evasions if the king is in check
        moves = Vec::with_capacity(8);
        evasions::<M, T>(b, cookie, &mut moves);
    } else {
        // in the overwhelming majority of cases, there are fewer than 50
        // legal moves total
        let capacity = match M {
            ALL => 50,
            CAPTURES => 8,
            QUIETS => 40,
            _ => unreachable!(),
        };
        moves = Vec::with_capacity(capacity);
        non_evasions::<M, T>(b, cookie, &mut moves);
    };

    moves
}

#[must_use]
/// Does the player to move have any legal moves in this position?
/// Requires that the board is legal (i.e. has one of each king) to be correct.
///
/// Note that since a `Board` does not contain historical information, it will
/// still return `true` on positions with repetition.
///
/// # Examples
///
/// ```
/// use fiddler_base::{Board, movegen::has_moves};
///
/// let b = Board::new();
/// assert!(has_moves(&b));
/// ```
pub fn has_moves(b: &Board) -> bool {
    let player = b.player;
    let allies = b[player];
    let enemies = b[!player];
    let opponent = !player;
    let occupancy = allies | enemies;
    let mut legal_targets = !allies;
    let king_sq = b.king_sqs[player as usize];
    // king does not have to block its checks
    let king_to_sqs = KING_MOVES[king_sq as usize] & legal_targets;
    let unpinned = !b.pinned;
    let queens = b[Piece::Queen];

    if b.is_drawn() {
        return false;
    }

    if b.checkers.more_than_one() {
        // in double check, only consider king moves
        let new_occupancy = occupancy ^ Bitboard::from(king_sq);
        for to_sq in king_to_sqs {
            if square_attackers_occupancy(b, to_sq, opponent, new_occupancy).is_empty() {
                return true;
            }
        }

        return false;
    }

    // king is either single-checked or not at all
    if !b.checkers.is_empty() {
        // SAFETY: We checked that the square is nonzero.
        let checker_sq = unsafe { Square::unsafe_from(b.checkers) };
        // Look for blocks or captures
        legal_targets &= Bitboard::between(king_sq, checker_sq) | b.checkers;
    }
    // save the (expensive) king move generation/validation for later

    // pinned knights can never move, but an unpinned knight does whatever it
    // wants
    for sq in b[Piece::Knight] & allies & unpinned {
        if !(KNIGHT_MOVES[sq as usize] & legal_targets).is_empty() {
            return true;
        }
    }

    // unpinned bishops/diagonal queens
    let bishop_movers = (b[Piece::Bishop] | queens) & allies;
    for sq in bishop_movers & unpinned {
        if !(MAGIC.bishop_attacks(occupancy, sq) & legal_targets).is_empty() {
            return true;
        }
    }

    // pinned bishops/diagonal queens
    for sq in bishop_movers & b.pinned {
        if !(MAGIC.bishop_attacks(occupancy, sq) & legal_targets & Bitboard::line(king_sq, sq))
            .is_empty()
        {
            return true;
        }
    }

    let rook_movers = (b[Piece::Rook] | queens) & allies;
    // unpinned rooks/horizontal queens
    for sq in rook_movers & unpinned {
        if !(MAGIC.rook_attacks(occupancy, sq) & legal_targets).is_empty() {
            return true;
        }
    }

    // pinned rooks/horizontal queens
    for sq in rook_movers & b.pinned {
        if !(MAGIC.rook_attacks(occupancy, sq) & legal_targets & Bitboard::line(king_sq, sq))
            .is_empty()
        {
            return true;
        }
    }

    // "normal" pawn moves
    let our_pawns = b[Piece::Pawn] & allies;
    for sq in our_pawns {
        let singlemove_sq = sq + player.pawn_direction();
        let mut to_bb = Bitboard::from(singlemove_sq);
        if !occupancy.contains(singlemove_sq) && player.pawn_start_rank().contains(sq) {
            to_bb.insert(singlemove_sq + player.pawn_direction());
        }

        to_bb &= !occupancy;
        to_bb |= PAWN_ATTACKS[player as usize][sq as usize] & enemies;
        to_bb &= legal_targets;

        if b.pinned.contains(sq) {
            to_bb &= Bitboard::line(king_sq, sq);
        }

        if !to_bb.is_empty() {
            return true;
        }
    }

    // en passant might save us
    if let Some(ep_sq) = b.en_passant_square {
        let ep_less_occupancy = occupancy ^ Bitboard::from(ep_sq);
        for sq in our_pawns {
            if PAWN_ATTACKS[player as usize][sq as usize].contains(ep_sq) {
                let new_occupancy = ep_less_occupancy ^ Bitboard::from(sq);
                if (MAGIC.rook_attacks(new_occupancy, king_sq)
                    & (b[Piece::Rook] | b[Piece::Queen])
                    & enemies)
                    .is_empty()
                    && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                        & (b[Piece::Bishop] | b[Piece::Queen])
                        & enemies)
                        .is_empty()
                {
                    return true;
                }
            }
        }
    }

    // king moves are expensive to validate
    let new_occupancy = occupancy ^ Bitboard::from(king_sq);
    for to_sq in king_to_sqs {
        if square_attackers_occupancy(b, to_sq, opponent, new_occupancy).is_empty() {
            return true;
        }
    }

    false
}

#[inline(always)]
#[must_use]
/// In a given board state, is a square attacked by the given color?
/// Squares which are threatened by only non-capture moves (i.e. pawn-pushes)
/// will not qualify as attacked.
///
/// # Examples
///
/// ```
/// use fiddler_base::{Board, Square, Color, movegen::is_square_attacked_by};
///
/// let b = Board::new();
/// assert!(is_square_attacked_by(&b, Square::E2, Color::White));
/// ```
pub fn is_square_attacked_by(board: &Board, sq: Square, color: Color) -> bool {
    !square_attackers(board, sq, color).is_empty()
}

#[inline(always)]
/// Enumerate the legal moves a player of the given color would be
/// able to make if it were their turn to move, and if the player is not in
/// check.
fn non_evasions<const M: GenMode, T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
) {
    let target_sqs = match M {
        ALL => Bitboard::ALL,
        CAPTURES => b[!b.player],
        QUIETS => !b[!b.player],
        _ => unreachable!(),
    };

    let mut pawn_targets = target_sqs;
    if M != QUIETS {
        if let Some(ep_sq) = b.en_passant_square {
            pawn_targets.insert(ep_sq);
        }
    }
    pawn_assistant::<M, T>(b, cookie, moves, pawn_targets);

    normal_piece_assistant::<T>(b, cookie, moves, target_sqs);

    // generate king moves
    if M != CAPTURES {
        castles::<T>(b, cookie, moves);
    }
    king_move_non_castle::<T>(b, cookie, moves, target_sqs);
}

/// Compute the evasions in a position where the king is checked, and then push
/// those evading moves into the moves buffer.
fn evasions<const M: GenMode, T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
) {
    let player = b.player;
    let king_sq = b.king_sqs[player as usize];

    // only look at non-king moves if we are not in double check
    if b.checkers.has_single_bit() {
        // SAFETY: We checked that the square is nonzero.
        let checker_sq = unsafe { Square::unsafe_from(b.checkers) };
        // Look for blocks or captures
        let mut target_sqs = Bitboard::between(king_sq, checker_sq) | b.checkers;
        match M {
            ALL => (),
            CAPTURES => target_sqs &= b[!player],
            QUIETS => target_sqs &= !b[!player],
            _ => unreachable!(),
        }

        let mut pawn_targets = target_sqs;
        if M != QUIETS {
            if let Some(ep_sq) = b.en_passant_square {
                // can en passant save us from check?
                let ep_attacker_sq = ep_sq - player.pawn_direction();
                if b.checkers.contains(ep_attacker_sq) {
                    pawn_targets.insert(ep_sq);
                }
            }
        }

        pawn_assistant::<M, T>(b, cookie, moves, pawn_targets);
        normal_piece_assistant::<T>(b, cookie, moves, target_sqs);
    }

    let king_targets = match M {
        ALL => Bitboard::ALL,
        CAPTURES => b[!player],
        QUIETS => !b[!player],
        _ => unreachable!(),
    };
    king_move_non_castle::<T>(b, cookie, moves, king_targets);
}

#[inline(always)]
#[must_use]
/// Get the attackers of a given color on a square as a `Bitboard`
/// representing the squares of the attackers.
///
/// # Examples
///
/// ```
/// use fiddler_base::{Bitboard, Board, Square, Color, movegen::square_attackers};
///
/// let b = Board::new();
/// let mut attackers = Bitboard::EMPTY;
/// attackers.insert(Square::E1);
/// attackers.insert(Square::D1);
/// attackers.insert(Square::F1);
/// attackers.insert(Square::G1);
/// assert_eq!(square_attackers(&b, Square::E2, Color::White), attackers);
/// ```
pub fn square_attackers(board: &Board, sq: Square, color: Color) -> Bitboard {
    square_attackers_occupancy(board, sq, color, board.occupancy())
}

/// Same functionality as `square_attackers`, but uses the provided
/// `occupancy` bitboard (as opposed to the board's occupancy.)
fn square_attackers_occupancy(
    board: &Board,
    sq: Square,
    color: Color,
    occupancy: Bitboard,
) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    let color_bb = board[color];
    // Check for pawn attacks
    let pawn_vision = PAWN_ATTACKS[!color as usize][sq as usize];
    attackers |= pawn_vision & board[Piece::Pawn];

    // Check for knight attacks
    let knight_vision = KNIGHT_MOVES[sq as usize];
    attackers |= knight_vision & board[Piece::Knight];

    let queens_bb = board[Piece::Queen];

    // Check for rook/horizontal queen attacks
    let rook_vision = MAGIC.rook_attacks(occupancy, sq);
    attackers |= rook_vision & (queens_bb | board[Piece::Rook]);

    // Check for bishop/diagonal queen attacks
    let bishop_vision = MAGIC.bishop_attacks(occupancy, sq);
    attackers |= bishop_vision & (queens_bb | board[Piece::Bishop]);

    // Check for king attacks
    let king_vision = KING_MOVES[sq as usize];
    attackers |= king_vision & board[Piece::King];

    attackers & color_bb
}

/// Generate the moves all pawns can make and populate `moves` with those
/// moves. `target` is the set of squares for which it is desired to move a
/// pawn.
fn pawn_assistant<const M: GenMode, T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
    target: Bitboard,
) {
    const COL_A: Bitboard = Bitboard::new(0x0101_0101_0101_0101);

    let board = &b;
    let player = b.player;
    let allies = board[player];
    let opponents = board[!player];
    let occupancy = allies | opponents;
    let unoccupied = !occupancy;
    let pawns = board[Piece::Pawn] & allies;
    let rank8 = player.pawn_promote_rank();
    let not_rank8 = !rank8;
    let rank3 = match player {
        Color::White => Bitboard::new(0x0000_0000_00FF_0000),
        Color::Black => Bitboard::new(0x0000_FF00_0000_0000),
    };
    let direction = player.pawn_direction();
    let doubledir = 2 * direction;
    let unpinned = !board.pinned;
    let king_sq = board.king_sqs[player as usize];
    let king_file_mask = COL_A << king_sq.file();

    if M != QUIETS {
        // pawn captures

        const NOT_WESTMOST: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
        const NOT_EASTMOST: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);
        const RANK_1: Bitboard = Bitboard::new(0x0000_0000_0000_00FF);

        // only pawns which are unpinned or which move along the same diagonal
        // as the king can capture
        let king_rank_mask = RANK_1 << (king_sq.rank() << 3);
        let capturers = pawns & (unpinned | b.pinned & !(king_file_mask | king_rank_mask));

        let capture_mask = opponents & target;

        // prevent pawns from capturing by wraparound
        let west_capturers = capturers & NOT_WESTMOST;
        let east_capturers = capturers & NOT_EASTMOST;
        // hack because negative bitshift is UB
        let (west_targets, west_direction, east_targets, east_direction) = match player {
            Color::White => (
                west_capturers << 7 & capture_mask,
                Direction::NORTHWEST,
                east_capturers << 9 & capture_mask,
                Direction::NORTHEAST,
            ),
            Color::Black => (
                west_capturers >> 9 & capture_mask,
                Direction::SOUTHWEST,
                east_capturers >> 7 & capture_mask,
                Direction::SOUTHEAST,
            ),
        };

        // promotion captures
        for to_sq in east_targets & rank8 {
            let from_sq = to_sq - east_direction;
            if !b.pinned.contains(from_sq) || Square::aligned(king_sq, to_sq, from_sq) {
                for pt in Piece::PROMOTING {
                    let m = Move::promoting(from_sq, to_sq, pt);
                    moves.push((m, T::tag_move(m, b, cookie)));
                }
            }
        }

        for to_sq in west_targets & rank8 {
            let from_sq = to_sq - west_direction;
            if !b.pinned.contains(from_sq) || Square::aligned(king_sq, to_sq, from_sq) {
                for pt in Piece::PROMOTING {
                    let m = Move::promoting(from_sq, to_sq, pt);
                    moves.push((m, T::tag_move(m, b, cookie)));
                }
            }
        }

        // normal captures
        for to_sq in east_targets & not_rank8 {
            let from_sq = to_sq - east_direction;
            if !b.pinned.contains(from_sq) || Square::aligned(king_sq, to_sq, from_sq) {
                let m = Move::normal(from_sq, to_sq);
                moves.push((m, T::tag_move(m, b, cookie)));
            }
        }
        for to_sq in west_targets & not_rank8 {
            let from_sq = to_sq - west_direction;
            if !b.pinned.contains(from_sq) || Square::aligned(king_sq, to_sq, from_sq) {
                let m = Move::normal(from_sq, to_sq);
                moves.push((m, T::tag_move(m, b, cookie)));
            }
        }

        // en passant
        if let Some(ep_square) = board.en_passant_square {
            if target.contains(ep_square) {
                let king_sq = b.king_sqs[b.player as usize];
                let enemy = b[!b.player];
                let to_bb = Bitboard::from(ep_square);
                let capture_bb = match player {
                    Color::White => to_bb >> 8,
                    Color::Black => to_bb << 8,
                };
                let from_sqs = PAWN_ATTACKS[!player as usize][ep_square as usize] & pawns;
                for from_sq in from_sqs {
                    let new_occupancy =
                        b.occupancy() ^ Bitboard::from(from_sq) ^ capture_bb ^ to_bb;
                    if (MAGIC.rook_attacks(new_occupancy, king_sq)
                        & (b[Piece::Rook] | b[Piece::Queen])
                        & enemy)
                        .is_empty()
                        && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                            & (b[Piece::Bishop] | b[Piece::Queen])
                            & enemy)
                            .is_empty()
                    {
                        let m = Move::en_passant(from_sq, ep_square);
                        moves.push((m, T::tag_move(m, b, cookie)));
                    }
                }
            }
        }
    }

    if M != CAPTURES {
        // pawn forward moves

        // pawns which are not pinned or on the same file as the king can move
        let pushers = pawns & (unpinned | king_file_mask);
        let mut singles = match b.player {
            Color::White => pushers << 8,
            Color::Black => pushers >> 8,
        } & unoccupied;
        let double_candidates = singles & rank3;
        let doubles = match b.player {
            Color::White => double_candidates << 8,
            Color::Black => double_candidates >> 8,
        } & target
            & unoccupied;
        singles &= target;

        // promotion single-moves
        for to_sq in singles & rank8 {
            let from_sq = to_sq - direction;
            for pt in Piece::PROMOTING {
                let m = Move::promoting(from_sq, to_sq, pt);
                moves.push((m, T::tag_move(m, b, cookie)));
            }
        }

        // doublemoves
        for to_sq in doubles {
            let m = Move::normal(to_sq - doubledir, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }

        // normal single-moves
        for to_sq in singles & not_rank8 {
            let m = Move::normal(to_sq - direction, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }
}

/// Generate all the moves for a knight, bishop, rook, or queen which end
/// up on the target.
fn normal_piece_assistant<T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
    target: Bitboard,
) {
    let board = &b;
    let player = b.player;
    let allies = board[player];
    let legal_targets = !allies & target;
    let occupancy = allies | board[!player];
    let queens = board[Piece::Queen];
    let rook_movers = (board[Piece::Rook] | queens) & allies;
    let bishop_movers = (board[Piece::Bishop] | queens) & allies;
    let king_sq = board.king_sqs[player as usize];
    let unpinned = !board.pinned;

    for sq in board[Piece::Knight] & allies & unpinned {
        append_normal::<T>(
            sq,
            KNIGHT_MOVES[sq as usize] & legal_targets,
            b,
            cookie,
            moves,
        );
    }
    for sq in bishop_movers & board.pinned {
        append_normal::<T>(
            sq,
            MAGIC.bishop_attacks(occupancy, sq) & legal_targets & Bitboard::line(king_sq, sq),
            b,
            cookie,
            moves,
        );
    }
    for sq in bishop_movers & unpinned {
        append_normal::<T>(
            sq,
            MAGIC.bishop_attacks(occupancy, sq) & legal_targets,
            b,
            cookie,
            moves,
        );
    }
    for sq in rook_movers & board.pinned {
        append_normal::<T>(
            sq,
            MAGIC.rook_attacks(occupancy, sq) & legal_targets & Bitboard::line(king_sq, sq),
            b,
            cookie,
            moves,
        );
    }
    for sq in rook_movers & unpinned {
        append_normal::<T>(
            sq,
            MAGIC.rook_attacks(occupancy, sq) & legal_targets,
            b,
            cookie,
            moves,
        );
    }
}

#[inline(always)]
/// Append a number of normal moves, starting from `from_sq` and ending at each
/// square in `to_bb`, onto `moves`.
fn append_normal<T: Tagger>(
    from_sq: Square,
    to_bb: Bitboard,
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
) {
    for to_sq in to_bb {
        let m = Move::normal(from_sq, to_sq);
        moves.push((m, T::tag_move(m, b, cookie)));
    }
}

#[inline(always)]
/// Get the moves that a king could make in a position that are not castles,
/// and append them into the moves buffer.
fn king_move_non_castle<T: Tagger>(
    b: &Board,
    cookie: &T::Cookie,
    moves: &mut Vec<(Move, T::Tag)>,
    target: Bitboard,
) {
    let king_sq = b.king_sqs[b.player as usize];
    let allies = b[b.player];
    let to_bb = KING_MOVES[king_sq as usize] & !allies & target;
    let king_bb = b[Piece::King] & b[b.player];
    let old_occupancy = b.occupancy();
    for to_sq in to_bb {
        let new_occupancy = (old_occupancy ^ king_bb) | Bitboard::from(to_sq);
        if square_attackers_occupancy(b, to_sq, !b.player, new_occupancy).is_empty() {
            let m = Move::normal(king_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }
}

#[inline(always)]
/// Get the castling moves that the king could make in this position, and
/// append them onto the target vector.
///
/// Will not generate valid moves if the king is in check.
fn castles<T: Tagger>(b: &Board, cookie: &T::Cookie, moves: &mut Vec<(Move, T::Tag)>) {
    let player = b.player;
    let occ = b.occupancy();
    let king_sq = b.king_sqs[player as usize];

    // the squares the king must pass through to reach the castled position
    let kingside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x0000_0000_0000_0060),
        Color::Black => Bitboard::new(0x6000_0000_0000_0000),
    };

    let can_kingside_castle = b.castle_rights.is_kingside_castle_legal(player)
        && (occ & kingside_castle_passthrough_sqs).is_empty();

    if can_kingside_castle {
        // ignore start sq since we assume the king is not in check
        let passthrough_squares = match player {
            Color::White => [Square::F1, Square::G1],
            Color::Black => [Square::F8, Square::G8],
        };
        if !passthrough_squares
            .iter()
            .any(|&sq| is_square_attacked_by(b, sq, !player))
        {
            let m = Move::castling(king_sq, passthrough_squares[1]);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }

    // now, repeat the same process for queenside castling

    let queenside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x0000_0000_0000_000E),
        Color::Black => Bitboard::new(0x0E00_0000_0000_0000),
    };

    let can_queenside_castle = b.castle_rights.is_queenside_castle_legal(player)
        && (occ & queenside_castle_passthrough_sqs).is_empty();

    if can_queenside_castle {
        // ignore start sq since we assume the king is not in check
        let passthrough_squares = match player {
            Color::White => [Square::D1, Square::C1],
            Color::Black => [Square::D8, Square::C8],
        };
        if !passthrough_squares
            .iter()
            .any(|&sq| is_square_attacked_by(b, sq, !player))
        {
            let m = Move::castling(king_sq, passthrough_squares[1]);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }
}

/// Get the step attacks that could be made by moving in `dirs` from each point
/// in the square. Exclude the steps that travel more than `max_dist` (this
/// prevents overflow around the edges of the board).
fn create_step_attacks(dirs: &[Direction], max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard::EMPTY; 64];
    for (i, item) in attacks.iter_mut().enumerate() {
        #[allow(clippy::cast_possible_truncation)]
        for dir in dirs {
            let start_sq = Square::try_from(i as u8).unwrap();
            let target_sq = start_sq + *dir;
            if target_sq.chebyshev_to(start_sq) <= max_dist {
                item.insert(target_sq);
            }
        }
    }

    attacks
}

#[must_use]
#[allow(clippy::cast_precision_loss, clippy::similar_names)]
/// Perform a performance test on the move generator and print out facts. The
/// input fen is the FEN of the board to start from, and the depth is the depth
/// from which to generate moves.
///
/// # Panics
///
/// This function will panic if `fen` is not a legal board.
pub fn perft(fen: &str, depth: u8) -> u64 {
    let b = Board::from_fen(fen).unwrap();
    let tic = Instant::now();
    let num_nodes = perft_search::<true>(&b, depth);
    let toc = Instant::now();
    let time = toc - tic;
    let speed = (num_nodes as f64) / time.as_secs_f64();
    println!(
        "time {:.2} secs, num nodes {num_nodes}: {speed:.0} nodes/sec",
        time.as_secs_f64()
    );

    num_nodes
}

/// The core search algorithm for perft.
fn perft_search<const DIVIDE: bool>(b: &Board, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = get_moves::<ALL, NoTag>(b, &());
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut total = 0;
    let mut bcopy;
    for (m, _) in moves {
        bcopy = *b;
        bcopy.make_move(m);
        let perft_count = perft_search::<false>(&bcopy, depth - 1);
        if DIVIDE {
            println!("{}, {perft_count}", m);
        }
        total += perft_count;
    }

    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that we can play Qf3+, the critical move in the Fried Liver
    /// opening.
    fn best_queen_fried_liver() {
        let m = Move::normal(Square::D1, Square::F3);
        // the fried liver position, before Qf3+
        let b =
            Board::from_fen("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7").unwrap();
        let moves = get_moves::<ALL, NoTag>(&b, &());
        assert!(moves.contains(&(m, ())));
        for m in moves {
            assert!(is_legal(m.0, &b));
        }
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn pawn_capture_generated() {
        // check that exf5 is generated
        let b = Board::from_fen("rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2")
            .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        for (m, _) in get_moves::<ALL, NoTag>(&b, &()) {
            println!("{m}");
            assert!(is_legal(m, &b));
        }
        assert!(get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
        assert!(get_moves::<CAPTURES, NoTag>(&b, &()).contains(&(m, ())));
    }

    #[test]
    /// The pawn is checking the king. Is move enumeration correct?
    fn enumerate_pawn_checking_king() {
        let b =
            Board::from_fen("r1bq1b1r/ppp2kpp/2n5/3n4/2B5/8/PPP1pPPP/RN1Q1K1R w - - 0 10").unwrap();

        let moves = get_moves::<ALL, NoTag>(&b, &());

        for m2 in &moves {
            assert!(is_legal(m2.0, &b));
        }
    }

    #[test]
    /// Check that the king has exactly one move in this position.
    fn king_has_only_one_move() {
        let b = Board::from_fen("2k5/4R3/8/5K2/3R4/8/8/8 b - - 2 2").unwrap();
        assert!(has_moves(&b));
        assert!(get_moves::<ALL, NoTag>(&b, &()).len() == 1);
        assert!(is_legal(Move::normal(Square::C8, Square::B8), &b));
    }

    #[test]
    /// Test that queenside castling actually works.
    fn queenside_castle() {
        let b =
            Board::from_fen("r3kb1r/ppp1p1pp/2nq1n2/1B1p4/3P4/2N2Q2/PPP2PPP/R1B1K2R b KQkq - 0 8")
                .unwrap();
        let m = Move::castling(Square::E8, Square::C8);
        assert!(get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
        assert!(is_legal(m, &b));
    }

    #[test]
    /// Test that Black cannot castle because there is a knight in the way.
    fn no_queenside_castle_through_knight() {
        let b = Board::from_fen("rn2kbnr/ppp1pppp/3q4/3p4/6b1/8/PPPPPPPP/RNBQKBNR b KQkq - 5 4")
            .unwrap();
        let m = Move::castling(Square::E8, Square::C8);
        assert!(!get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));

        assert!(!is_legal(m, &b));
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn king_escape_without_capture() {
        let b = Board::from_fen("r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10")
            .unwrap();
        let moves = get_moves::<ALL, NoTag>(&b, &());
        let expected_moves = vec![
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in &moves {
            assert!(expected_moves.contains(&m.0));
            assert!(is_legal(m.0, &b));
        }
        for em in &expected_moves {
            assert!(moves.contains(&(*em, ())));
            assert!(is_legal(*em, &b));
        }
    }

    #[test]
    /// Test that Black can promote a piece (on e1).
    fn black_can_promote() {
        let b = Board::from_fen("8/8/5k2/3K4/8/8/4p3/8 b - - 0 1").unwrap();
        let moves = get_moves::<ALL, NoTag>(&b, &());
        for m in &moves {
            assert!(is_legal(m.0, &b));
        }
        assert!(moves.contains(&(Move::promoting(Square::E2, Square::E1, Piece::Queen), ())));
    }

    #[test]
    /// Test that pawns cannot "wrap around" the side of the board.
    fn no_wraparound() {
        let b = Board::from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBPPP3/q4N2/Pp4PP/R2Q1RK1 b kq - 0 1")
            .unwrap();

        let moves = get_moves::<ALL, NoTag>(&b, &());
        let m = Move::normal(Square::H7, Square::A7);
        assert!(!(moves.contains(&(m, ()))));
        assert!(!is_legal(m, &b));
    }

    #[test]
    /// Test that a move flagged as en passant is illegal, even if it is an
    /// otherwise normal capture.
    fn en_passant_illegal() {
        let b = Board::from_fen("r6r/3n1pk1/p4p2/3p4/2p1p1q1/1P2P1P1/P1PP1P1P/R1B1R1K1 b - - 0 25")
            .unwrap();
        let m = Move::en_passant(Square::C4, Square::B3);

        assert!(!is_legal(m, &b));
        assert!(!get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
        assert!(!get_moves::<CAPTURES, NoTag>(&b, &()).contains(&(m, ())));
    }

    #[test]
    /// Test that a pawn cannot en passant if doing so would put the king in
    /// check.
    fn en_passant_pinned() {
        let b = Board::from_fen("8/2p5/3p4/KPr5/2R1Pp1k/8/6P1/8 b - e3 0 2").unwrap();
        let moves = get_moves::<ALL, NoTag>(&b, &());
        let m = Move::en_passant(Square::F4, Square::E3);
        assert!(!moves.contains(&(m, ())));
        assert!(!is_legal(m, &b));
    }

    #[test]
    /// Test that a move must be tagged as en passant to be considered legal to
    /// escape check.
    fn en_passant_tagged() {
        let b = Board::from_fen("2B1kb2/pp2pp2/7p/1PpQP3/2nK4/8/P1r4R/R7 w - c6 0 27").unwrap();

        let m = Move::normal(Square::B5, Square::C6);
        assert!(!is_legal(m, &b));
        assert!(!get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
    }
    #[test]
    /// Test that a pinned piece cannot make a capture if it does not defend
    /// against the pin.
    fn pinned_knight_capture() {
        let b = Board::from_fen("r2q1b1r/ppp2kpp/2n5/3npb2/2B5/2N5/PPPP1PPP/R1BQ1RK1 b - - 3 8")
            .unwrap();
        let illegal_move = Move::normal(Square::D5, Square::C3);

        assert!(!get_moves::<ALL, NoTag>(&b, &()).contains(&(illegal_move, ())));
        assert!(!get_moves::<CAPTURES, NoTag>(&b, &()).contains(&(illegal_move, ())));
        assert!(!is_legal(illegal_move, &b));
    }

    #[test]
    /// Test that en passant moves are generated correctly.
    fn en_passant_generated() {
        // exf6 is en passant
        let b = Board::from_fen("rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3")
            .unwrap();

        let m = Move::en_passant(Square::E5, Square::F6);

        assert!(get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
        assert!(get_moves::<CAPTURES, NoTag>(&b, &()).contains(&(m, ())));
        assert!(is_legal(m, &b));
    }

    #[test]
    /// Test that a player can en passant out of check if it results in a
    /// checking pawn being captured.
    fn en_passant_out_of_check() {
        // bxc6 should be legal here
        let b = Board::from_fen("8/8/8/1Ppp3r/1KR2p1k/8/4P1P1/8 w - c6 0 3").unwrap();

        let m = Move::en_passant(Square::B5, Square::C6);

        assert!(get_moves::<ALL, NoTag>(&b, &()).contains(&(m, ())));
        assert!(is_legal(m, &b));
        assert!(has_moves(&b));
    }

    #[test]
    /// Test that the king can actually move (and `has_moves` reflects that
    /// fact).
    fn king_can_move() {
        let b = Board::from_fen("3k4/3R4/1R6/5K2/8/8/8/8 b - - 1 1").unwrap();

        assert!(!get_moves::<ALL, NoTag>(&b, &()).is_empty());
        assert!(!get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
        assert!(!get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        assert!(has_moves(&b));
    }

    #[test]
    /// Test that the start position of the game has moves.
    fn startpos_has_moves() {
        assert!(has_moves(&Board::default()));
    }

    /// Tests that mates are correct.
    mod mates {
        use super::*;

        #[test]
        /// Test that a ladder-mated position has no legal moves.
        fn ladder() {
            let b = Board::from_fen("1R1k4/R7/8/5K2/8/8/8/8 b - - 1 1").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }

        #[test]
        /// A position where if pawn pushes could be captures, there would not
        /// be a mate.
        fn cant_pawn_push() {
            let b = Board::from_fen("2r2r2/5R2/p2p2pk/3P2Q1/P4n2/7P/1P6/1K4R1 b - - 2 34").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }

        #[test]
        /// Test that a position where a rook is horizontal to the king is mate.
        fn horizontal_rook() {
            let b = Board::from_fen("r1b2k1R/3n1p2/p7/3P4/6Qp/2P3b1/6P1/4R2K b - - 0 32").unwrap();

            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
            assert!(!has_moves(&b));
        }

        #[test]
        /// A mate where the queen is adjacent to the king, and cuts off all
        /// escape.
        fn queen_defended() {
            let b = Board::from_fen("r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11")
                .unwrap();
            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
        }
    }

    mod draws {
        use super::*;

        #[test]
        /// Test that a king-versus-king endgame is a draw.
        fn draw_kk() {
            let b = Board::from_fen("K1k5/8/8/8/8/8/8/8 w - - 0 1").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }

        #[test]
        /// Test that a king-bishop versus king endgame is a draw.
        fn draw_kbk() {
            let b = Board::from_fen("KBk5/8/8/8/8/8/8/8 w - - 0 1").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }

        #[test]
        /// Test that a king-knight versus king endgame is a draw.
        fn draw_knk() {
            let b = Board::from_fen("KNk5/8/8/8/8/8/8/8 w - - 0 1").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }

        #[test]
        /// Test that a same-colored king-bishop versus king-bishop endgame is a draw.
        fn draw_kbkb() {
            let b = Board::from_fen("K1k5/8/8/8/3B4/8/3b4/8 w - - 0 1").unwrap();

            assert!(!has_moves(&b));
            assert!(get_moves::<ALL, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<CAPTURES, NoTag>(&b, &()).is_empty());
            assert!(get_moves::<QUIETS, NoTag>(&b, &()).is_empty());
        }
    }

    mod perft {
        use super::*;

        #[allow(clippy::cast_possible_truncation)]
        fn perft_assistant(fen: &str, node_counts: &[u64]) {
            for (i, num) in node_counts.iter().enumerate() {
                assert_eq!(*num, perft(fen, i as u8));
            }
        }

        #[test]
        /// Test the perft values for the board starting position.
        fn start_position() {
            perft_assistant(
                "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
                &[1, 20, 400, 8_902, 197_281, 4_865_609, 119_060_324],
            );
        }

        #[test]
        /// Test the perft values for the
        /// [Kiwipete](https://www.chessprogramming.org/Perft_Results#Position_2)
        /// position.
        fn kiwipete() {
            perft_assistant(
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                &[1, 48, 2039, 97_862, 4_085_603, 193_690_690],
            );
        }

        #[test]
        fn endgame() {
            // https://www.chessprogramming.org/Perft_Results#Position_3
            perft_assistant(
                "8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1",
                &[1, 14, 191, 2_812, 43_238, 674_624, 11_030_083, 178_633_661],
            );
        }

        #[test]
        /// Test the perft values for an unbalanced position. Uses results from
        /// [the CPW wiki](https://www.chessprogramming.org/Perft_Results#Position_4).
        fn unbalanced() {
            perft_assistant(
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
                &[1, 6, 264, 9_467, 422_333, 15_833_292],
            );
        }

        #[test]
        fn edwards() {
            // https://www.chessprogramming.org/Perft_Results#Position_5
            perft_assistant(
                "rnbq1k1r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQ - 1 8",
                &[1, 44, 1_486, 62_379, 2_103_487, 89_941_194],
            );
        }

        #[test]
        fn edwards2() {
            // https://www.chessprogramming.org/Perft_Results#Position_6
            perft_assistant(
                "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P1b1/P1NP1N2/1PP1QPPP/R4RK1 w - - 0 10",
                &[1, 46, 2_079, 89_890, 3_894_594, 164_075_551],
            );
        }
    }
}
