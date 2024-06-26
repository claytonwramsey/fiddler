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

//! Static evaluation of positions.
//!
//! Of all the parts of a chess engine, static evaluation is arguably the most important.
//! Every leaf of the search is statically evaluated, and based on the comparisons of each
//! evaluation, the full minimax search is achieved.
//!
//! Fiddler uses a classical approach to static evaluation: the final evaluation is the sum of a
//! number of rules.
//! Each rule contributes a quantity to the evaluation.
//!
//! Also like other engines, Fiddler uses a "tapered" evaluation: rules are given different weights
//! at different phases of the game.
//! To prevent sharp changes in evaluation as the phase blends, a "midgame" and "endgame" evaluation
//! is created, and then the final evaluation is a linear combination of those two.
//!
//! More uniquely, Fiddler is obsessed with cumulative evaluation.
//! Often, learning facts about a board is lengthy and difficult
//! (in computer time - it takes nanoseconds in wall time).
//! However, it is generally easy to guess what effect a move will have on the
//! static evaluation of a position.
//! We therefore tag moves with their effect on the evaluation,
//! allowing us to cheaply evaluate the final leaf position.

use std::{
    fmt::{Display, Formatter},
    ops::{Add, AddAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

use crate::base::{game::Game, Bitboard, Color, Move, Piece};

pub mod material;
pub mod mobility;
pub mod pst;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
#[repr(C)]
/// A wrapper for the evaluation of a position.
/// The higher an evaluation is, the better the position is for White.
/// An evaluation of 0 is a draw.
/// Internally, an evaluation is a 16-bit signed interger.
/// The integer value is 1/1000 of a pawn (so if the internal value is +2000, the position is +2
/// pawns for White).
///
/// Values > 29,000 are reserved for mates.
/// 30,000 is White to mate in 0 (i.e. White has won the game), 29,999 is White to mate in 1 (White
/// will play their move and mate), 29,998 is White to mate in 1, with Black to move (Black will
/// play their move, then White will play their move to mate) and so on.
/// Values of < -29,000 are reserved for black mates, likewise.
///
/// # Examples
///
/// ```
/// use fiddler::engine::evaluate::Eval;
/// let mate_eval = Eval::mate_in(3);
/// let draw_eval = Eval::DRAW;
/// assert!(mate_eval > draw_eval);
/// ```
pub struct Eval(i16);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
/// A `Score` is a pair of two [`Eval`]s - one for the midgame and one for the endgame.
/// The values inside of a `Score` should never be mate values.
pub struct Score {
    /// The midgame-only evaluation of a position.
    pub mg: Eval,
    /// The endgame-only evaluation of a position.
    pub eg: Eval,
}

#[must_use]
/// Get the change in the quantity of mid-game non-pawn material that a move `m` would cause on a
/// game `g`.
pub fn mg_npm_delta(m: Move, g: &Game) -> Eval {
    let mut mg_npm_delta = m
        .promote_type()
        .map_or(Eval::DRAW, |pt| material::value(pt).mg);
    if g.is_move_capture(m) {
        mg_npm_delta -= match g[m.destination()] {
            None | Some((Piece::Pawn, _)) => Eval::DRAW,
            Some((pt, _)) => material::value(pt).mg,
        }
    };
    mg_npm_delta
}

#[must_use]
/// Compute the quantity of mid-game non-pawn material on a game.
pub fn mg_npm(g: &Game) -> Eval {
    let mut total = Eval::DRAW;
    for pt in Piece::PROMOTING {
        total += material::value(pt).mg * g.by_piece(pt).len();
    }
    total
}

#[must_use]
/// Get the expected change in the evaluation of the position `g` by playing `m`, in perspective of
/// the player to move.
/// Does not include leaf-related evaluation information.
pub fn eval_nl_delta(m: Move, g: &Game) -> Score {
    material::delta(g, m) + pst::delta(g, m)
}

#[must_use]
/// Initialize the cumulatively-evaluated score for a position.
/// Returns a score in perspective of `g`'s player to move.
pub fn cumulative_init(g: &Game) -> Score {
    material::evaluate(g) + pst::evaluate(g)
}

/// The cutoff for pure midgame material.
pub const MG_LIMIT: Eval = Eval::centipawns(2172);

/// The cutoff for pure endgame material.
pub const EG_LIMIT: Eval = Eval::centipawns(1263);

/// Mask containing ones along the A file.
/// Bitshifting left by a number from 0 through 7 will cause it to become a mask for each file.
const A_FILE_MASK: Bitboard = Bitboard::new(0x0101_0101_0101_0101);

/// The value of having your own pawn doubled.
pub const DOUBLED_PAWN_VALUE: Score = Score::centipawns(-21, -21);
/// The value of having a rook with no same-colored pawns in front of it which are not advanced past
/// the 3rd rank.
pub const OPEN_ROOK_VALUE: Score = Score::centipawns(27, 29);

/// The value of having the right to castle kingside.
pub const KINGSIDE_CASTLE_VALUE: Score = Score::centipawns(0, 0);

/// The value of having the right to castle queenside.
pub const QUEENSIDE_CASTLE_VALUE: Score = Score::centipawns(1, 1);

#[must_use]
#[allow(clippy::module_name_repetitions)]
/// Evaluate a leaf position on a game whose cumulative values have been computed correctly.
pub fn leaf_evaluate(g: &Game, cumulative_score: Score, phase: f32) -> Eval {
    let cum = match g.meta().player {
        Color::White => cumulative_score,
        Color::Black => -cumulative_score,
    };
    (leaf_rules(g) + cum).blend(phase)
}

/// Get the score gained from evaluations that are only performed at the leaf.
fn leaf_rules(g: &Game) -> Score {
    let mut score = Score::DRAW;
    let meta = g.meta();

    // Add gains from castling rights
    let kingside_net = i8::from(meta.castle_rights.kingside(meta.player))
        - i8::from(meta.castle_rights.kingside(!meta.player));
    score += KINGSIDE_CASTLE_VALUE * kingside_net;

    let queenside_net = i8::from(meta.castle_rights.queenside(meta.player))
        - i8::from(meta.castle_rights.queenside(!meta.player));
    score -= QUEENSIDE_CASTLE_VALUE * queenside_net;

    // Add losses due to doubled pawns
    score += DOUBLED_PAWN_VALUE * net_doubled_pawns(g);

    // Add gains from open rooks
    score += OPEN_ROOK_VALUE * net_open_rooks(g);
    score += mobility::evaluate(g);

    score
}

#[must_use]
/// Count the number of "open" rooks (i.e., those which are not blocked by unadvanced pawns) in a
/// position.
/// The number is a net value, so it will be negative if Black has more open rooks than White.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use fiddler::{base::game::Game, engine::evaluate::net_open_rooks};
///
/// assert_eq!(net_open_rooks(&Game::new()), 0);
/// assert_eq!(
///     net_open_rooks(&Game::from_fen("5r2/4r3/2k5/8/3K4/8/4p3/4R3 w - - 0 1")?),
///     -1
/// );
/// # Ok(())
/// # }
/// ```
pub fn net_open_rooks(g: &Game) -> i8 {
    // Mask for pawns which are below rank 3 (i.e. on the white half of the board).
    const WHITE_HALF: Bitboard = Bitboard::new(0x0000_0000_FFFF_FFFF);
    // Mask for pawns which are on the black half of the board
    const BLACK_HALF: Bitboard = Bitboard::new(0xFFFF_FFFF_0000_0000);
    let mut net_open_rooks = 0i8;
    let rooks = g.rooks();
    let pawns = g.pawns();
    let white = g.white();
    let black = g.black();

    // count white rooks
    for wrook_sq in rooks & white {
        if wrook_sq.rank() >= 3 {
            net_open_rooks += 1;
            continue;
        }
        let pawns_in_col = (pawns & white) & (A_FILE_MASK << wrook_sq.file());
        let important_pawns = WHITE_HALF & pawns_in_col;
        // check that the forward-most pawn of the important pawns is in front of or behind the rook
        if important_pawns.as_u64().leading_zeros() > (63 - (wrook_sq as u32)) {
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
        let important_pawns = BLACK_HALF & pawns_in_col;
        // check that the lowest-rank pawn that could block the rook is behind the rook
        if important_pawns.trailing_zeros() > brook_sq as u32 {
            net_open_rooks -= 1;
        }
    }

    net_open_rooks
}

#[must_use]
/// Count the number of doubled pawns, in net.
/// For instance, if White had 1 doubled pawn, and Black had 2, this function would return -1.
///
/// # Examples
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// use fiddler::{base::game::Game, engine::evaluate::net_doubled_pawns};
///
/// assert_eq!(net_doubled_pawns(&Game::new()), 0);
/// # Ok(())
/// # }
/// ```
pub fn net_doubled_pawns(g: &Game) -> i8 {
    let pawns = g.pawns();
    let mut net_doubled: i8 = 0;
    // all ones on the A column, shifted left by the col
    let mut col_mask = Bitboard::new(0x0101_0101_0101_0101);
    #[allow(clippy::cast_possible_wrap)]
    for _ in 0..8 {
        let col_pawns = pawns & col_mask;

        net_doubled -= match (g.black() & col_pawns).len() {
            0 => 0,
            x => x as i8 - 1,
        };
        net_doubled += match (g.white() & col_pawns).len() {
            0 => 0,
            x => x as i8 - 1,
        };

        col_mask <<= 1;
    }

    net_doubled
}

#[must_use]
/// Get a blending float describing the current phase of the game.
/// Will range from 0 (full endgame) to 1 (full midgame).
///
/// # Examples
///
/// ```
/// use fiddler::{base::game::Game, engine::evaluate::phase_of};
///
/// assert!(phase_of(&Game::new()).eq(&1.0));
/// ```
pub fn phase_of(g: &Game) -> f32 {
    // amount of non-pawn material in the board, under midgame values
    calculate_phase(mg_npm(g))
}
#[must_use]
/// Get a blending float describing the current phase of the game.
/// Will range from 0 (full endgame) to 1 (full midgame).
/// `mg_npm` is the amount of midgame non-pawn material on the board.
pub fn calculate_phase(mg_npm: Eval) -> f32 {
    let bounded_npm = mg_npm.clamp(EG_LIMIT, MG_LIMIT);

    (EG_LIMIT - bounded_npm).float_val() / (EG_LIMIT - MG_LIMIT).float_val()
}

impl Eval {
    /// An evaluation which is smaller than every other "normal" evaluation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    ///
    /// assert!(Eval::MIN < Eval::BLACK_MATE);
    /// assert!(Eval::MIN < Eval::DRAW);
    /// assert!(Eval::MIN < Eval::WHITE_MATE);
    /// assert!(Eval::MIN < Eval::MAX);
    /// ```
    pub const MIN: Self = Self(-Self::MATE_0_VAL - 1000);

    /// An evaluation which is larger than every other "normal" evaluation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    ///
    /// assert!(Eval::MIN < Eval::MAX);
    /// assert!(Eval::BLACK_MATE < Eval::MAX);
    /// assert!(Eval::DRAW < Eval::MAX);
    /// assert!(Eval::WHITE_MATE < Eval::MAX);
    /// ```
    pub const MAX: Self = Self(Self::MATE_0_VAL + 1000);

    /// An evaluation where Black has won the game by mate.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    ///
    /// assert!(Eval::MIN < Eval::BLACK_MATE);
    /// assert!(Eval::BLACK_MATE < Eval::DRAW);
    /// assert!(Eval::BLACK_MATE < Eval::WHITE_MATE);
    /// assert!(Eval::BLACK_MATE < Eval::MAX);
    /// ```
    pub const BLACK_MATE: Self = Self(-Self::MATE_0_VAL);

    /// An evaluation where White has won the game by mate.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    ///
    /// assert!(Eval::MIN < Eval::WHITE_MATE);
    /// assert!(Eval::BLACK_MATE < Eval::WHITE_MATE);
    /// assert!(Eval::DRAW < Eval::WHITE_MATE);
    /// assert!(Eval::WHITE_MATE < Eval::MAX);
    /// ```
    pub const WHITE_MATE: Self = Self(Self::MATE_0_VAL);

    /// The evaluation of a drawn position.
    pub const DRAW: Self = Self(0);

    /// The internal evaluation of a mate in 0 for White (i.e. White made the mating move on the
    /// previous ply).
    const MATE_0_VAL: i16 = 30_000;

    /// The highest value of a position which is not a mate.
    const MATE_CUTOFF: i16 = 29_000;

    /// The value of one pawn.
    const PAWN_VALUE: i16 = 100;

    #[must_use]
    #[allow(clippy::cast_possible_truncation)]
    /// Get an evaluation equivalent to the given pawn value.
    /// Will round down by the centipawn.
    pub fn pawns(x: f64) -> Self {
        Self((x * f64::from(Self::PAWN_VALUE)) as i16)
    }

    #[must_use]
    /// Construct an `Eval` with the given value in centipawns.
    pub const fn centipawns(x: i16) -> Self {
        Self(x)
    }

    #[must_use]
    /// Create an `Eval` based on the number of half-moves required for White to mate.
    /// `-Eval::mate_in(n)` will give Black to mate in the number of plies.
    pub const fn mate_in(nplies: u8) -> Self {
        Self(Self::MATE_0_VAL - (nplies as i16))
    }

    #[must_use]
    /// Step this evaluation back in time by `n` moves.
    /// If the evaluation is within `n` steps of the mate cutoff, this will result in weird
    /// behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    /// let current_eval = Eval::mate_in(0);
    /// let previous_ply_eval = current_eval.step_back_by(1);
    /// assert_eq!(previous_ply_eval, Eval::mate_in(1));
    /// ```
    pub fn step_back_by(self, n: u8) -> Self {
        if self.0 < -Self::MATE_CUTOFF {
            Self(self.0 + i16::from(n))
        } else if Self::MATE_CUTOFF < self.0 {
            Self(self.0 - i16::from(n))
        } else {
            self
        }
    }

    #[must_use]
    /// Step this evaluation forward by a given number of steps.
    /// If the evaluation is within `n` steps of the mate cutoff, this will result in weird
    /// behavior.
    pub fn step_forward_by(self, n: u8) -> Self {
        if self.0 < -Self::MATE_CUTOFF {
            Self(self.0 - i16::from(n))
        } else if Self::MATE_CUTOFF < self.0 {
            Self(self.0 + i16::from(n))
        } else {
            self
        }
    }

    #[must_use]
    /// Is this evaluation a mate (i.e. a non-normal evaluation)?
    pub const fn is_mate(self) -> bool {
        self.0 > Self::MATE_CUTOFF || self.0 < -Self::MATE_CUTOFF
    }

    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    /// Get the number of moves until a mated position, assuming perfect play.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::engine::evaluate::Eval;
    /// let ev1 = Eval::pawns(2.5);
    /// let ev2 = Eval::mate_in(3);
    /// assert_eq!(ev1.moves_to_mate(), None);
    /// assert_eq!(ev2.moves_to_mate(), Some(2));
    /// ```
    pub fn moves_to_mate(self) -> Option<u8> {
        self.is_mate().then_some(if self.0 > 0 {
            // white to mate
            ((Self::MATE_0_VAL - self.0 + 1) / 2) as u8
        } else {
            // black to mate
            ((Self::MATE_0_VAL + self.0 + 1) / 2) as u8
        })
    }

    #[must_use]
    /// Get the value in centipawns of this evaluation.
    /// Will return a number with magnitude greater than 29000 for mates.
    pub const fn centipawn_val(self) -> i16 {
        self.0
    }

    #[must_use]
    /// Get the value in floating-point pawns of this evaluation.
    pub fn float_val(self) -> f32 {
        f32::from(self.0) / 100.
    }

    #[must_use]
    /// Put this evaluation into the perspective of the given player.
    /// In essence, if the player is Black, the evaluation will be inverted, but if the player is
    /// White, the evaluation will remain the same.
    /// This function is an involution, meaning that calling it twice with the same player will
    /// yield the original evaluation.
    pub const fn in_perspective(self, player: Color) -> Self {
        match player {
            Color::White => self,
            Color::Black => Self(-self.0),
        }
    }
}

impl Score {
    /// The score for a position which is completely drawn.
    pub const DRAW: Self = Self::centipawns(0, 0);

    #[must_use]
    /// Create a new `Score` by composing two evaluations together.
    pub const fn new(mg: Eval, eg: Eval) -> Self {
        Self { mg, eg }
    }

    #[must_use]
    /// Create a `Score` directly as a pair of centipawn values.
    pub const fn centipawns(mg: i16, eg: i16) -> Self {
        Self::new(Eval::centipawns(mg), Eval::centipawns(eg))
    }

    #[must_use]
    /// Blend the midgame and endgame
    pub fn blend(self, phase: f32) -> Eval {
        // in test mode, require that the phase is between 0 and 1
        debug_assert!(0. <= phase);
        debug_assert!(phase <= 1.);

        self.mg * phase + self.eg * (1. - phase)
    }
}

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 > Self::MATE_CUTOFF {
            // white to mate
            write!(f, "+M{:.0}", (Self::MATE_0_VAL - self.0 + 1) / 2)?;
        } else if self.0 < -Self::MATE_CUTOFF {
            // black to mate
            write!(f, "-M{:.0}", (Self::MATE_0_VAL + self.0 + 1) / 2)?;
        } else if self.0 == 0 {
            // draw
            write!(f, "00.00")?;
        } else {
            // normal eval
            write!(
                f,
                "{:+2.2}",
                f32::from(self.0) / f32::from(Self::PAWN_VALUE)
            )?;
        }
        Ok(())
    }
}

impl Display for Score {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.mg, self.eg)
    }
}

