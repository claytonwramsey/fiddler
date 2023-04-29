/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

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

pub(crate) mod magic;
#[cfg(test)]
mod tests;

use std::{convert::TryFrom, mem::transmute};

use once_cell::sync::Lazy;

use self::magic::AttacksTable;

use super::{bitboard::Bitboard, game::Game, Color, Direction, Move, Piece, Square};

/// A master copy of a magic move-generation table.
///
/// This table can be used to generate moves for rooks and bishops in constant time.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::MAGIC, Bitboard, Square};
///
/// let rook_attacks_e3 = MAGIC.rook_attacks(Bitboard::EMPTY, Square::E3);
/// assert_eq!(rook_attacks_e3, Bitboard::hv(Square::E3));
/// ```
pub static MAGIC: Lazy<AttacksTable> = Lazy::new(AttacksTable::load);

/// A lookup table for the legal squares a knight to move to from a given square.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::KNIGHT_MOVES, Bitboard, Square};
///
/// let mut knight_attacks_a1 = Bitboard::EMPTY
///     .with_square(Square::C2)
///     .with_square(Square::B3);
///
/// assert_eq!(KNIGHT_MOVES[Square::A1 as usize], knight_attacks_a1);
/// ```
// bob seger
pub const KNIGHT_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KNIGHT_STEPS, 2);

/// A lookup table for the legal squares a king can move to from a given square.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::KING_MOVES, Bitboard, Square};
///
/// let mut king_attacks_a1 = Bitboard::EMPTY
///     .with_square(Square::A2)
///     .with_square(Square::B1)
///     .with_square(Square::B2);
///
/// assert_eq!(KING_MOVES[Square::A1 as usize], king_attacks_a1);
/// ```
pub const KING_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KING_STEPS, 1);

/// A lookup table for the legal squares a pawn can attack if it is on a given square and of a given
/// color.
///
/// `PAWN_ATTACKS[0]` is a lookup table for the squares that a white pawn can attack, while
/// `PAWN_ATTACKS[1]` is a lookup table for the squares that a black pawn can attack.
///
/// This table does not include squares that pawns can move to by pushing forward.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::PAWN_ATTACKS, Bitboard, Color, Square};
///
/// let mut attacked_squares = Bitboard::EMPTY
///     .with_square(Square::A4)
///     .with_square(Square::C4);
///
/// // A white pawn on B3 can attack squares A4 and C4.
/// assert_eq!(
///     PAWN_ATTACKS[Color::White as usize][Square::B3 as usize],
///     attacked_squares
/// );
/// // A black pawn on B5 can attack squares A4 and C4.
/// assert_eq!(
///     PAWN_ATTACKS[Color::Black as usize][Square::B5 as usize],
///     attacked_squares
/// );
/// ```
pub const PAWN_ATTACKS: [[Bitboard; 64]; 2] = [
    create_step_attacks(&[Direction::NORTHEAST, Direction::NORTHWEST], 1),
    create_step_attacks(&[Direction::SOUTHEAST, Direction::SOUTHWEST], 1),
];

/// Get the step attacks that could be made by moving in `dirs` from each point in the square.
///
/// Exclude the steps that travel more than `max_dist` (this prevents overflow around the edges of
/// the board).
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
                // square is out of bounds. prevent UB in transmutation
                j += 1;
                continue;
            }
            let target_sq: Square = unsafe { transmute(target_sq_disc as u8) };
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

#[derive(PartialEq, Eq, Debug)]
/// The possible modes for move generation.
/// They are inteded for use as const-generic parameters for [`get_moves()`].
pub enum GenMode {
    /// The mode identifier for [`get_moves()`] to generate all legal moves.
    All,
    /// The mode identifier for [`get_moves()`] to generate captures only.
    Captures,
    /// The mode identifier for [`get_moves()`] to generate non-captures only.
    Quiets,
}

