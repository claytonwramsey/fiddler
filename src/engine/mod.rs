use crate::Game;
use crate::Move;
use crate::MoveGenerator;

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{AddAssign, Mul, SubAssign};

pub mod greedy;
pub mod minimax;
pub mod positional;

pub type EvaluationFn = fn(&mut Game, &MoveGenerator) -> Eval;

/**
 * An `Engine` is something that can evaluate a `Game`, and give moves which it
 * thinks are good. All the public methods require it to be mutable so that the
 * engine can alter its internal state (such as with transposition tables) to
 * update its internal data.
 */
pub trait Engine {
    /**
     * Evaluate the position of the given game. `g` is only given as mutable to
     * allow this method access to the ability to make and undo moves, but `g`
     * should be the same before and after its use.
     */
    fn evaluate(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Eval;

    /**
     * Get what this engine believes to be the best move in the given position.
     * `g` is only given as mutable to allow this method access to the ability
     * to make and undo moves, but `g` should be the same before and after its
     * use.
     */
    fn get_best_move(&mut self, g: &mut Game, mgen: &MoveGenerator) -> Move {
        self.get_evals(g, mgen)
            .into_iter()
            .max_by(|a, b| a.1.cmp(&b.1))
            .map(|(k, _)| k)
            .unwrap_or(Move::BAD_MOVE)
    }

    /**
     * Get the evaluation of each move in this position. `g` is only given as
     * mutable to allow this method access to the ability to make and undo
     * moves, but `g` should be the same before and after its use.
     */
    fn get_evals(&mut self, g: &mut Game, mgen: &MoveGenerator) -> HashMap<Move, Eval> {
        let moves = g.get_moves(mgen);
        let mut evals = HashMap::new();
        for m in moves {
            g.make_move(m);
            let ev = self.evaluate(g, mgen);

            //this should never fail since we just made a move, but who knows?
            if let Ok(_) = g.undo() {
                evals.insert(m, ev);
            } else {
                println!("somehow, undoing failed on a game");
            }
        }
        return evals;
    }
}

const MATE_0_VAL: i32 = 1_000_000;

const MATE_CUTOFF: i32 = 999_000;

const PAWN_VALUE: i32 = 1_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
/**
 * A wrapper for the evaluation of a position.
 * The higher an evaluation is, the better the position is for White. An
 * evaluation of 0 is a draw.
 *
 * Internally, the i32 represents an integer. The integer value is 1/1000 of a
 * pawn (so if the internal value is +2000, the position is +2 pawns for White).
 *
 * Values >= 999,000 are reserved for mates. 1,000,000 is White to mate in
 * 0 (i.e. White has won the game), 999,999 is White to mate in 1 (White will
 * play their move and mate), 999,998 is White to mate in 1, with Black to
 * move (Black will play their move, then White will play their move to mate)
 * and so on. Values of <= -999,000 are reserved for black mates, likewise.
 */
pub struct Eval(i32);

impl Eval {
    pub const MIN: Eval = Eval(-MATE_0_VAL - 1);
    pub const MAX: Eval = Eval(MATE_0_VAL + 1);

    pub const BLACK_MATE: Eval = Eval(-MATE_0_VAL);
    pub const WHITE_MATE: Eval = Eval(MATE_0_VAL);

    #[inline]
    /**
     * Get an evaluation equivalent to the given pawn value.
     */
    pub fn pawns(x: f64) -> Eval {
        Eval((x / (PAWN_VALUE as f64)) as i32)
    }

    #[inline]
    #[allow(dead_code)]
    /**
     * Create an Eval based on the number of half-moves required for White to mate. -mate_in(n) will give Black to mate in the number of plies.
     */
    pub fn mate_in(nplies: u16) -> Eval {
        Eval(MATE_0_VAL - (nplies as i32))
    }

    #[inline]
    /**
     * Step this evaluation back in time one move. "normal" evaluations will
     * not be changed, but mates will be moved one closer to 0.
     */
    pub fn step_back(&self) -> Eval {
        if self.0 > MATE_CUTOFF {
            return Eval(self.0 - 1);
        } else if self.0 < -MATE_CUTOFF {
            return Eval(self.0 + 1);
        }
        *self
    }

    #[inline]
    /**
     * Is this evaluation a mate (i.e. a non-normal evaluation)?
     */
    pub fn is_mate(&self) -> bool {
        self.0 > MATE_CUTOFF || self.0 < -MATE_CUTOFF
    }
}

impl Display for Eval {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if self.0 > MATE_CUTOFF {
            // white to mate
            write!(f, "+M{:2.0}", (MATE_0_VAL - self.0 + 1) / 2)?;
        } else if self.0 < -MATE_CUTOFF {
            // black to mate
            write!(f, "-M{:2.0}", (MATE_0_VAL + self.0 + 1) / 2)?;
        } else if self.0 == 0 {
            // draw
            write!(f, "00.00")?;
        } else {
            // normal eval
            write!(f, "{:+2.2}", self.0 as f32 / PAWN_VALUE as f32)?;
        }
        Ok(())
    }
}

impl Mul<u32> for Eval {
    type Output = Self;
    #[inline]
    fn mul(self, rhs: u32) -> Self::Output {
        Eval(self.0 * rhs as i32)
    }
}

impl AddAssign<Eval> for Eval {
    fn add_assign(&mut self, rhs: Eval) {
        self.0 += rhs.0;
    }
}

impl SubAssign<Eval> for Eval {
    fn sub_assign(&mut self, rhs: Eval) {
        self.0 -= rhs.0;
    }
}
