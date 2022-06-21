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

use super::{
    magic::MagicTable, moves::Move, Bitboard, Board, Color, Direction, Piece, Position, Square,
};

use lazy_static::lazy_static;
use std::convert::TryFrom;

// Construct common lookup tables for use in move generation.
lazy_static! {
    /// A master copy of the main magic table. Used for generating bishop,
    /// rook, and queen moves.
    static ref MAGIC: MagicTable = MagicTable::load();

    /// A lookup table for the squares on a line between any two squares,
    /// either down a row like a rook or diagonal like a bishop.
    /// `lines[A1][B2]` would return a bitboard with active squares down the
    /// main diagonal.
    static ref LINES: [[Bitboard; 64]; 64] = {
        let mut lines = [[Bitboard::EMPTY; 64]; 64];

        for sq1 in Bitboard::ALL {
            let bishop_1 = MAGIC.bishop_attacks(Bitboard::EMPTY, sq1);
            let rook_1 = MAGIC.rook_attacks(Bitboard::EMPTY, sq1);
            for sq2 in Bitboard::ALL {
                if bishop_1.contains(sq2) {
                    let bishop_2 = MAGIC.bishop_attacks(Bitboard::EMPTY, sq2);
                    lines[sq1 as usize][sq2 as usize] |=
                        Bitboard::from(sq1) | Bitboard::from(sq2);
                    lines[sq1 as usize][sq2 as usize] |= bishop_1 & bishop_2;
                }
                if rook_1.contains(sq2) {
                    let rook_2 = MAGIC.rook_attacks(Bitboard::EMPTY, sq2);
                    lines[sq1 as usize][sq2 as usize] |=
                        Bitboard::from(sq1) | Bitboard::from(sq2);

                    lines[sq1 as usize][sq2 as usize] |= rook_1 & rook_2;
                }
            }
        }

        lines
    };

    /// A lookup table for the squares "between" two other squares, either down
    /// a row like a rook or on a diagonal like a bishop. `between[A1][A3]`
    /// would return a `Bitboard` with A2 as its only active square.
    static ref BETWEEN: [[Bitboard; 64]; 64] = {
        // start with an unitialized value and then set it element-wise
        let mut between = [[Bitboard::EMPTY; 64]; 64];

        for sq1 in Bitboard::ALL {
            for sq2 in Bitboard::ALL {
                if MAGIC.bishop_attacks(Bitboard::EMPTY, sq1).contains(sq2) {
                    let bishop1 = MAGIC.bishop_attacks(
                        Bitboard::from(sq2), sq1);
                    let bishop2 = MAGIC.bishop_attacks(
                        Bitboard::from(sq1), sq2);

                    between[sq1 as usize][sq2 as usize] |= bishop1 & bishop2;
                }
                if MAGIC.rook_attacks(Bitboard::EMPTY, sq1).contains(sq2) {
                    let rook1 = MAGIC.rook_attacks(Bitboard::from(sq2), sq1);
                    let rook2 = MAGIC.rook_attacks(Bitboard::from(sq1), sq2);

                    between[sq1 as usize][sq2 as usize] |= rook1 & rook2;
                }
            }
        }

        between
    };

    /// A bitboard of all the squares a knight can move to if its position is
    /// the index of the list.
    static ref KNIGHT_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KNIGHT_STEPS, 2);

    /// A bitboard of all the squares a king can move to if his position is the
    /// index in the list.
    static ref KING_MOVES: [Bitboard; 64] = create_step_attacks(&Direction::KING_STEPS, 1);

    /// A bitboard of all the squares which a pawn on the given square can
    /// attack. The first index is for White's pawn attacks, the second is for
    /// Black's.
    static ref PAWN_ATTACKS: [[Bitboard; 64]; 2] = [
        create_step_attacks(&[Direction::NORTHEAST, Direction::NORTHWEST], 1),
        create_step_attacks(&[Direction::SOUTHEAST, Direction::SOUTHWEST], 1),
    ];
}

/// The types of move generation. These are used in const generics, as enums are
/// not supported in const generics.
pub type GenMode = u8;

/// Generate all legal moves.
pub const ALL: GenMode = 0;
/// Generate all captures.
pub const CAPTURES: GenMode = 1;
/// Generate all quiet moves.
pub const QUIETS: GenMode = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A struct containing information for generating moves without self-checking,
/// such as the necessary pieces to block the king.
pub struct CheckInfo {
    /// The locations of pieces which are checking the king in the current
    /// position.
    pub checkers: Bitboard,
    /// The locations of pieces that are blocking would-be checkers from the
    /// opponent.
    pub king_blockers: [Bitboard; 2],

    /* One day I will use these, but for now we will just pretend they're not here. */
    // #[allow(unused)]
    // /// The locations of pieces which are pinning their corresponding blockers
    // /// in `king_blockers`.
    // pub pinners: [Bitboard; 2],
    // #[allow(unused)]
    // /// The squares which each piece could move to to check the opposing king.
    // pub check_squares: [Bitboard; Piece::NUM_TYPES],
}