impl Mul<u8> for Eval {
    type Output = Self;
    fn mul(self, rhs: u8) -> Self::Output {
        Self(self.0 * i16::from(rhs))
    }
}

impl Mul<i16> for Eval {
    type Output = Self;
    fn mul(self, rhs: i16) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Mul<i8> for Eval {
    type Output = Self;
    fn mul(self, rhs: i8) -> Self::Output {
        Self(self.0 * i16::from(rhs))
    }
}

impl Mul<f32> for Eval {
    type Output = Self;
    #[allow(clippy::cast_possible_truncation)]
    fn mul(self, rhs: f32) -> Self::Output {
        Self((f32::from(self.0) * rhs) as i16)
    }
}

impl MulAssign<i16> for Eval {
    fn mul_assign(&mut self, rhs: i16) {
        self.0 *= rhs;
    }
}

impl AddAssign<Self> for Eval {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Self> for Eval {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

impl Add for Eval {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self(self.0 + rhs.0)
    }
}

impl Sub for Eval {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self(self.0 - rhs.0)
    }
}

impl Neg for Eval {
    type Output = Self;
    fn neg(self) -> Self {
        Self(-self.0)
    }
}

impl AddAssign for Score {
    fn add_assign(&mut self, rhs: Self) {
        self.mg += rhs.mg;
        self.eg += rhs.eg;
    }
}

impl SubAssign<Self> for Score {
    fn sub_assign(&mut self, rhs: Self) {
        self.mg -= rhs.mg;
        self.eg -= rhs.eg;
    }
}

impl Add for Score {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::new(self.mg + rhs.mg, self.eg + rhs.eg)
    }
}