#[must_use]
#[allow(clippy::missing_panics_doc)]
#[allow(clippy::too_many_lines)]
/// Determine whether any given move is legal, given a position in which it could be played.
///
/// Requires that the move must have been legal on *some* board, but not necessarily the given one.
/// `is_legal` will make no regard to whether a position is drawn by repetition, 50-move-rule, or
/// insufficient material.
///
/// # Examples
///
/// ```
/// use fiddler::base::{game::Game, movegen::is_legal, Move, Square};
///
/// let game = Game::new();
/// assert!(is_legal(Move::normal(Square::E2, Square::E4), &game));
/// assert!(!is_legal(Move::normal(Square::E2, Square::D4), &game));
/// ```
pub fn is_legal(m: Move, g: &Game) -> bool {
    let meta = g.meta();
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let from_bb = Bitboard::from(from_sq);
    let to_bb = Bitboard::from(to_sq);
    let player = meta.player;
    let allies = g[player];
    let enemies = g[!player];
    let occupancy = allies | enemies;
    if allies.contains(to_sq) {
        // cannot move to square occupied by our piece
        return false;
    }
    if !allies.contains(from_sq) {
        return false;
    }

    let Some((pt, _)) = g[from_sq] else {
        return false;
    };

    if pt == Piece::King {
        // king has special rules for its behavior
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
            let mut valid = false;
            castles(g, &mut |m2| valid |= m == m2);
            return valid;
        }

        if !KING_MOVES[from_sq as usize].contains(to_sq) {
            return false;
        }

        // normal king moves can't step into check
        let new_occupancy = (g.occupancy() ^ from_bb) | to_bb;
        return square_attackers_occupancy(g, to_sq, !meta.player, new_occupancy).is_empty();
    }

    // normal piece

    if meta.checkers.more_than_one() {
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
    if is_ep && (pt != Piece::Pawn || meta.en_passant_square != Some(to_sq)) {
        // only pawns can en passant
        // also, en passant must target the en passant square
        return false;
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
                || (is_ep && meta.en_passant_square == Some(to_sq))
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
            (MAGIC.bishop_attacks(occupancy, from_sq) | MAGIC.rook_attacks(occupancy, from_sq))
                .contains(to_sq)
        }
        Piece::King => unreachable!(),
    } {
        return false;
    };

    // check that the move is not a self check
    if !meta.checkers.is_empty() {
        // we already handled the two-checker case, so there is only one
        // checker
        let checker_sq = Square::try_from(meta.checkers).unwrap();
        let player_idx = meta.player as usize;
        let mut targets =
            Bitboard::between(meta.king_sqs[player_idx], checker_sq) | Bitboard::from(checker_sq);

        if let Some(ep_sq) = meta.en_passant_square {
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
        let king_sq = meta.king_sqs[meta.player as usize];
        let capture_bb = match player {
            Color::White => to_bb >> 8,
            Color::Black => to_bb << 8,
        };

        let new_occupancy = g.occupancy() ^ from_bb ^ capture_bb ^ to_bb;

        return (MAGIC.rook_attacks(new_occupancy, king_sq)
            & (g[Piece::Rook] | g[Piece::Queen])
            & enemies)
            .is_empty()
            && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                & (g[Piece::Bishop] | g[Piece::Queen])
                & enemies)
                .is_empty();
    }

    !meta.pinned.contains(from_sq)
        || Square::aligned(from_sq, to_sq, meta.king_sqs[player as usize])
}