impl CheckInfo {
    /// Create a new `CheckInfo` describing the given board. Requires that the
    /// board `b` is valid (i.e. only has one king of each color).
    pub fn about(b: &Board) -> CheckInfo {
        let kings = b[Piece::King];
        // we trust the board is valid here
        let (white_king_sq, black_king_sq, king_sq) = unsafe {
            (
                Square::unsafe_from(kings & b[Color::White]),
                Square::unsafe_from(kings & b[Color::Black]),
                Square::unsafe_from(kings & b[b.player_to_move]),
            )
        };

        let (blockers_white, _pinners_black) =
            CheckInfo::analyze_pins(b, b[Color::Black], white_king_sq);
        let (blockers_black, _pinners_white) =
            CheckInfo::analyze_pins(b, b[Color::White], black_king_sq);

        // outdated check square computations
        
        /*
        // we assume that we were not in check before
        let bishop_check_sqs = MAGIC.bishop_attacks(b.occupancy(), king_sq);
        let rook_check_sqs = MAGIC.rook_attacks(b.occupancy(), king_sq);
        */

        CheckInfo {
            checkers: square_attackers(b, king_sq, !b.player_to_move),
            king_blockers: [blockers_white, blockers_black],
            /*
            pinners: [pinners_white, pinners_black],
            check_squares: [
                PAWN_ATTACKS[b.player_to_move as usize][king_sq as usize],
                KNIGHT_MOVES[king_sq as usize],
                bishop_check_sqs,
                rook_check_sqs,
                bishop_check_sqs | rook_check_sqs,
                Bitboard::EMPTY,
            ],
            */
        }
    }

    /// Examine the pins in a position to generate the set of pinners and
    /// blockers on the square `sq`. The first return val is the set of
    /// blockers, and the second return val is the set of pinners. The blockers
    /// are pieces of either color that prevent an attack on `sq`. `sliders` is
    /// the set of all squares containing attackers we are interested in -
    /// typically, this is the set of all pieces owned by one color.
    fn analyze_pins(board: &Board, sliders: Bitboard, sq: Square) -> (Bitboard, Bitboard) {
        let mut blockers = Bitboard::EMPTY;
        let mut pinners = Bitboard::EMPTY;
        let sq_color = board.color_at_square(sq);
        let occupancy = board.occupancy();

        let rook_mask = MAGIC.rook_attacks(Bitboard::EMPTY, sq);
        let bishop_mask = MAGIC.bishop_attacks(Bitboard::EMPTY, sq);

        // snipers are pieces that could be pinners
        let snipers = sliders
            & ((rook_mask & (board[Piece::Queen] | board[Piece::Rook]))
                | (bishop_mask & (board[Piece::Queen] | board[Piece::Bishop])));

        // find the snipers which are blocked by only one piece
        for sniper_sq in snipers {
            let between_bb = between(sq, sniper_sq);

            if (between_bb & occupancy).count_ones() == 1 {
                blockers |= between_bb;
                if let Some(color) = sq_color {
                    if !(board[color] & between_bb).is_empty() {
                        pinners |= Bitboard::from(sniper_sq);
                    }
                }
            }
        }

        (blockers, pinners)
    }
}

/// A trait for objects which will give a "candidacy" to each move. This trait
/// exists to eliminate excess heap allocations on move generation, so that
/// moves are scored by their candidacy as soon as they're generated. The
/// generic parameter on the trait is the type of score that the nominator
/// creates.
pub trait NominateMove {
    /// The output type of the scoring function.
    type Output;

    /// Evaluate a move on a board, and score its quality on midgame and
    /// endgame results. The second element of the tuple is a blended
    /// evaluation from the originally-created score.
    fn score(m: Move, pos: &Position) -> Self::Output;
}

/// A simple nominator which performs no scoring on any move.
pub struct NoopNominator {}

impl NominateMove for NoopNominator {
    type Output = ();

    #[inline(always)]
    /// Do absolutely nothing.
    fn score(_: Move, _: &Position) {}
}

