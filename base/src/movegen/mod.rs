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

#[cfg(test)]
mod tests;

use crate::{
    game::{NoTag, Tagger},
    MAGIC,
};

use super::{moves::Move, Bitboard, Board, Color, Direction, Piece, Square};

use std::{convert::TryFrom, mem::transmute, time::Instant};

/// A bitboard of all the squares a knight can move to if its position is
/// the index of the list.
pub const KNIGHT_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KNIGHT_STEPS, 2);

/// A bitboard of all the squares a king can move to if his position is the
/// index in the list.
pub const KING_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KING_STEPS, 1);

/// A bitboard of all the squares which a pawn on the given square can
/// attack. The first index is for White's pawn attacks, the second is for
/// Black's.
pub const PAWN_ATTACKS: [[Bitboard; 64]; 2] = [
    create_step_attacks(&[Direction::NORTHEAST, Direction::NORTHWEST], 1),
    create_step_attacks(&[Direction::SOUTHEAST, Direction::SOUTHWEST], 1),
];

/// Get the step attacks that could be made by moving in `dirs` from each point
/// in the square. Exclude the steps that travel more than `max_dist` (this
/// prevents overflow around the edges of the board).
const fn create_step_attacks(dirs: &[Direction], max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard::EMPTY; 64];
    let mut i = 0;
    #[allow(clippy::cast_possible_truncation)]
    while i < attacks.len() {
        // SAFETY: we know that `attacks` is 64 elements long, which is the number of
        // `Square`s, so we will not create an illegal variant.
        let sq: Square = unsafe { transmute(i as u8) };
        let mut j = 0;
        #[allow(clippy::cast_sign_loss)]
        while j < dirs.len() {
            let dir = dirs[j];
            let target_sq_disc = sq as i8 + dir.0;
            if target_sq_disc < 0 || 64 <= target_sq_disc {
                // square is out of bounds
                j += 1;
                continue;
            }
            let target_sq: Square = unsafe { transmute((sq as i8 + dir.0) as u8) };
            if target_sq.chebyshev_to(sq) <= max_dist {
                attacks[i] = attacks[i].with_square(target_sq);
            }
            j += 1;
        }
        // sanity check that we added only two attacks
        debug_assert!(attacks[i].len() as usize <= dirs.len());
        i += 1;
    }

    attacks
}

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
/// `is_legal` will make no regard to whether a position is drawn by repetition,
/// 50-move-rule, or insufficient material.
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
/// `get_moves()` will make no regard to whether the position is drawn by
/// repetition, 50-move-rule, or by insufficient material.
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
    let king_file_mask = Bitboard::vertical(king_sq);
    if M != QUIETS {
        // pawn captures

        const NOT_WESTMOST: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
        const NOT_EASTMOST: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

        // Pin masks for capture movement
        let (west_pin_diag, east_pin_diag) = match b.player {
            Color::White => (
                Bitboard::anti_diagonal(king_sq),
                Bitboard::diagonal(king_sq),
            ),
            Color::Black => (
                Bitboard::diagonal(king_sq),
                Bitboard::anti_diagonal(king_sq),
            ),
        };

        let capture_mask = opponents & target;

        // prevent pawns from capturing by wraparound
        let west_capturers = pawns & NOT_WESTMOST & (unpinned | west_pin_diag);
        let east_capturers = pawns & NOT_EASTMOST & (unpinned | east_pin_diag);
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
            for pt in Piece::PROMOTING {
                let m = Move::promoting(from_sq, to_sq, pt);
                moves.push((m, T::tag_move(m, b, cookie)));
            }
        }

        for to_sq in west_targets & rank8 {
            let from_sq = to_sq - west_direction;
            for pt in Piece::PROMOTING {
                let m = Move::promoting(from_sq, to_sq, pt);
                moves.push((m, T::tag_move(m, b, cookie)));
            }
        }

        // normal captures
        for to_sq in east_targets & not_rank8 {
            let from_sq = to_sq - east_direction;
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
        for to_sq in west_targets & not_rank8 {
            let from_sq = to_sq - west_direction;
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
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

    // only unpinned knights can move
    for from_sq in board[Piece::Knight] & allies & unpinned {
        for to_sq in KNIGHT_MOVES[from_sq as usize] & legal_targets {
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }

    // pinned bishops and queens
    for from_sq in bishop_movers & board.pinned {
        for to_sq in MAGIC.bishop_attacks(occupancy, from_sq)
            & legal_targets
            & Bitboard::line(king_sq, from_sq)
        {
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }

    // unpinned bishops and queens
    for from_sq in bishop_movers & unpinned {
        for to_sq in MAGIC.bishop_attacks(occupancy, from_sq) & legal_targets {
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }

    // pinned rooks and queens
    for from_sq in rook_movers & board.pinned {
        for to_sq in MAGIC.rook_attacks(occupancy, from_sq)
            & legal_targets
            & Bitboard::line(king_sq, from_sq)
        {
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
    }

    // unpinned rooks and queens
    for from_sq in rook_movers & unpinned {
        for to_sq in MAGIC.rook_attacks(occupancy, from_sq) & legal_targets {
            let m = Move::normal(from_sq, to_sq);
            moves.push((m, T::tag_move(m, b, cookie)));
        }
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
