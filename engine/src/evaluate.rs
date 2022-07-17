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

//! Static evaluation of positions.
//!
//! Of all the parts of a chess engine, static evaluation is arguably the most
//! important. Every leaf of the search is statically evaluated, and based on
//! the comparisons of each evaluation, the full minimax search is achieved.
//!
//! Fiddler uses a classical approach to static evaluation: the final evaluation
//! is the sum of a number of rules. Each rule contributes a quantity to the
//! evaluation.
//!
//! Also like other engines, Fiddler uses a "tapered" evaluation: rules are
//! given different weights at different phases of the game. To prevent sharp
//! changes in evaluation as the phase blends, a "midgame" and "endgame"
//! evaluation is created, and then the final evaluation is a linear combination
//! of those two.
//!
//! More uniquely, Fiddler is obsessed with cumulative evaluation. Often,
//! learning facts about a board is lengthy and difficult (in computer time - it
//! takes nanoseconds in wall time). However, it is generally easy to guess what
//! effect a move will have on the static evaluation of a position. We therefore
//! tag moves with their effect on the evaluation, allowing us to cheaply
//! evaluate the final leaf position.

use std::{
    cmp::{max, min},
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use fiddler_base::{
    game::{TaggedGame, Tagger},
    Bitboard, Board, Color, Move, Piece,
};

use crate::{
    material::material_delta,
    pick::candidacy,
    pst::{pst_delta, pst_evaluate},
};

use super::material;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
/// A wrapper for the evaluation of a position.
/// The higher an evaluation is, the better the position is for White. An
/// evaluation of 0 is a draw.
/// Internally, the i32 represents an integer. The integer value is 1/1000 of a
/// pawn (so if the internal value is +2000, the position is +2 pawns for White)
/// .
///
/// Values > 29,000 are reserved for mates. 30,000 is White to mate in
/// 0 (i.e. White has won the game), 29,999 is White to mate in 1 (White will
/// play their move and mate), 29,998 is White to mate in 1, with Black to
/// move (Black will play their move, then White will play their move to mate)
/// and so on. Values of < -29,000 are reserved for black mates, likewise.
///
/// # Examples
///
/// ```
/// use fiddler_engine::evaluate::Eval;
/// let mate_eval = Eval::mate_in(3);
/// let draw_eval = Eval::DRAW;
/// assert!(mate_eval > draw_eval);
/// ```
pub struct Eval(i16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A `Score` is a pair of two `Evals` - one for the midgame and one for the
/// endgame. The values inside of a `Score` should never be mate values.
///
/// Internally, `Score`s are represented as a single integer to improve
/// arithmetic speed. The higher 16 bits are for the midgame evaluation, and the
/// lower 16 are for endgame.
pub struct Score {
    /// The midgame-only evaluation of a position.
    pub mg: Eval,
    /// The endgame-only evaluation of a position.
    pub eg: Eval,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoreTag;

pub type ScoredGame = TaggedGame<ScoreTag>;

impl Tagger for ScoreTag {
    type Tag = (Score, Eval);
    type Cookie = Score;

    /// Compute the change in scoring that a move made on a board will cause.
    fn tag_move(m: Move, b: &Board) -> Self::Tag {
        let delta = pst_delta(b, m) + material_delta(b, m);
        (delta, candidacy(b, m, delta))
    }

    fn update_cookie(
        _: Move,
        tag: &Self::Tag,
        b: &Board,
        prev_cookie: &Self::Cookie,
    ) -> Self::Cookie {
        match b.player {
            Color::White => *prev_cookie + tag.0,
            Color::Black => *prev_cookie - tag.0,
        }
    }

    /// Compute a static, cumulative-invariant evaluation of a position.
    /// It is much faster in search to use cumulative evaluation, but this should be used when
    /// importing positions.
    /// Static evaluation will not include the leaf rules (such as number of
    /// doubled pawns), as this will be handled by `leaf_evaluate` at the end of
    /// the search tree.
    fn init_cookie(b: &Board) -> Self::Cookie {
        material::evaluate(b) + pst_evaluate(b)
    }
}

/// Mask containing ones along the A file. Bitshifting left by a number from 0
/// through 7 will cause it to become a mask for each file.
const A_FILE_MASK: Bitboard = Bitboard::new(0x0101010101010101);

/// The value of having your own pawn doubled.
pub const DOUBLED_PAWN_VALUE: Score = Score::centipawns(-25, -26);
/// The value of having a rook with no same-colored pawns in front of it which
/// are not advanced past the 3rd rank.
pub const OPEN_ROOK_VALUE: Score = Score::centipawns(5, 78);

/// Evaluate a leaf position on a game whose cumulative values have been
/// computed correctly.
pub fn leaf_evaluate(g: &ScoredGame) -> Eval {
    let b = g.board();

    match g.is_over() {
        (true, Some(_)) => {
            return match b.player {
                Color::Black => Eval::mate_in(0),
                Color::White => -Eval::mate_in(0),
            }
        }
        (true, None) => {
            return Eval::DRAW;
        }
        _ => {}
    };

    let b = g.board();
    let leaf_val = leaf_rules(b);

    (leaf_val + *g.cookie()).blend(phase_of(b))
}

/// Get the score gained from evaluations that are only performed at the leaf.
fn leaf_rules(b: &Board) -> Score {
    // Add losses due to doubled pawns
    let mut score = DOUBLED_PAWN_VALUE * net_doubled_pawns(b);

    // Add gains from open rooks
    score += OPEN_ROOK_VALUE * net_open_rooks(b);

    score
}

/// Count the number of "open" rooks (i.e., those which are not blocked by
/// unadvanced pawns) in a position. The number is a net value, so it will be
/// negative if Black has more open rooks than White.
pub fn net_open_rooks(b: &Board) -> i8 {
    // Mask for pawns which are above rank 3 (i.e. on the white half of the
    // board).
    const BELOW_RANK3: Bitboard = Bitboard::new(0xFFFFFFFF);
    // Mask for pawns which are on the black half of the board
    const ABOVE_RANK3: Bitboard = Bitboard::new(0x00000000FFFFFFFF);
    let mut net_open_rooks = 0i8;
    let rooks = b[Piece::Rook];
    let pawns = b[Piece::Pawn];
    let white = b[Color::White];
    let black = b[Color::Black];

    // count white rooks
    for wrook_sq in rooks & white {
        if wrook_sq.rank() >= 3 {
            net_open_rooks += 1;
            continue;
        }
        let pawns_in_col = (pawns & white) & (A_FILE_MASK << wrook_sq.file());
        let important_pawns = BELOW_RANK3 & pawns_in_col;
        // check that the forward-most pawn of the important pawns is in front
        // of or behind the rook
        if important_pawns.leading_zeros() > (63 - (wrook_sq as u32)) {
            // all the important pawns are behind the rook
            net_open_rooks += 1;
        }
    }

    // count black rooks
    for brook_sq in rooks & black {
        if brook_sq.rank() <= 4 {
            net_open_rooks -= 1;
            continue;
        }
        let pawns_in_col = (pawns & black) & (A_FILE_MASK << brook_sq.file());
        let important_pawns = ABOVE_RANK3 & pawns_in_col;
        // check that the lowest-rank pawn that could block the rook is behind
        // the rook
        if important_pawns.trailing_zeros() > brook_sq as u32 {
            net_open_rooks -= 1;
        }
    }

    net_open_rooks
}

/// Count the number of doubled pawns, in net. For instance, if White had 1
/// doubled pawn, and Black had 2, this function would return -1.
pub fn net_doubled_pawns(b: &Board) -> i8 {
    let white_occupancy = b[Color::White];
    let pawns = b[Piece::Pawn];
    let mut npawns: i8 = 0;
    let mut col_mask = Bitboard::new(0x0101010101010101);
    for _ in 0..8 {
        let col_pawns = pawns & col_mask;

        // all ones on the A column, shifted left by the col
        let num_black_doubled_pawns = match ((!white_occupancy) & col_pawns).len() {
            0 => 0,
            x => x as i8 - 1,
        };
        let num_white_doubled_pawns = match (white_occupancy & col_pawns).len() {
            0 => 0,
            x => x as i8 - 1,
        };

        npawns -= num_black_doubled_pawns;
        npawns += num_white_doubled_pawns;

        col_mask <<= 1;
    }

    npawns
}

/// Get a blending float describing the current phase of the game. Will range
/// from 0 (full endgame) to 1 (full midgame).
pub fn phase_of(b: &Board) -> f32 {
    const MG_LIMIT: Eval = Eval::centipawns(2500);
    const EG_LIMIT: Eval = Eval::centipawns(1400);
    // amount of non-pawn material in the board, under midgame values
    let mg_npm = {
        let mut total = Eval::DRAW;
        for pt in Piece::NON_PAWN_TYPES {
            total += material::value(pt).mg * b[pt].len();
        }
        total
    };
    let bounded_npm = max(EG_LIMIT, min(MG_LIMIT, mg_npm));

    (EG_LIMIT - bounded_npm).float_val() / (EG_LIMIT - MG_LIMIT).float_val()
}

impl Eval {
    /// An evaluation which is smaller than every other "normal" evaluation.
    pub const MIN: Eval = Eval(-Eval::MATE_0_VAL - 1000);

    /// An evaluation which is larger than every other "normal" evaluation.
    pub const MAX: Eval = Eval(Eval::MATE_0_VAL + 1000);

    /// An evaluation where Black has won the game by mate.
    pub const BLACK_MATE: Eval = Eval(-Eval::MATE_0_VAL);

    /// An evaluation where White has won the game by mate.
    pub const WHITE_MATE: Eval = Eval(Eval::MATE_0_VAL);

    /// The evaluation of a drawn position.
    pub const DRAW: Eval = Eval(0);

    /// The internal evaluation of a mate in 0 for White (i.e. White made the
    /// mating move on the previous ply).
    const MATE_0_VAL: i16 = 30_000;

    /// The highest value of a position which is not a mate.
    const MATE_CUTOFF: i16 = 29_000;

    /// The value of one pawn.
    const PAWN_VALUE: i16 = 100;

    #[inline(always)]
    /// Get an evaluation equivalent to the given pawn value.
    pub fn pawns(x: f64) -> Eval {
        Eval((x * Eval::PAWN_VALUE as f64) as i16)
    }

    #[inline(always)]
    /// Construct an `Eval` with the given value in centipawns.
    pub const fn centipawns(x: i16) -> Eval {
        Eval(x)
    }

    #[inline(always)]
    /// Create an `Eval` based on the number of half-moves required for White to
    /// mate. `-Eval::mate_in(n)` will give Black to mate in the number of
    /// plies.
    pub const fn mate_in(nplies: u16) -> Eval {
        Eval(Eval::MATE_0_VAL - (nplies as i16))
    }

    #[inline(always)]
    /// Step this evaluation back in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one closer to 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF+1)`, this will result in undefined
    /// behavior.
    /// # Examples
    /// ```
    /// use fiddler_engine::evaluate::Eval;
    /// let current_eval = Eval::mate_in(0);
    /// let previous_ply_eval = current_eval.step_back();
    /// assert_eq!(previous_ply_eval, Eval::mate_in(1));
    /// ```
    pub const fn step_back(&self) -> Eval {
        Eval(self.0 - self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline(always)]
    /// Step this evaluation forward in time one move. "normal" evaluations will
    /// not be changed, but mates will be moved one further from 0. When the
    /// evaluation is `+/-(Eval::MATE_CUTOFF)`, this will result in undefined
    /// behavior.
    pub const fn step_forward(&self) -> Eval {
        Eval(self.0 + self.0 / (Eval::MATE_CUTOFF + 1))
    }

    #[inline(always)]
    /// Is this evaluation a mate (i.e. a non-normal evaluation)?
    pub const fn is_mate(&self) -> bool {
        self.0 > Eval::MATE_CUTOFF || self.0 < -Eval::MATE_CUTOFF
    }

    /// Get the number of moves until a mated position, assuming perfect play.
    /// # Examples
    /// ```
    /// use fiddler_engine::evaluate::Eval;
    /// let ev1 = Eval::pawns(2.5);
    /// let ev2 = Eval::mate_in(3);
    /// assert_eq!(ev1.moves_to_mate(), None);
    /// assert_eq!(ev2.moves_to_mate(), Some(2));
    /// ```
    pub const fn moves_to_mate(&self) -> Option<u8> {
        match self.is_mate() {
            true => {
                if self.0 > 0 {
                    // white to mate
                    Some(((Eval::MATE_0_VAL - self.0 + 1) / 2) as u8)
                } else {
                    // black to mate
                    Some(((Eval::MATE_0_VAL + self.0 + 1) / 2) as u8)
                }
            }
            false => None,
        }
    }

    #[inline(always)]
    /// Get the value in centipawns of this evaluation. Will return a number
    /// with magnitude greater than 29000 for mates.
    pub const fn centipawn_val(&self) -> i16 {
        self.0
    }

    #[inline(always)]
    /// Get the value in floating-point pawns of this evaluation.
    pub fn float_val(&self) -> f32 {
        (self.0 as f32) / 100.
    }

    #[inline(always)]
    /// Put this evaluation into the perspective of the given player.
    /// In essence, if the player is Black, the evaluation will be inverted, but
    /// if the player is White, the evaluation will remain the same. This
    /// function is an involution, meaning that calling it twice with the same
    /// player will yield the original evaluation.
    pub const fn in_perspective(&self, player: Color) -> Eval {
        match player {
            Color::White => *self,
            Color::Black => Eval(-self.0),
        }
    }
}

impl Score {
    /// The score for a position which is completely drawn.
    pub const DRAW: Score = Score::centipawns(0, 0);

    /// Create a new `Score` by composing two evaluations together.
    pub const fn new(mg: Eval, eg: Eval) -> Score {
        Score { mg, eg }
    }

    /// Create a `Score` directly as a pair of centipawn values.
    pub const fn centipawns(mg: i16, eg: i16) -> Score {
        Score::new(Eval::centipawns(mg), Eval::centipawns(eg))
    }

    /// Blend the midgame and endgame
    pub fn blend(&self, phase: f32) -> Eval {
        // in test mode, require that the phase is between 0 and 1
        debug_assert!(0. <= phase);
        debug_assert!(phase <= 1.);

        self.mg * phase + self.eg * (1. - phase)
    }
}

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 > Eval::MATE_CUTOFF {
            // white to mate
            write!(f, "+M{:.0}", (Eval::MATE_0_VAL - self.0 + 1) / 2)?;
        } else if self.0 < -Eval::MATE_CUTOFF {
            // black to mate
            write!(f, "-M{:.0}", (Eval::MATE_0_VAL + self.0 + 1) / 2)?;
        } else if self.0 == 0 {
            // draw
            write!(f, "00.00")?;
        } else {
            // normal eval
            write!(f, "{:+2.2}", self.0 as f32 / Eval::PAWN_VALUE as f32)?;
        }
        Ok(())
    }
}

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.mg, self.eg)
    }
}