/// Determine whether any given move is legal, given a position in which it
/// could be played. Requires that the move must have been legal on *some*
/// board, but not necessarily the given one.
pub fn is_legal(m: Move, pos: &Position) -> bool {
    let n_checkers = pos.check_info.checkers.count_ones();
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let player = pos.board.player_to_move;
    let allies = pos.board[player];
    let enemies = pos.board[!player];
    let occupancy = allies | enemies;
    if allies.contains(to_sq) {
        // cannot move to square occupied by our piece
        return false;
    }
    if !allies.contains(from_sq) {
        return false;
    }
    match pos.board.type_at_square(from_sq) {
        Some(Piece::King) => {
            if m.promote_type().is_some() {
                // cannot promote non-pawn
                return false;
            }

            if m.is_en_passant() {
                // king cannot en passant
                return false;
            }

            let mut is_pseudolegal = KING_MOVES[from_sq as usize].contains(to_sq);
            if m.is_castle() && n_checkers == 0 {
                // just generate moves, since castle is quite rare
                let mut move_buf = Vec::with_capacity(2);
                castles::<NoopNominator>(pos, &mut move_buf);
                is_pseudolegal |= move_buf.contains(&(m, ()));
            }

            is_pseudolegal && validate(m, pos)
        }
        Some(pt) => {
            if n_checkers == 2 {
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

                if pos.board.en_passant_square != Some(to_sq) {
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
                        || (is_ep && pos.board.en_passant_square == Some(to_sq))
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
            match n_checkers {
                0 => (),
                1 => {
                    let checker_sq = Square::try_from(pos.check_info.checkers).unwrap();
                    let player_idx = pos.board.player_to_move as usize;
                    let king_idx = pos.king_sqs[player_idx] as usize;
                    let mut targets =
                        BETWEEN[king_idx][checker_sq as usize] | Bitboard::from(checker_sq);

                    if let Some(ep_sq) = pos.board.en_passant_square {
                        if pt == Piece::Pawn && (checker_sq == ep_sq - player.pawn_direction()) {
                            // allow en passants that let us escape check
                            targets |= Bitboard::from(ep_sq);
                        }
                    }

                    if !targets.contains(to_sq) {
                        return false;
                    }
                }
                _ => unreachable!(),
            };

            validate(m, pos)
        }
        None => false,
    }
}

#[inline(always)]
/// Get all the legal moves on a board.
pub fn get_moves<const M: GenMode, N: NominateMove>(pos: &Position) -> Vec<(Move, N::Output)> {
    let mut moves;
    let in_check = !pos.check_info.checkers.is_empty();

    match in_check {
        false => {
            // in the overwhelming majority of cases, there are fewer than 50
            // legal moves total
            let capacity = match M {
                ALL => 50,
                CAPTURES => 8,
                QUIETS => 40,
                _ => unreachable!(),
            };
            moves = Vec::with_capacity(capacity);
            non_evasions::<M, N>(pos, &mut moves);
        }
        true => {
            // in the overwhelming majority of cases, there are 8 or fewer
            // legal evasions if the king is in check
            moves = Vec::with_capacity(8);
            evasions::<M, N>(pos, &mut moves);
        }
    };

    moves
}

/// Does the player to move have any legal moves in this position? Requires
/// that the board is legal (i.e. has one of each king) to be correct.
pub fn has_moves(pos: &Position) -> bool {
    let b = &pos.board;
    let player = b.player_to_move;
    let player_occupancy = b[player];
    let opponent = !player;
    let occupancy = player_occupancy | b[opponent];
    let legal_targets = !player_occupancy;
    let king_square = pos.king_sqs[player as usize];
    let king_attackers = pos.check_info.checkers;
    let king_to_sqs = KING_MOVES[king_square as usize] & !player_occupancy;

    if !king_attackers.is_empty() {
        // king is in check

        // King can probably get out on his own
        for to_sq in king_to_sqs {
            let m = Move::normal(king_square, to_sq);
            if !is_move_self_check(pos, m) && !m.is_castle() {
                return true;
            }
        }

        // king moves could not prevent checks
        // if this is a double check, we must be mated
        if king_attackers.count_ones() > 1 {
            return false;
        }

        // only blocks can save us from checks
    } else {
        // examine king moves normally
        // we need not consider the castling squares because otherwise the king
        // would be able to escape naturally without castling
        for to_sq in king_to_sqs {
            if !is_move_self_check(pos, Move::normal(king_square, to_sq)) {
                return true;
            }
        }
    }
    for pt in Piece::NON_KING_TYPES {
        // examine moves that other pieces can make
        for from_sq in b[pt] & player_occupancy {
            let to_bb = match pt {
                Piece::Pawn => pawn_moves(b, from_sq, player),
                Piece::Bishop => MAGIC.bishop_attacks(occupancy, from_sq) & legal_targets,
                Piece::Rook => MAGIC.rook_attacks(occupancy, from_sq) & legal_targets,
                Piece::Queen => {
                    (MAGIC.bishop_attacks(occupancy, from_sq)
                        | MAGIC.rook_attacks(occupancy, from_sq) & legal_targets)
                        & legal_targets
                }
                Piece::Knight => KNIGHT_MOVES[from_sq as usize] & legal_targets,
                _ => Bitboard::EMPTY,
            };

            // we need not handle promotion because pawn promotion also can
            // block
            for to_sq in to_bb {
                if !is_move_self_check(pos, Move::normal(from_sq, to_sq)) {
                    return true;
                }
            }
        }
    }

    false
}

/// Determine whether a move is valid in the position on the board, given
/// that it was generated during the pseudolegal move generation process.
fn validate(m: Move, pos: &Position) -> bool {
    let b = &pos.board;
    let player = b.player_to_move;
    // the pieces which are pinned
    let pinned = pos.check_info.king_blockers[player as usize];
    let from_sq = m.from_square();
    let from_bb = Bitboard::from(from_sq);
    let to_sq = m.to_square();
    let to_bb = Bitboard::from(to_sq);

    // verify that taking en passant does not result in self-check
    if m.is_en_passant() {
        let king_sq = pos.king_sqs[player as usize];
        let enemy = b[!b.player_to_move];
        let capture_bb = to_bb << -b.player_to_move.pawn_direction().0;

        let new_occupancy = b.occupancy() ^ from_bb ^ capture_bb ^ to_bb;

        return (MAGIC.rook_attacks(new_occupancy, king_sq)
            & (b[Piece::Rook] | b[Piece::Queen])
            & enemy)
            .is_empty()
            && (MAGIC.bishop_attacks(new_occupancy, king_sq)
                & (b[Piece::Bishop] | b[Piece::Queen])
                & enemy)
                .is_empty();
    }

    // Validate passthrough squares for castling
    if m.is_castle() {
        let is_queen_castle = m.to_square().file() == 2;
        let mut king_passthru_min = 4;
        let mut king_passthru_max = 7;
        if is_queen_castle {
            king_passthru_min = 2;
            king_passthru_max = 5;
        }
        for file in king_passthru_min..king_passthru_max {
            let target_sq = Square::new(m.from_square().rank(), file).unwrap();
            if is_square_attacked_by(b, target_sq, !b.player_to_move) {
                return false;
            }
        }
    }

    let king_sq = pos.king_sqs[player as usize];

    // Other king moves must make sure they don't step into check
    if from_sq == king_sq {
        let new_occupancy = (b.occupancy() ^ from_bb) | to_bb;
        return square_attackers_occupancy(b, to_sq, !b.player_to_move, new_occupancy).is_empty();
    }

    // the move is valid if the piece is not pinned, or if the piece is pinned
    // and stays on the same line as it was pinned on.
    //
    // it is reasonable to use `aligned()` here because there's no way a piece
    // can stay aligned in a move without keeping the pin appeased.
    (pinned & from_bb).is_empty() || aligned(m.from_square(), m.to_square(), king_sq)
}

/// In a given board state, is a move illegal because it would be a
/// self-check?
pub fn is_move_self_check(pos: &Position, m: Move) -> bool {
    let board = &pos.board;
    let from_sq = m.from_square();
    let to_sq = m.to_square();
    let player = board.player_to_move;
    // Square where the king will be after this move ends.
    let mut king_square = pos.king_sqs[player as usize];
    let is_king_move = king_square == from_sq;
    let opponent = !player;

    if is_king_move {
        if is_square_attacked_by(board, from_sq, opponent) {
            return true;
        }
        // The previous check skips moves where the king blocks himself. We
        // can use magic bitboards to find out the rest.
        king_square = to_sq;
    }
    // Self checks can only happen by discovery (including by moving the
    // king "out of its own way"), or by doing nothing about a check.
    // Typically, only one square is emptied by moving. However, in en
    // passant, two squares are emptied. We can check the results by masking
    // out the squares which were emptied, and then seeing which attacks
    // went through using magic bitboards.

    let mut squares_emptied = Bitboard::from(from_sq);
    if m.is_en_passant() {
        squares_emptied |=
            Bitboard::from(board.en_passant_square.unwrap() + opponent.pawn_direction());
    }
    let occupancy = (board.occupancy() & !squares_emptied) | Bitboard::from(from_sq);

    let attackers = square_attackers_occupancy(board, king_square, opponent, occupancy);

    //attackers which we will capture are not a threat
    !(attackers & !Bitboard::from(m.to_square())).is_empty()
}

#[inline(always)]
/// In a given board state, is a square attacked by the given color?
pub fn is_square_attacked_by(board: &Board, sq: Square, color: Color) -> bool {
    !square_attackers(board, sq, color).is_empty()
}

#[inline(always)]
/// Enumerate the legal moves a player of the given color would be
/// able to make if it were their turn to move, and if the player is not in
/// check.
fn non_evasions<const M: GenMode, N: NominateMove>(
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
) {
    let target_sqs = match M {
        ALL => Bitboard::ALL,
        CAPTURES => pos.board[!pos.board.player_to_move],
        QUIETS => !pos.board[!pos.board.player_to_move],
        _ => unreachable!(),
    };

    normal_piece_assistant::<N>(pos, moves, target_sqs);

    let mut pawn_targets = target_sqs;
    if M != QUIETS {
        if let Some(ep_sq) = pos.board.en_passant_square {
            pawn_targets |= Bitboard::from(ep_sq);
        }
    }
    pawn_assistant::<M, N>(pos, moves, pawn_targets);

    // generate king moves
    if M != CAPTURES {
        castles::<N>(pos, moves);
    }
    king_move_non_castle::<N>(pos, moves, target_sqs);
}

/// Compute the evasions in a position where the king is checked, and then push
/// those evading moves into the moves buffer.
fn evasions<const M: GenMode, N: NominateMove>(pos: &Position, moves: &mut Vec<(Move, N::Output)>) {
    let b = &pos.board;
    let player = b.player_to_move;
    let king_sq = pos.king_sqs[player as usize];

    // only look at non-king moves if we are not in double check
    if pos.check_info.checkers.count_ones() == 1 {
        // this unsafe is fine because we already checked
        let checker_sq = unsafe { Square::unsafe_from(pos.check_info.checkers) };
        // Look for blocks or captures
        let mut target_sqs = between(king_sq, checker_sq) | pos.check_info.checkers;
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
                if pos.check_info.checkers.contains(ep_attacker_sq) {
                    pawn_targets |= Bitboard::from(ep_sq);
                }
            }
        }

        pawn_assistant::<M, N>(pos, moves, pawn_targets);
        normal_piece_assistant::<N>(pos, moves, target_sqs);
    }

    let king_targets = match M {
        ALL => Bitboard::ALL,
        CAPTURES => b[!player],
        QUIETS => !b[!player],
        _ => unreachable!(),
    };
    king_move_non_castle::<N>(pos, moves, king_targets);
}