/// Get the legal moves in a board.
///
/// `M` is the generation mode of move generation: it specifies which subset of all legal moves to
/// generate.
/// There are currently 3 legal generation modes:
///
/// * `GenMode::All` will generate all legal moves.
/// * `GenMode::Captures` will generate all captures, including en passant.
/// * `GenMode::Quiets` will generate all quiet (i.e. non-capture) moves.
///
/// `callback` is the callback function which will handle moves as they are created.
///
/// `get_moves()` will make no regard to whether the position is drawn by
/// repetition, 50-move-rule, or by insufficient material.
/// No guarantees are made as to the ordering of moves generated.
///
/// # Examples
///
/// Generate all legal moves:
/// ```
/// use fiddler::base::{
///     game::Game,
///     movegen::{get_moves, is_legal, GenMode},
/// };
///
/// let g = Game::new();
/// get_moves::<{ GenMode::All }>(&g, |m| {
///     assert!(is_legal(m, &g));
/// });
/// ```
///
/// Generate captures:
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>>{
/// use fiddler::base::{
///     game::Game,
///     movegen::{get_moves, is_legal, GenMode},
///     Move, Square,
/// };
///
/// // Scandinavian defense. The only legal capture is exd5.
/// let g = Game::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")?;
///
/// get_moves::<{ GenMode::Captures }>(&g, |m| {
///     assert_eq!(m, Move::normal(Square::E4, Square::D5));
/// });
/// # Ok(())
/// # }
/// ```
///
/// Generate quiet moves:
///
/// ```
/// use fiddler::base::{
///     game::Game,
///     movegen::{get_moves, is_legal, GenMode},
/// };
///
/// let g = Game::new();
/// get_moves::<{ GenMode::Quiets }>(&g, |m| {
///     assert!(is_legal(m, &g));
///     assert!(!g.is_move_capture(m));
/// });
/// ```
pub fn get_moves<const M: GenMode>(g: &Game, callback: impl FnMut(Move)) {
    if g.meta().checkers.is_empty() {
        non_evasions::<M>(g, callback);
    } else {
        evasions::<M>(g, callback);
    };
}

#[must_use]
/// A convenient helper function to get a vector of legal moves on the board.
///
/// Working with [`get_moves`] is not particularly ergonomic under many circumstances, especially
/// when you don't care about performance.
/// This function simply creates a list of moves created from `get_moves`.
///
/// `M` is the generation mode for the set of moves to create.
/// For more details, refer to `get_moves`.
///
/// # Examples
///
/// ```
/// use fiddler::base::{
///     game::Game,
///     movegen::{get_moves, is_legal, make_move_vec, GenMode},
/// };
///
/// let g = Game::default();
/// let moves = make_move_vec::<{ GenMode::All }>(&g);
/// for m in moves {
///     assert!(is_legal(m, &g))
/// }
/// ```
pub fn make_move_vec<const M: GenMode>(g: &Game) -> Vec<Move> {
    // initialize capacity to make vector push faster
    let mut moves = Vec::with_capacity(match M {
        GenMode::All => 40,
        GenMode::Captures => 30,
        GenMode::Quiets => 10,
    });
    get_moves::<M>(g, |m| moves.push(m));
    moves
}

#[must_use]
/// Determine whether the player to move have any legal moves in this position.
///
/// Returns `true` if there are any legal moves, and `false` if there are none.
/// Requires that the board is legal (i.e. has one of each king) to be correct.
/// This does not take into account whether the position is drawn by insufficient material,
/// repetion, or the 50-move rule.
///
/// # Examples
///
/// ```
/// use fiddler::base::{game::Game, movegen::has_moves};
///
/// let g = Game::new();
/// assert!(has_moves(&g));
/// ```
pub fn has_moves(g: &Game) -> bool {
    let meta = g.meta();

    let player = meta.player;
    let allies = g[player];
    let enemies = g[!player];
    let opponent = !player;
    let occupancy = allies | enemies;
    let mut legal_targets = !allies;
    let king_sq = meta.king_sqs[player as usize];
    // king does not have to block its checks
    let king_to_sqs = KING_MOVES[king_sq as usize] & legal_targets;
    let unpinned = !meta.pinned;
    let queens = g[Piece::Queen];

    if meta.checkers.more_than_one() {
        // in double check, only consider king moves
        let new_occupancy = occupancy ^ Bitboard::from(king_sq);
        for to_sq in king_to_sqs {
            if square_attackers_occupancy(g, to_sq, opponent, new_occupancy).is_empty() {
                return true;
            }
        }

        return false;
    }

    // king is either single-checked or not at all
    if !meta.checkers.is_empty() {
        // SAFETY: We checked that the set of checkers is nonzero.
        let checker_sq = unsafe { Square::unsafe_from(meta.checkers) };
        // Restrict move search to things that can block the check
        legal_targets &= Bitboard::between(king_sq, checker_sq) | meta.checkers;
    }
    // save the (expensive) king move generation/validation for later

    // pinned knights can never move, but an unpinned knight does whatever it
    // wants
    for sq in g[Piece::Knight] & allies & unpinned {
        if !(KNIGHT_MOVES[sq as usize] & legal_targets).is_empty() {
            return true;
        }
    }

    // unpinned bishops/diagonal queens
    let bishop_movers = (g[Piece::Bishop] | queens) & allies;
    for sq in bishop_movers & unpinned {
        if !(MAGIC.bishop_attacks(occupancy, sq) & legal_targets).is_empty() {
            return true;
        }
    }

    // pinned bishops/diagonal queens
    let king_diags = Bitboard::diagonal(king_sq);
    for sq in bishop_movers & meta.pinned & king_diags {
        if !(MAGIC.bishop_attacks(occupancy, sq) & legal_targets & king_diags).is_empty() {
            return true;
        }
    }

    let rook_movers = (g[Piece::Rook] | queens) & allies;
    // unpinned rooks/horizontal queens
    for sq in rook_movers & unpinned {
        if !(MAGIC.rook_attacks(occupancy, sq) & legal_targets).is_empty() {
            return true;
        }
    }

    // pinned rooks/horizontal queens
    let king_hv = Bitboard::hv(king_sq);
    for sq in rook_movers & meta.pinned & king_hv {
        if !(MAGIC.rook_attacks(occupancy, sq) & legal_targets & king_hv).is_empty() {
            return true;
        }
    }

    // "normal" pawn moves
    let our_pawns = g[Piece::Pawn] & allies;
    for sq in our_pawns {
        let singlemove_sq = sq + player.pawn_direction();
        let mut to_bb = Bitboard::from(singlemove_sq);
        if !occupancy.contains(singlemove_sq) && player.pawn_start_rank().contains(sq) {
            to_bb.insert(singlemove_sq + player.pawn_direction());
        }

        to_bb &= !occupancy;
        to_bb |= PAWN_ATTACKS[player as usize][sq as usize] & enemies;
        to_bb &= legal_targets;

        if meta.pinned.contains(sq) {
            to_bb &= Bitboard::line(king_sq, sq);
        }

        if !to_bb.is_empty() {
            return true;
        }
    }

    // en passant might save us
    if let Some(ep_sq) = meta.en_passant_square {
        let ep_less_occupancy = occupancy ^ Bitboard::from(ep_sq);
        for sq in our_pawns {
            if PAWN_ATTACKS[player as usize][sq as usize].contains(ep_sq) {
                let new_occupancy = ep_less_occupancy ^ Bitboard::from(sq);
                if (MAGIC.rook_attacks(new_occupancy, king_sq)
                    & (g[Piece::Rook] | g[Piece::Queen])
                    & enemies)
                    .is_empty()
                    && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                        & (g[Piece::Bishop] | g[Piece::Queen])
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
        if square_attackers_occupancy(g, to_sq, opponent, new_occupancy).is_empty() {
            return true;
        }
    }
    // if a king cannot do any "normal" moves, it also cannot castle, so we don't need to check for
    // those moves

    false
}

#[must_use]
/// Determine whether a square is attacked by the pieces of a given color in a position.
/// Squares which are threatened by only non-capture moves (i.e. pawn-pushes) will not qualify as
/// attacked.
///
/// # Examples
///
/// ```
/// use fiddler::base::{game::Game, movegen::is_square_attacked_by, Color, Square};
///
/// let g = Game::new();
/// assert!(is_square_attacked_by(&g, Square::E2, Color::White));
/// assert!(!is_square_attacked_by(&g, Square::E4, Color::White));
/// ```
pub fn is_square_attacked_by(game: &Game, sq: Square, color: Color) -> bool {
    !square_attackers(game, sq, color).is_empty()
}

/// Enumerate the legal moves a player of the given color would be able to make if it were their
/// turn to move, assuming the player's king is not in check.
///
/// Requires that the player to move's king is not in check.
fn non_evasions<const M: GenMode>(g: &Game, mut callback: impl FnMut(Move)) {
    let meta = g.meta();
    let target_sqs = match M {
        GenMode::All => !g[meta.player],
        GenMode::Captures => g[!meta.player],
        GenMode::Quiets => !g.occupancy(),
    };

    let mut pawn_targets = target_sqs;
    if M != GenMode::Quiets {
        if let Some(ep_sq) = meta.en_passant_square {
            pawn_targets.insert(ep_sq);
        }
    }
    pawn_assistant::<M>(g, &mut callback, pawn_targets);

    normal_piece_assistant(g, &mut callback, target_sqs);

    // generate king moves
    if M != GenMode::Captures {
        castles(g, &mut callback);
    }
    king_move_non_castle(g, &mut callback, target_sqs);
}

/// Enumerate the legal moves a player of the given color would be able to make if it were their
/// turn to move, assuming the player's king is in check.
///
/// Requires that the player to move's king is in check.
fn evasions<const M: GenMode>(g: &Game, mut callback: impl FnMut(Move)) {
    let meta = g.meta();
    let player = meta.player;
    let king_sq = meta.king_sqs[player as usize];

    // only look at non-king moves if we are not in double check
    if meta.checkers.has_single_bit() {
        // SAFETY: We checked that the set of checkers is nonzero.
        let checker_sq = unsafe { Square::unsafe_from(meta.checkers) };
        // Look for blocks or captures
        let mut target_sqs = !g[player] & Bitboard::between(king_sq, checker_sq) | meta.checkers;
        match M {
            GenMode::All => (),
            GenMode::Captures => target_sqs &= g[!player],
            GenMode::Quiets => target_sqs &= !g[!player],
        }

        let mut pawn_targets = target_sqs;
        if M != GenMode::Quiets {
            if let Some(ep_sq) = meta.en_passant_square {
                // can en passant save us from check?
                let ep_attacker_sq = ep_sq - player.pawn_direction();
                if meta.checkers.contains(ep_attacker_sq) {
                    pawn_targets.insert(ep_sq);
                }
            }
        }

        pawn_assistant::<M>(g, &mut callback, pawn_targets);
        normal_piece_assistant(g, &mut callback, target_sqs);
    }

    let king_targets = match M {
        GenMode::All => !g[player],
        GenMode::Captures => g[!player],
        GenMode::Quiets => !g.occupancy(),
    };
    king_move_non_castle(g, &mut callback, king_targets);
}

#[must_use]
/// Get the attackers of a given color on a square as a `Bitboard` representing the squares of the
/// attackers.
///
/// # Examples
///
/// ```
/// use fiddler::base::{game::Game, movegen::square_attackers, Bitboard, Color, Square};
///
/// let g = Game::new();
/// let attackers = Bitboard::EMPTY
///     .with_square(Square::E1)
///     .with_square(Square::D1)
///     .with_square(Square::F1)
///     .with_square(Square::G1);
///
/// assert_eq!(square_attackers(&g, Square::E2, Color::White), attackers);
/// ```
pub fn square_attackers(game: &Game, sq: Square, color: Color) -> Bitboard {
    square_attackers_occupancy(game, sq, color, game.occupancy())
}

/// Same functionality as `square_attackers`, but uses the provided `occupancy` bitboard (as
/// opposed to the board's occupancy.)
fn square_attackers_occupancy(
    game: &Game,
    sq: Square,
    color: Color,
    occupancy: Bitboard,
) -> Bitboard {
    let mut attackers = Bitboard::EMPTY;
    let color_bb = game[color];
    // Check for pawn attacks
    let pawn_vision = PAWN_ATTACKS[!color as usize][sq as usize];
    attackers |= pawn_vision & game[Piece::Pawn];

    // Check for knight attacks
    let knight_vision = KNIGHT_MOVES[sq as usize];
    attackers |= knight_vision & game[Piece::Knight];

    let queens_bb = game[Piece::Queen];

    // Check for rook/horizontal queen attacks
    let rook_vision = MAGIC.rook_attacks(occupancy, sq);
    attackers |= rook_vision & (queens_bb | game[Piece::Rook]);

    // Check for bishop/diagonal queen attacks
    let bishop_vision = MAGIC.bishop_attacks(occupancy, sq);
    attackers |= bishop_vision & (queens_bb | game[Piece::Bishop]);

    // Check for king attacks
    let king_vision = KING_MOVES[sq as usize];
    attackers |= king_vision & game[Piece::King];

    attackers & color_bb
}

#[allow(clippy::too_many_lines)]
/// Generate the moves all pawns can make and populate `moves` with those moves.
/// Only moves which result in a pawn landing on `target` will be generated.
///
/// Moves which capture allies will also be generated.
/// To prevent this, ensure all squares containing allies are excluded from `target`.
fn pawn_assistant<const M: GenMode>(g: &Game, callback: &mut impl FnMut(Move), target: Bitboard) {
    let meta = g.meta();
    let player = meta.player;
    let allies = g[player];
    let opponents = g[!player];
    let occupancy = allies | opponents;
    let unoccupied = !occupancy;
    let pawns = g[Piece::Pawn] & allies;
    let rank8 = player.pawn_promote_rank();
    let not_rank8 = !rank8;
    let rank3 = match player {
        Color::White => Bitboard::new(0x0000_0000_00FF_0000),
        Color::Black => Bitboard::new(0x0000_FF00_0000_0000),
    };
    let direction = player.pawn_direction();
    let doubledir = 2 * direction;
    let unpinned = !meta.pinned;
    let king_sq = meta.king_sqs[player as usize];
    let king_file_mask = Bitboard::vertical(king_sq);
    if M != GenMode::Quiets {
        // pawn captures

        const NOT_WESTMOST: Bitboard = Bitboard::new(0xFEFE_FEFE_FEFE_FEFE);
        const NOT_EASTMOST: Bitboard = Bitboard::new(0x7F7F_7F7F_7F7F_7F7F);

        // Pin masks for capture movement
        let (west_pin_diag, east_pin_diag) = match player {
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
                callback(m);
            }
        }

        for to_sq in west_targets & rank8 {
            let from_sq = to_sq - west_direction;
            for pt in Piece::PROMOTING {
                let m = Move::promoting(from_sq, to_sq, pt);
                callback(m);
            }
        }

        // normal captures
        for to_sq in east_targets & not_rank8 {
            let from_sq = to_sq - east_direction;
            let m = Move::normal(from_sq, to_sq);
            callback(m);
        }
        for to_sq in west_targets & not_rank8 {
            let from_sq = to_sq - west_direction;
            let m = Move::normal(from_sq, to_sq);
            callback(m);
        }

        // en passant
        if let Some(ep_square) = meta.en_passant_square {
            if target.contains(ep_square) {
                let king_sq = meta.king_sqs[player as usize];
                let enemy = g[!player];
                let to_bb = Bitboard::from(ep_square);
                let capture_bb = match player {
                    Color::White => to_bb >> 8,
                    Color::Black => to_bb << 8,
                };
                let from_sqs = PAWN_ATTACKS[!player as usize][ep_square as usize] & pawns;
                for from_sq in from_sqs {
                    let new_occupancy =
                        g.occupancy() ^ Bitboard::from(from_sq) ^ capture_bb ^ to_bb;
                    if (MAGIC.rook_attacks(new_occupancy, king_sq)
                        & (g[Piece::Rook] | g[Piece::Queen])
                        & enemy)
                        .is_empty()
                        && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                            & (g[Piece::Bishop] | g[Piece::Queen])
                            & enemy)
                            .is_empty()
                    {
                        let m = Move::en_passant(from_sq, ep_square);
                        callback(m);
                    }
                }
            }
        }
    }

    if M != GenMode::Captures {
        // pawn forward moves

        // pawns which are not pinned or on the same file as the king can move
        let pushers = pawns & (unpinned | king_file_mask);
        let mut singles = match player {
            Color::White => pushers << 8,
            Color::Black => pushers >> 8,
        } & unoccupied;
        let double_candidates = singles & rank3;
        let doubles = match player {
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
                callback(m);
            }
        }

        // doublemoves
        for to_sq in doubles {
            let m = Move::normal(to_sq - doubledir, to_sq);
            callback(m);
        }

        // normal single-moves
        for to_sq in singles & not_rank8 {
            let m = Move::normal(to_sq - direction, to_sq);
            callback(m);
        }
    }
}