impl Mul<u32> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: u32) -> Self::Output {
        Eval(self.0 * rhs as i16)
    }
}

impl Mul<i16> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: i16) -> Self::Output {
        Eval(self.0 * rhs)
    }
}

impl Mul<i8> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: i8) -> Self::Output {
        Eval(self.0 * (rhs as i16))
    }
}

impl Mul<f32> for Eval {
    type Output = Self;
    #[inline(always)]
    fn mul(self, rhs: f32) -> Self::Output {
        Eval((self.0 as f32 * rhs) as i16)
    }
}

impl MulAssign<i16> for Eval {
    #[inline(always)]
    fn mul_assign(&mut self, rhs: i16) {
        self.0 *= rhs;
    }
}

impl AddAssign<Eval> for Eval {
    #[inline(always)]
    fn add_assign(&mut self, rhs: Eval) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Eval> for Eval {
    #[inline(always)]
    fn sub_assign(&mut self, rhs: Eval) {
        self.0 -= rhs.0;
    }
}

impl Add<Eval> for Eval {
    type Output = Self;
    #[inline(always)]
    fn add(self, rhs: Eval) -> Eval {
        Eval(self.0 + rhs.0)
    }
}

impl Sub<Eval> for Eval {
    type Output = Self;
    #[inline(always)]
    fn sub(self, rhs: Eval) -> Eval {
        Eval(self.0 - rhs.0)
    }
}

impl Neg for Eval {
    type Output = Self;
    #[inline(always)]
    fn neg(self) -> Eval {
        Eval(-self.0)
    }
}

impl AddAssign<Score> for Score {
    fn add_assign(&mut self, rhs: Score) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl SubAssign<Score> for Score {
    fn sub_assign(&mut self, rhs: Score) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl Add<Score> for Score {
    type Output = Self;

    fn add(self, rhs: Score) -> Self::Output {
        Score::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }
}

impl Sub<Score> for Score {
    type Output = Self;

    fn sub(self, rhs: Score) -> Self::Output {
        Score::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl Mul<i8> for Score {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self::Output {
        Score::new(self.mg * rhs, self.eg * rhs)
    }
}

impl Mul<u32> for Score {
    type Output = Self;

    fn mul(self, rhs: u32) -> Self::Output {
        Score::new(self.mg * rhs, self.eg * rhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fiddler_base::movegen::ALL;

    fn delta_helper(fen: &str) {
        let mut g = ScoredGame::from_fen(fen).unwrap();
        for (m, tag) in g.get_moves::<ALL>() {
            g.make_move(m, tag);
            // println!("{g}");
            assert_eq!(ScoreTag::init_cookie(g.board()), *g.cookie());
            g.undo().unwrap();
        }
    }

    #[test]
    fn delta_captures() {
        delta_helper("r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8");
    }

    #[test]
    fn delta_promotion() {
        // undoubling capture promotion is possible
        delta_helper("r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13");
    }

    #[test]
    fn certainly_endgame() {
        assert_eq!(
            phase_of(&Board::from_fen("8/5k2/6p1/8/5PPP/8/pb3P2/6K1 w - - 0 37").unwrap()),
            0.0
        );
    }

    #[test]
    fn certainly_midgame() {
        assert_eq!(phase_of(&Board::default()), 1.0);
    }

    #[test]
    /// Test that stepping forward a normal evaluation will make no changes.
    fn step_forward_draw() {
        assert_eq!(Eval(0), Eval(0).step_forward());
    }

    #[test]
    /// Test that stepping backward a normal evaluation will make no changes.
    fn step_backward_draw() {
        assert_eq!(Eval(0), Eval(0).step_back());
    }

    #[test]
    /// Test that stepping forward the highest non-mate will make no change.
    fn step_forward_highest_non_mate() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF),
            Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    /// Test that stepping backward the highest non-mate will make no change.
    fn step_bacwkard_highest_non_mate() {
        assert_eq!(Eval(Eval::MATE_CUTOFF), Eval(Eval::MATE_CUTOFF).step_back());
    }

    #[test]
    /// Test that stepping forward the lowest non-mate will make no change.
    fn step_forward_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_forward()
        );
    }

    #[test]
    /// Test that stepping backward the lowest non-mate will make no change.
    fn step_bacwkard_lowest_non_mate() {
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF),
            -Eval(Eval::MATE_CUTOFF).step_back()
        );
    }

    #[test]
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
    fn step_forward_tighmates() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF + 2),
            Eval(Eval::MATE_CUTOFF + 1).step_forward()
        );
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF + 2),
            -Eval(Eval::MATE_CUTOFF + 1).step_forward()
        );
    }
    #[test]
    /// Test that stepping forward the mates closest to being a normal
    /// evaluation will correctly step forward.
    fn step_backward_tighmates() {
        assert_eq!(
            Eval(Eval::MATE_CUTOFF + 1),
            Eval(Eval::MATE_CUTOFF + 2).step_back()
        );
        assert_eq!(
            -Eval(Eval::MATE_CUTOFF + 1),
            -Eval(Eval::MATE_CUTOFF + 2).step_back()
        );
    }

    #[test]
    /// Test that multiplying scores doesn't screw up and cause weird overflows.
    fn score_multiply() {
        let s1 = Score::centipawns(-289, 0);
        let s2 = Score::centipawns(-289, -200);
        assert_eq!(s1 * 2i8, Score::centipawns(-578, 0));
        assert_eq!(s2 * 2i8, Score::centipawns(-578, -400));

        assert_eq!(s1 * -2i8, Score::centipawns(578, 0));
        assert_eq!(s2 * -2i8, Score::centipawns(578, 400));
    }
}