#[inline(always)]
/// Get the attackers of a given color on a square as a `Bitboard`
/// representing the squares of the attackers.
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
    let pawn_vision = pawn_captures(board, sq, !color);
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
fn pawn_assistant<const M: GenMode, N: NominateMove>(
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
    target: Bitboard,
) {
    let board = &pos.board;
    let player = pos.board.player_to_move;
    let allies = board[player];
    let opponents = board[!player];
    let occupancy = allies | opponents;
    let unoccupied = !occupancy;
    let pawns = board[Piece::Pawn] & allies;
    let rank8 = player.pawn_promote_rank();
    let not_rank8 = !rank8;
    let rank3 = match player {
        Color::White => Bitboard::new(0xFF0000),
        Color::Black => Bitboard::new(0xFF0000000000),
    };
    let direction = player.pawn_direction();
    let doubledir = 2 * direction;

    if M != CAPTURES {
        // pawn forward moves

        let mut singles = (pawns << direction.0) & unoccupied;
        let doubles = ((singles & rank3) << direction.0) & target & unoccupied;
        singles &= target;

        for to_sq in singles & not_rank8 {
            let m = Move::normal(to_sq - direction, to_sq);
            if validate(m, pos) {
                moves.push((m, N::score(m, pos)));
            }
        }

        for to_sq in doubles {
            let m = Move::normal(to_sq - doubledir, to_sq);
            if validate(m, pos) {
                moves.push((m, N::score(m, pos)));
            }
        }

        pawn_promotion_helper::<N>(singles & rank8, direction, pos, moves);
    }

    if M != QUIETS {
        // pawn captures
        let capture_dir_e = direction + Direction::EAST;
        let capture_dir_w = direction + Direction::WEST;

        let capture_mask = opponents & target;

        // prevent pawns from capturing by wraparound
        const NOT_WEST: Bitboard = Bitboard::new(0xFEFEFEFEFEFEFEFE);
        const NOT_EAST: Bitboard = Bitboard::new(0x7F7F7F7F7F7F7F7F);
        let capture_e = ((pawns & NOT_EAST) << capture_dir_e.0) & capture_mask;
        let capture_w = ((pawns & NOT_WEST) << capture_dir_w.0) & capture_mask;

        for to_sq in capture_e & not_rank8 {
            let m = Move::normal(to_sq - capture_dir_e, to_sq);
            if validate(m, pos) {
                moves.push((m, N::score(m, pos)));
            }
        }
        for to_sq in capture_w & not_rank8 {
            let m = Move::normal(to_sq - capture_dir_w, to_sq);
            if validate(m, pos) {
                moves.push((m, N::score(m, pos)));
            }
        }

        pawn_promotion_helper::<N>(capture_e & rank8, capture_dir_e, pos, moves);
        pawn_promotion_helper::<N>(capture_w & rank8, capture_dir_w, pos, moves);

        // en passant
        if let Some(ep_square) = board.en_passant_square {
            if target.contains(ep_square) {
                let from_sqs = PAWN_ATTACKS[!player as usize][ep_square as usize] & pawns;
                for from_sq in from_sqs {
                    let m = Move::en_passant(from_sq, ep_square);
                    if validate(m, pos) {
                        moves.push((m, N::score(m, pos)));
                    }
                }
            }
        }
    }
}