/// Generate all the moves for a knight, bishop, rook, or queen which end up on the target.
///
/// Moves which capture allies will also be generated.
/// To prevent this, ensure all squares containing allies are excluded from `target`.
fn normal_piece_assistant(g: &Game, callback: &mut impl FnMut(Move), target: Bitboard) {
    let meta = g.meta();

    let player = meta.player;
    let allies = g[player];
    let occupancy = allies | g[!player];
    let queens = g[Piece::Queen];
    let rook_movers = (g[Piece::Rook] | queens) & allies;
    let bishop_movers = (g[Piece::Bishop] | queens) & allies;
    let king_sq = meta.king_sqs[player as usize];
    let unpinned = !meta.pinned;
    let king_hv = Bitboard::hv(king_sq);
    let king_diags = Bitboard::diags(king_sq);

    // only unpinned knights can move
    for from_sq in g[Piece::Knight] & allies & unpinned {
        for to_sq in KNIGHT_MOVES[from_sq as usize] & target {
            callback(Move::normal(from_sq, to_sq));
        }
    }

    // pinned bishops and queens
    for from_sq in bishop_movers & meta.pinned & king_diags {
        for to_sq in MAGIC.bishop_attacks(occupancy, from_sq) & target & king_diags {
            callback(Move::normal(from_sq, to_sq));
        }
    }

    // unpinned bishops and queens
    for from_sq in bishop_movers & unpinned {
        for to_sq in MAGIC.bishop_attacks(occupancy, from_sq) & target {
            callback(Move::normal(from_sq, to_sq));
        }
    }

    // pinned rooks and queens
    for from_sq in rook_movers & meta.pinned & king_hv {
        for to_sq in MAGIC.rook_attacks(occupancy, from_sq) & target & king_hv {
            callback(Move::normal(from_sq, to_sq));
        }
    }

    // unpinned rooks and queens
    for from_sq in rook_movers & unpinned {
        for to_sq in MAGIC.rook_attacks(occupancy, from_sq) & target {
            callback(Move::normal(from_sq, to_sq));
        }
    }
}