impl Sub<Self> for Score {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::new(self.mg - rhs.mg, self.eg - rhs.eg)
    }
}

impl Neg for Score {
    type Output = Self;
    fn neg(self) -> Self {
        Self {
            mg: -self.mg,
            eg: -self.eg,
        }
    }
}

impl Mul<i8> for Score {
    type Output = Self;

    fn mul(self, rhs: i8) -> Self::Output {
        Self::new(self.mg * rhs, self.eg * rhs)
    }
}

impl Mul<u8> for Score {
    type Output = Self;

    fn mul(self, rhs: u8) -> Self::Output {
        Self::new(self.mg * rhs, self.eg * rhs)
    }
}

#[cfg(test)]
mod tests {
    use crate::base::{
        movegen::{make_move_vec, GenMode},
        Square,
    };

    use super::*;

    /// Helper function to verify that cumulative evaluation is correct.
    fn cumulative_helper(fen: &str) {
        let mut game = Game::from_fen(fen).unwrap();
        let orig_cumulative = cumulative_init(&game);
        for m in make_move_vec::<{ GenMode::All }>(&game) {
            let leaf_cumulative = -eval_nl_delta(m, &game) - orig_cumulative;
            game.make_move(m);
            assert_eq!(cumulative_init(&game), leaf_cumulative);
            game.undo().unwrap();
        }
    }

    #[test]
    fn cumulative_captures() {
        cumulative_helper("r1bq1b1r/ppp2kpp/2n5/3n4/2BPp3/2P5/PP3PPP/RNBQK2R b KQ d3 0 8");
    }

    #[test]
    fn cumulative_promotion() {
        // undoubling capture promotion is possible
        cumulative_helper("r4bkr/pPpq2pp/2n1b3/3n4/2BPp3/2P5/1P3PPP/RNBQK2R w KQ - 1 13");
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn certainly_endgame() {
        assert_eq!(
            phase_of(&Game::from_fen("8/5k2/6p1/8/5PPP/8/pb3P2/6K1 w - - 0 37").unwrap()),
            0.0
        );
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn certainly_midgame() {
        assert_eq!(phase_of(&Game::default()), 1.0);
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

    #[test]
    fn e2e4_delta() {
        let delta = eval_nl_delta(Move::new(Square::E2, Square::E4), &Game::new()).blend(1.0);
        println!("{delta}");
        let b2b4_delta = eval_nl_delta(Move::new(Square::B2, Square::B4), &Game::new()).blend(1.0);
        println!("{b2b4_delta}");

        assert!(delta > Eval::DRAW);
    }
}