#[inline(always)]
/// Helper function to create pawn promotion moves. `to_bb` is the set of
/// target bitboards, and `move_direction` is the direction pawns would move to
/// reach the targets in `to_bb`. The moves will be appended onto the `moves`
/// vector.
fn pawn_promotion_helper<N: NominateMove>(
    to_bb: Bitboard,
    move_direction: Direction,
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
) {
    for to_sq in to_bb {
        let from_sq = to_sq - move_direction;
        // we only need to validate one promotion move.
        // order our promotions so that moves which will probably be better
        // (queen promotions) will be nearer to the front.
        let m1 = Move::promoting(from_sq, to_sq, Piece::Queen);
        if validate(m1, pos) {
            moves.push((m1, N::score(m1, pos)));
            for promote_type in [Piece::Knight, Piece::Rook, Piece::Bishop] {
                let m = Move::promoting(from_sq, to_sq, promote_type);
                moves.push((m, N::score(m, pos)));
            }
        }
    }
}

/// Generate all the moves for a knight, bishop, rook, or queen which end
/// up on the target.
fn normal_piece_assistant<N: NominateMove>(
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
    target: Bitboard,
) {
    let board = &pos.board;
    let player = pos.board.player_to_move;
    let allies = board[player];
    let legal_targets = !allies & target;
    let occupancy = allies | board[!player];
    let queens = board[Piece::Queen];
    let rook_movers = (board[Piece::Rook] | queens) & allies;
    let bishop_movers = (board[Piece::Bishop] | queens) & allies;

    for sq in board[Piece::Knight] & allies {
        append_valid_normal::<N>(sq, KNIGHT_MOVES[sq as usize] & legal_targets, pos, moves)
    }
    for sq in bishop_movers {
        append_valid_normal::<N>(
            sq,
            MAGIC.bishop_attacks(occupancy, sq) & legal_targets,
            pos,
            moves,
        );
    }
    for sq in rook_movers {
        append_valid_normal::<N>(
            sq,
            MAGIC.rook_attacks(occupancy, sq) & legal_targets,
            pos,
            moves,
        );
    }
}

/// Get the pseudolegal moves that a pawn on square `sq` with color `color`
/// could make in this position, expressed as a `Bitboard` with a 1 at every
/// valid target square.
fn pawn_moves(board: &Board, sq: Square, color: Color) -> Bitboard {
    let dir = color.pawn_direction();
    let start_rank = color.pawn_start_rank();
    let from_bb = Bitboard::from(sq);
    let occupancy = board.occupancy();
    let mut target_squares = Bitboard::EMPTY;
    //this will never be out of bounds because pawns don't live on promotion rank
    if !occupancy.contains(sq + dir) {
        target_squares |= Bitboard::from(sq + dir);
        //pawn is on start rank and double-move square is not occupied
        if !(start_rank & from_bb).is_empty() && !occupancy.contains(sq + 2 * dir) {
            target_squares |= Bitboard::from(sq + 2 * dir);
        }
    }
    target_squares |= pawn_captures(board, sq, color);
    target_squares &= !board[color];

    target_squares
}