/// Get the moves that a king could make in a position that are not castles.
///
/// Only moves which result in a king landing on a square contained by `target` will be generated.
/// If `target` contains a square occupied by an ally, it can generate a move with the ally as the
/// target square.
fn king_move_non_castle(g: &Game, callback: &mut impl FnMut(Move), target: Bitboard) {
    let meta = g.meta();
    let king_sq = meta.king_sqs[meta.player as usize];
    let allies = g[meta.player];
    let to_bb = KING_MOVES[king_sq as usize] & !allies & target;
    let king_bb = g[Piece::King] & g[meta.player];
    let old_occupancy = g.occupancy();
    for to_sq in to_bb {
        let new_occupancy = (old_occupancy ^ king_bb) | Bitboard::from(to_sq);
        if square_attackers_occupancy(g, to_sq, !meta.player, new_occupancy).is_empty() {
            callback(Move::normal(king_sq, to_sq));
        }
    }
}

/// Get the castling moves that the king could make in this position and hand them off to
/// `callback`.
///
/// Will not generate valid moves if the king is in check.
fn castles(g: &Game, callback: &mut impl FnMut(Move)) {
    let meta = g.meta();

    let player = meta.player;
    let occ = g.occupancy();
    let king_sq = meta.king_sqs[player as usize];

    // the squares the king must pass through to reach the castled position
    let kingside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x0000_0000_0000_0060),
        Color::Black => Bitboard::new(0x6000_0000_0000_0000),
    };

    let can_kingside_castle =
        meta.castle_rights.kingside(player) && (occ & kingside_castle_passthrough_sqs).is_empty();

    if can_kingside_castle {
        // ignore start sq since we assume the king is not in check
        let passthrough_squares = match player {
            Color::White => [Square::F1, Square::G1],
            Color::Black => [Square::F8, Square::G8],
        };
        if !passthrough_squares
            .iter()
            .any(|&sq| is_square_attacked_by(g, sq, !player))
        {
            callback(Move::castling(king_sq, passthrough_squares[1]));
        }
    }

    // now, repeat the same process for queenside castling

    let queenside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x0000_0000_0000_000E),
        Color::Black => Bitboard::new(0x0E00_0000_0000_0000),
    };

    let can_queenside_castle =
        meta.castle_rights.queenside(player) && (occ & queenside_castle_passthrough_sqs).is_empty();

    if can_queenside_castle {
        // ignore start sq since we assume the king is not in check
        let passthrough_squares = match player {
            Color::White => [Square::D1, Square::C1],
            Color::Black => [Square::D8, Square::C8],
        };
        if !passthrough_squares
            .iter()
            .any(|&sq| is_square_attacked_by(g, sq, !player))
        {
            callback(Move::castling(king_sq, passthrough_squares[1]));
        }
    }
}