#[inline(always)]
/// Get the captures a pawn can make in the current position. The given
/// color is the color that a pawn would be to generate the captures from
/// this square. `color` is the color of the piece at `sq`. The result is a
/// `Bitboard` with a 1 at every valid target square.
fn pawn_captures(board: &Board, sq: Square, color: Color) -> Bitboard {
    let mut capture_mask = board[!color];
    if let Some(ep_square) = board.en_passant_square {
        capture_mask |= Bitboard::from(ep_square);
    }

    PAWN_ATTACKS[color as usize][sq as usize] & capture_mask
}

#[inline(always)]
/// Get the moves that a king could make in a position that are not castles,
/// and append them into the moves buffer.
fn king_move_non_castle<N: NominateMove>(
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
    target: Bitboard,
) {
    let king_sq = pos.king_sqs[pos.board.player_to_move as usize];
    let allies = pos.board[pos.board.player_to_move];
    let to_bb = KING_MOVES[king_sq as usize] & !allies & target;
    append_valid_normal::<N>(king_sq, to_bb, pos, moves);
}

#[inline(always)]
/// Get the castling moves that the king could make in this position, and
/// append them onto the target vector.
fn castles<N: NominateMove>(pos: &Position, moves: &mut Vec<(Move, N::Output)>) {
    let player = pos.board.player_to_move;
    let occ = pos.board.occupancy();
    let king_sq = pos.king_sqs[player as usize];

    // the squares the king must pass through to reach the castled position
    let kingside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x0000000000000060),
        Color::Black => Bitboard::new(0x6000000000000000),
    };

    let can_kingside_castle = pos.board.castle_rights.is_kingside_castle_legal(player)
        && (occ & kingside_castle_passthrough_sqs).is_empty();

    if can_kingside_castle {
        let m = Move::castling(king_sq, Square::new(king_sq.rank(), 6).unwrap());
        if validate(m, pos) {
            moves.push((m, N::score(m, pos)));
        }
    }

    // now, repeat the same process for queenside castling

    let queenside_castle_passthrough_sqs = match player {
        Color::White => Bitboard::new(0x000000000000000E),
        Color::Black => Bitboard::new(0x0E00000000000000),
    };

    let can_queenside_castle = pos.board.castle_rights.is_queenside_castle_legal(player)
        && (occ & queenside_castle_passthrough_sqs).is_empty();

    if can_queenside_castle {
        let m = Move::castling(king_sq, Square::new(king_sq.rank(), 2).unwrap());
        if validate(m, pos) {
            moves.push((m, N::score(m, pos)));
        }
    }
}

#[inline(always)]
/// Get a bitboard of all the squares between the two given squares, along
/// the moves of a bishop or rook.
pub fn between(sq1: Square, sq2: Square) -> Bitboard {
    BETWEEN[sq1 as usize][sq2 as usize]
}

#[inline(always)]
/// Determine whether three squares are aligned according to rook or bishop
/// directions.
pub fn aligned(sq1: Square, sq2: Square, sq3: Square) -> bool {
    !(LINES[sq1 as usize][sq2 as usize] & Bitboard::from(sq3)).is_empty()
}

/// Get the step attacks that could be made by moving in `dirs` from each point
/// in the square. Exclude the steps that travel more than `max_dist` (this
/// prevents overflow around the edges of the board).
fn create_step_attacks(dirs: &[Direction], max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard::EMPTY; 64];
    for (i, item) in attacks.iter_mut().enumerate() {
        for dir in dirs {
            let start_sq = Square::try_from(i as u8).unwrap();
            let target_sq = start_sq + *dir;
            if target_sq.chebyshev_to(start_sq) <= max_dist {
                *item |= Bitboard::from(target_sq);
            }
        }
    }

    attacks
}

#[inline(always)]
/// Perform `append_valid_moves()`, assuming the move is a "normal" one, i.e.
/// not castling, en passant, or a promotion.
fn append_valid_normal<N: NominateMove>(
    from_sq: Square,
    to_bb: Bitboard,
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
) {
    append_valid_moves::<N>(from_sq, to_bb, None, false, false, pos, moves);
}

/// Append all the validated pseudolegal moves from `from_sq` to each square in
/// `to_bb` to the `moves` vector, computing the nomination values along the
/// way. `promote_type`, `castle`, and `en_passant` are the tags on the move
/// which describe its metadata.
fn append_valid_moves<N: NominateMove>(
    from_sq: Square,
    to_bb: Bitboard,
    promote_type: Option<Piece>,
    castle: bool,
    en_passant: bool,
    pos: &Position,
    moves: &mut Vec<(Move, N::Output)>,
) {
    for to_sq in to_bb {
        let m = Move::new(from_sq, to_sq, promote_type, castle, en_passant);
        if validate(m, pos) {
            moves.push((m, N::score(m, pos)));
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    /// Test that we can play Qf3+, the critical move in the Fried Liver
    /// opening.
    fn test_best_queen_fried_liver() {
        let m = Move::normal(Square::D1, Square::F3);
        // the fried liver position, before Qf3+
        let pos = Position::from_fen(
            "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
            Position::no_eval,
        )
        .unwrap();
        let moves = get_moves::<ALL, NoopNominator>(&pos);
        assert!(moves.contains(&(m, ())));
        for m in moves {
            assert!(is_legal(m.0, &pos));
        }
    }

    #[test]
    /// Test that capturing a pawn is parsed correctly.
    fn test_pawn_capture_generated() {
        // check that exf5 is generated
        let pos = Position::from_fen(
            "rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::normal(Square::E4, Square::F5);
        for m in get_moves::<ALL, NoopNominator>(&pos) {
            assert!(is_legal(m.0, &pos));
        }
        assert!(get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
        assert!(get_moves::<CAPTURES, NoopNominator>(&pos).contains(&(m, ())));
    }

    #[test]
    /// The pawn is checking the king. Is move enumeration correct?
    fn test_enumerate_pawn_checking_king() {
        let pos = Position::from_fen(
            "r1bq1b1r/ppp2kpp/2n5/3n4/2B5/8/PPP1pPPP/RN1Q1K1R w - - 0 10",
            Position::no_eval,
        )
        .unwrap();

        let moves = get_moves::<ALL, NoopNominator>(&pos);

        for m2 in moves.iter() {
            assert!(is_legal(m2.0, &pos));
        }
    }

    #[test]
    /// In a mated position, make sure that the king has no moves.
    fn test_white_mated_has_no_moves() {
        let pos = Position::from_fen(
            "r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11",
            Position::no_eval,
        )
        .unwrap();
        assert!(!has_moves(&pos));
        let moves = get_moves::<ALL, NoopNominator>(&pos);
        for m in moves {
            assert!(is_legal(m.0, &pos));
        }
        assert!(get_moves::<ALL, NoopNominator>(&pos).is_empty());
    }

    #[test]
    /// Check that the king has exactly one move in this position.
    fn test_king_has_only_one_move() {
        let pos =
            Position::from_fen("2k5/4R3/8/5K2/3R4/8/8/8 b - - 2 2", Position::no_eval).unwrap();
        assert!(has_moves(&pos));
        assert!(get_moves::<ALL, NoopNominator>(&pos).len() == 1);
        assert!(is_legal(Move::normal(Square::C8, Square::B8), &pos));
    }

    #[test]
    /// Test that queenside castling actually works.
    fn test_queenside_castle() {
        let pos = Position::from_fen(
            "r3kb1r/ppp1p1pp/2nq1n2/1B1p4/3P4/2N2Q2/PPP2PPP/R1B1K2R b KQkq - 0 8",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::castling(Square::E8, Square::C8);
        assert!(get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
        assert!(is_legal(m, &pos));
    }

    #[test]
    /// Test that Black cannot castle because there is a knight in the way.
    fn test_no_queenside_castle_through_knight() {
        let pos = Position::from_fen(
            "rn2kbnr/ppp1pppp/3q4/3p4/6b1/8/PPPPPPPP/RNBQKBNR b KQkq - 5 4",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::castling(Square::E8, Square::C8);
        assert!(!get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));

        assert!(!is_legal(m, &pos));
    }

    #[test]
    /// Test that loud moves are generated correctly on the Fried Liver
    /// position.
    fn test_get_loud_moves_fried_liver() {
        loud_moves_helper("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7");
    }

    #[test]
    /// Test that loud moves are generated correctly in a position where en
    /// passant is possible.
    fn test_get_loud_moves_en_passant() {
        loud_moves_helper("rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3");
    }

    #[test]
    fn test_get_loud_moves_pawn_capture() {
        loud_moves_helper("rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2");
    }

    #[test]
    fn test_get_loud_moves_rook_hanging() {
        loud_moves_helper("rnbqk2r/ppppnp1p/4p1pb/8/4P3/1P1P4/PBP2PPP/RN1QKBNR w KQkq - 1 5");
    }

    #[test]
    fn test_recapture_knight_loud_move() {
        loud_moves_helper("r2q1bkr/ppp3pp/2n5/3Np3/6Q1/8/PPPP1PPP/R1B1K2R b KQ - 0 10");
    }

    #[test]
    /// Test that a king can escape check without capturing the checker.
    fn test_king_escape_without_capture() {
        let pos = Position::from_fen(
            "r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10",
            Position::no_eval,
        )
        .unwrap();
        let moves = get_moves::<ALL, NoopNominator>(&pos);
        let expected_moves = vec![
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in moves.iter() {
            assert!(expected_moves.contains(&m.0));
            assert!(is_legal(m.0, &pos));
        }
        for em in expected_moves.iter() {
            assert!(moves.contains(&(*em, ())));
            assert!(is_legal(*em, &pos));
        }
    }

    #[test]
    /// Test that Black can promote a piece (on e1).
    fn test_black_can_promote() {
        let pos = Position::from_fen("8/8/5k2/3K4/8/8/4p3/8 b - - 0 1", Position::no_eval).unwrap();
        let moves = get_moves::<ALL, NoopNominator>(&pos);
        for m in moves.iter() {
            assert!(is_legal(m.0, &pos));
        }
        assert!(moves.contains(&(Move::promoting(Square::E2, Square::E1, Piece::Queen), ())));
    }

    #[test]
    /// Test that pawns cannot "wrap around" the side of the board.
    fn test_no_wraparound() {
        let pos = Position::from_fen(
            "r3k2r/Pppp1ppp/1b3nbN/nP6/BBPPP3/q4N2/Pp4PP/R2Q1RK1 b kq - 0 1",
            Position::no_eval,
        )
        .unwrap();

        let moves = get_moves::<ALL, NoopNominator>(&pos);
        let m = Move::normal(Square::H7, Square::A7);
        assert!(!(moves.contains(&(m, ()))));
        assert!(!is_legal(m, &pos));
    }

    #[test]
    /// Test that a move flagged as en passant is illegal, even if it is an
    /// otherwise normal capture.
    fn test_en_passant_illegal() {
        let pos = Position::from_fen(
            "r6r/3n1pk1/p4p2/3p4/2p1p1q1/1P2P1P1/P1PP1P1P/R1B1R1K1 b - - 0 25",
            Position::no_eval,
        )
        .unwrap();
        let m = Move::en_passant(Square::C4, Square::B3);

        assert!(!is_legal(m, &pos));
        assert!(!get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
        assert!(!get_moves::<CAPTURES, NoopNominator>(&pos).contains(&(m, ())));
    }

    #[test]
    /// Test that a pawn cannot en passant if doing so would put the king in
    /// check.
    fn test_en_passant_pinned() {
        let pos = Position::from_fen(
            "8/2p5/3p4/KPr5/2R1Pp1k/8/6P1/8 b - e3 0 2",
            Position::no_eval,
        )
        .unwrap();
        let moves = get_moves::<ALL, NoopNominator>(&pos);
        let m = Move::en_passant(Square::F4, Square::E3);
        assert!(!moves.contains(&(m, ())));
        assert!(!is_legal(m, &pos));
    }

    #[test]
    /// Test that a move must be tagged as en passant to be considered legal to
    /// escape check.
    fn test_en_passant_tagged() {
        let pos = Position::from_fen(
            "2B1kb2/pp2pp2/7p/1PpQP3/2nK4/8/P1r4R/R7 w - c6 0 27",
            Position::no_eval,
        )
        .unwrap();

        let m = Move::normal(Square::B5, Square::C6);
        assert!(!is_legal(m, &pos));
        assert!(!get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
    }
    #[test]
    /// Test that a pinned piece cannot make a capture if it does not defend
    /// against the pin.
    fn test_pinned_knight_capture() {
        let pos = Position::from_fen(
            "r2q1b1r/ppp2kpp/2n5/3npb2/2B5/2N5/PPPP1PPP/R1BQ1RK1 b - - 3 8",
            Position::no_eval,
        )
        .unwrap();
        let illegal_move = Move::normal(Square::D5, Square::C3);

        assert!(!get_moves::<ALL, NoopNominator>(&pos).contains(&(illegal_move, ())));
        assert!(!get_moves::<CAPTURES, NoopNominator>(&pos).contains(&(illegal_move, ())));
        assert!(!is_legal(illegal_move, &pos));
    }

    #[test]
    /// Test that en passant moves are generated correctly.
    fn test_en_passant_generated() {
        // exf6 is en passant
        let pos = Position::from_fen(
            "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            Position::no_eval,
        )
        .unwrap();

        let m = Move::en_passant(Square::E5, Square::F6);

        assert!(get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
        assert!(get_moves::<CAPTURES, NoopNominator>(&pos).contains(&(m, ())));
        assert!(is_legal(m, &pos));
    }

    #[test]
    /// Test that a player can en passant out of check if it results in a
    /// checking pawn being captured.
    fn test_en_passant_out_of_check() {
        // bxc6 should be legal here
        let pos = Position::from_fen(
            "8/8/8/1Ppp3r/1KR2p1k/8/4P1P1/8 w - c6 0 3",
            Position::no_eval,
        )
        .unwrap();

        let m = Move::en_passant(Square::B5, Square::C6);

        assert!(get_moves::<ALL, NoopNominator>(&pos).contains(&(m, ())));
        assert!(is_legal(m, &pos));
        assert!(has_moves(&pos));
    }

    #[test]
    /// Test that a position where a rook is horizontal to the king is mate.
    fn test_horizontal_rook_mate() {
        let pos = Position::from_fen(
            "r1b2k1R/3n1p2/p7/3P4/6Qp/2P3b1/6P1/4R2K b - - 0 32",
            Position::no_eval,
        )
        .unwrap();

        assert!(get_moves::<ALL, NoopNominator>(&pos).is_empty());
        assert!(!has_moves(&pos));
    }

    /// A helper function that will force that the given FEN will have loud
    /// moves generated correctly.
    fn loud_moves_helper(fen: &str) {
        let pos = Position::from_fen(fen, Position::no_eval).unwrap();

        let moves = get_moves::<ALL, NoopNominator>(&pos);
        let loud_moves = get_moves::<CAPTURES, NoopNominator>(&pos);

        for loud_move in loud_moves.iter() {
            assert!(moves.contains(loud_move));
            assert!(pos.board.is_move_capture(loud_move.0));
        }

        for normal_move in moves.iter() {
            assert!(is_legal(normal_move.0, &pos));
            if pos.board.is_move_capture(normal_move.0) {
                assert!(loud_moves.contains(normal_move));
            }
        }
    }
}
