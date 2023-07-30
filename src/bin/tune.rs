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

//! The tuner for the Fiddler chess engine.
//! This file exists to create a binary which can be used to generate weights from an annotated EPD
//! file.
//!
//! The tuner operates by using gradient descent on logistic regression to classify the results of a
//! given position.

#![warn(clippy::pedantic)]

use std::{
    env,
    fs::File,
    io::{BufRead, BufReader},
    ops::SubAssign,
    thread::scope,
    time::Instant,
};

use fiddler::{
    base::{
        game::Game,
        movegen::{bishop_moves, rook_moves, KING_MOVES, KNIGHT_MOVES, PAWN_ATTACKS},
        Color, Piece, Square,
    },
    engine::evaluate::{
        material,
        mobility::{ATTACKS_VALUE, MAX_MOBILITY},
        net_doubled_pawns, net_open_rooks,
        pst::PST,
        safety::{attack_units, UNIT_SCORES},
        DOUBLED_PAWN_VALUE, EG_LIMIT, KINGSIDE_CASTLE_VALUE, MG_LIMIT, OPEN_ROOK_VALUE,
        QUEENSIDE_CASTLE_VALUE,
    },
};

#[allow(clippy::similar_names, clippy::result_unit_err)]
/// Run the main training function.
///
/// The first command line argument must the the path of the file containing training data.
///
/// # Errors
///
/// This function will return an error if the EPD training data does not exist or cannot be parsed.
pub fn main() -> Result<(), i32> {
    let args: Vec<String> = env::args().collect();
    // first argument is the name of the binary
    let path_str = match args.len() {
        0 | 1 => {
            eprintln!("Error: path to a labeled EPD file must be given");
            return Err(-1);
        }
        2 => &args[1],
        _ => {
            eprintln!("Warning: extraneous arguments are being ignored");
            &args[1]
        }
    };
    let mut weights = load_weights();
    let value_learn_rate = 0.1;
    // The learn rate for the midgame/endgame cutoffs.
    let cutoff_learn_rate = value_learn_rate;

    let nthreads = 12;
    let tic = Instant::now();

    // construct the datasets.
    // Outer vector: each element for one datum
    // Inner vector: each element for one feature-quantity pair
    let Ok(input_sets) = extract_epd(path_str) else {
        eprintln!("Error: unable to parse EPD");
        return Err(-2);
    };

    let toc = Instant::now();
    println!("extracted data in {} secs", (toc - tic).as_secs());
    for i in 0..10_000 {
        println!("iteration {i}...");
        let (new_weights, _) = train_step(
            &input_sets,
            &weights,
            value_learn_rate,
            cutoff_learn_rate,
            nthreads,
        );
        weights = new_weights;
    }

    print_weights(&weights);

    Ok(())
}

#[derive(Debug)]
/// Extracted features from a board for gradient descent.
struct BoardFeatures {
    /// The total (not net) quantities of each piece type on the board.
    piece_counts: [f32; Piece::NUM - 1],
    /// The net rule counts for pieces on the board.
    /// Includes material counts.
    rules: Vec<(usize, f32)>,
}

#[derive(Clone)]
/// Weights for an evaulation.
/// These values will be gradient-descended on.
struct Weights {
    /// The cutoffs for blending bedween midgame and endgame.
    phase_cutoffs: (f32, f32),
    /// The values of each rule (such as square occupancy, mobility, or handmade rules).
    /// The first five entries in `rule_values` must always be the material values.
    rule_values: Vec<(f32, f32)>,
}

/// The gradient of the error with respect to the weights has the same dimension as weights.
type GradWeights = Weights;

/// Expand an EPD file into a set of features that can be used for training.
fn extract_epd(location: &str) -> Result<Vec<(BoardFeatures, f32)>, Box<dyn std::error::Error>> {
    let file = File::open(location)?;
    let reader = BufReader::new(file);
    let mut data = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let mut split_line = line.split("c9 \"");
        // first part of the split is the FEN, second is the score, last is just a semicolon
        // convert FEN to EPD description of the position
        let fen = split_line.next().ok_or("no FEN given")?.to_string() + "0 1 ";
        let g = Game::from_fen(&fen)?;
        let features = extract(&g);
        let score_str = split_line
            .next()
            .unwrap() //.ok_or("no result given")?
            .split('\"')
            .next()
            .unwrap(); // .ok_or("no result given")?;
        let score = match score_str {
            "1/2-1/2" => 0.5,
            "0-1" => 0.,
            "1-0" => 1.,
            _ => Err("unknown score string")?,
        };
        data.push((features, score));
    }

    Ok(data)
}

#[allow(clippy::similar_names, clippy::cast_precision_loss)]
/// Perform one epoch of gradient descent training.
///
/// Inputs:
/// * `inputs`: a vector containing the input vector and the expected evaluation.
/// * `weights`: the weight vector to train on.
/// * `learn_rate`: a coefficient on the speed at which the engine learns.
/// * `nthreads`: the number of threads to parallelize over.
///
/// Each element of `inputs` must be the same length as `weights`.
fn train_step(
    inputs: &[(BoardFeatures, f32)],
    weights: &Weights,
    value_learn_rate: f32,
    cutoff_learn_rate: f32,
    nthreads: usize,
) -> (Weights, f32) {
    let tic = Instant::now();
    let chunk_size = inputs.len() / nthreads;
    let mut new_weights = weights.clone();
    let mut sum_se = 0.;
    scope(|s| {
        let mut grads = Vec::new();
        for thread_id in 0..nthreads {
            // start the parallel work
            let start = chunk_size * thread_id;
            grads.push(s.spawn(move || train_thread(&inputs[start..][..chunk_size], weights)));
        }
        for grad_handle in grads {
            let (mut sub_grad, se) = grad_handle.join().unwrap();
            sum_se += se;
            sub_grad.scale_by(
                value_learn_rate / inputs.len() as f32,
                cutoff_learn_rate / inputs.len() as f32,
            );
            new_weights -= sub_grad;
        }
    });
    let toc = Instant::now();
    let mse = sum_se / inputs.len() as f32;
    println!(
        "{} nodes in {:?}: {:.0} nodes/sec; mse {}",
        inputs.len(),
        (toc - tic),
        inputs.len() as f32 / (toc - tic).as_secs_f32(),
        mse
    );

    (new_weights, mse)
}

/// Construct the gradient vector for a subset of the input data.
/// Returns the sum of the squared error across this epoch.
fn train_thread(input: &[(BoardFeatures, f32)], weights: &Weights) -> (Weights, f32) {
    let mut grad = Weights::zero(weights);
    let mut sum_se = 0.;
    for (features, sigm_expected) in input {
        let sigm_eval = sigmoid(weights.evaluate(features));
        let err = sigm_eval - sigm_expected;
        let coeff = sigm_eval * (1. - sigm_eval) * err;

        sum_se += err * err;
        weights.eval_gradient(&mut grad, coeff, features);
    }

    (grad, sum_se)
}

/// Compute the  sigmoid function of a variable.
/// `beta` is the horizontal scaling of the sigmoid.
///
/// The sigmoid function here is given by the LaTeX expression
/// `f(x) = \frac{1}{1 - \exp (- \beta x)}`.
fn sigmoid(x: f32) -> f32 {
    1. / (1. + (-x).exp())
}

/// Load the weight value constants from the ones defined in the PST evaluation.
fn load_weights() -> Weights {
    let mut weights = Weights {
        phase_cutoffs: (MG_LIMIT.float_val(), EG_LIMIT.float_val()),
        rule_values: Vec::new(),
    };

    // apply material values
    for pt in Piece::NON_KING {
        let val = material::value(pt);
        weights
            .rule_values
            .push((val.mg.float_val(), val.eg.float_val()));
    }

    for pt in Piece::ALL {
        for rank in 0..8 {
            for file in 0..8 {
                let sq_idx = 8 * rank + file;
                let score = PST[pt as usize][sq_idx];
                weights
                    .rule_values
                    .push((score.mg.float_val(), score.eg.float_val()));
            }
        }
    }

    for pt in Piece::ALL {
        for count in 0..MAX_MOBILITY {
            let score = ATTACKS_VALUE[pt as usize][count];
            weights
                .rule_values
                .push((score.mg.float_val(), score.eg.float_val()));
        }
    }

    weights.rule_values.push((
        DOUBLED_PAWN_VALUE.mg.float_val(),
        DOUBLED_PAWN_VALUE.eg.float_val(),
    ));

    weights.rule_values.push((
        OPEN_ROOK_VALUE.mg.float_val(),
        OPEN_ROOK_VALUE.eg.float_val(),
    ));

    weights.rule_values.push((
        KINGSIDE_CASTLE_VALUE.mg.float_val(),
        KINGSIDE_CASTLE_VALUE.eg.float_val(),
    ));

    weights.rule_values.push((
        QUEENSIDE_CASTLE_VALUE.mg.float_val(),
        QUEENSIDE_CASTLE_VALUE.eg.float_val(),
    ));

    for val in UNIT_SCORES {
        weights
            .rule_values
            .push((val.mg.float_val(), val.eg.float_val()));
    }

    weights
}

#[allow(clippy::cast_possible_truncation, clippy::similar_names)]
/// Print out a weights vector so it can be used as code.
fn print_weights(weights: &Weights) {
    let print_rule = |name: &str, idx: usize| {
        println!(
            "pub const {name}: Score = Score::centipawns({}, {});",
            (weights.rule_values[idx].0 * 100.) as i16,
            (weights.rule_values[idx].1 * 100.) as i16,
        );
    };

    println!(
        "pub const MG_LIMIT: Eval = Eval::centipawns({})",
        (weights.phase_cutoffs.0 * 100.) as i16
    );

    println!(
        "pub const EG_LIMIT: Eval = Eval::centipawns({})",
        (weights.phase_cutoffs.1 * 100.) as i16
    );

    let mut offset = 0;
    // print material values
    for pt in Piece::NON_KING {
        let fscore = weights.rule_values[offset + pt as usize];
        println!(
            "Piece::{pt:?} => Score::centipawns({}, {}),",
            (fscore.0 * 100.) as i16,
            (fscore.1 * 100.) as i16
        );
    }

    offset += 5;
    println!("-----");

    // print PST
    println!("pub const PST: Pst = unsafe {{ transmute([");
    for pt in Piece::ALL {
        println!("    [ // {pt}");
        let pt_idx = offset + (64 * pt as usize);
        for rank in 0..8 {
            print!("        ");
            for file in 0..8 {
                let sq = Square::new(rank, file).unwrap();
                let fscore = weights.rule_values[pt_idx + sq as usize];
                let mg_val = (fscore.0 * 100.) as i16;
                let eg_val = (fscore.1 * 100.) as i16;
                print!("({mg_val}, {eg_val}), ");
            }
            println!();
        }
        println!("    ],");
    }
    println!("]) }};");

    offset += 384;
    println!("-----");

    // print mobility
    println!(
        "pub const ATTACKS_VALUE: [[Score; MAX_MOBILITY]; Piece::NUM] = unsafe {{ transmute(["
    );
    for pt in Piece::ALL {
        let pt_idx = offset + MAX_MOBILITY * pt as usize;
        println!("    [ // {pt}");
        for count in 0..MAX_MOBILITY {
            let fscore = weights.rule_values[pt_idx + count];
            println!(
                "        ({}, {}), ",
                (fscore.0 * 100.) as i16,
                (fscore.1 * 100.) as i16
            );
        }
        println!("    ],");
    }
    println!("]) }};");

    offset += 168;
    println!("-----");

    // print potpourri
    print_rule("DOUBLED_PAWN_VALUE", offset);
    print_rule("OPEN_ROOK_VALUE", offset + 1);
    print_rule("KINGSIDE_CASTLE_VALUE", offset + 2);
    print_rule("QUEENSIDE_CASTLE_VALUE", offset + 3);

    offset += 4;

    println!("---");
    println!("pub const UNIT_SCORES: [Score; 256] = unsafe {{ transmute::<[(i16, i16); 256], [Score, 256]>([");
    for i in 0..256 {
        let fscore = weights.rule_values[offset + i];
        println!(
            "    ({}, {}),",
            (fscore.0 * 100.) as i16,
            (fscore.1 * 100.) as i16
        );
    }
    println!("])}};");
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::similar_names
)]
/// Extract a feature vector from a board.
fn extract(g: &Game) -> BoardFeatures {
    let bocc = g.black();
    let wocc = g.white();

    let mut piece_counts = [0.0; Piece::NUM - 1];
    let mut rules = Vec::new();
    // Indices 0..8: non-king piece values
    for pt in Piece::NON_KING {
        let n_white = (g.by_piece(pt) & wocc).len() as i8;
        let n_black = (g.by_piece(pt) & bocc).len() as i8;
        piece_counts[pt as usize] = f32::from(n_white + n_black);
        let net = n_white - n_black;
        if net != 0 {
            rules.push((pt as usize, f32::from(net)));
        }
    }
    let mut offset = 5;

    // Get piece-square quantities
    for pt in Piece::ALL {
        for sq in g.by_piece(pt) {
            let alt_sq = sq.opposite();
            let increment = match (wocc.contains(sq), bocc.contains(alt_sq)) {
                (true, false) => 1.,
                (false, true) => -1.,
                _ => continue, // sparsity requires no entry
            };
            let idx = offset + 64 * pt as usize;
            rules.push((idx, increment));
        }
    }

    // New offset after everything in the PST.
    offset += 384;
    extract_mobility(g, &mut rules, offset);

    // Offset after mobility.
    offset += 168;

    // Doubled pawns
    let doubled_count = net_doubled_pawns(g);
    if doubled_count != 0 {
        rules.push((offset, f32::from(doubled_count)));
    }
    offset += 1;

    // Open rooks
    let open_rook_count = net_open_rooks(g);
    if open_rook_count != 0 {
        rules.push((offset, f32::from(open_rook_count)));
    }
    offset += 1;

    // Add gains from castling rights
    let kingside_net = i8::from(g.meta().castle_rights.kingside(g.meta().player))
        - i8::from(g.meta().castle_rights.kingside(!g.meta().player));
    if kingside_net != 0 {
        rules.push((offset, f32::from(kingside_net)));
    }
    offset += 1;

    let queenside_net = i8::from(g.meta().castle_rights.queenside(g.meta().player));
    if queenside_net != 0 {
        rules.push((offset, f32::from(queenside_net)));
    }
    offset += 1;

    // Extract king safety
    // Get indices into the units value table
    let w_attack_idx = usize::from(attack_units::<{ Color::White }>(g));
    let b_attack_idx = usize::from(attack_units::<{ Color::Black }>(g));
    if w_attack_idx != b_attack_idx {
        rules.push((offset + w_attack_idx, 1.0));
        rules.push((offset + b_attack_idx, -1.0));
    }
    // offset += 256;

    BoardFeatures {
        piece_counts,
        rules,
    }
}

/// Helper function to extract mobility information into the sparse feature vector.
/// Adds 168 new features.
fn extract_mobility(g: &Game, rules: &mut Vec<(usize, f32)>, offset: usize) {
    let white = g.white();
    let black = g.black();
    let not_white = !white;
    let not_black = !black;
    let occupancy = white | black;
    let mut count = [[0i8; MAX_MOBILITY]; Piece::NUM];

    // count knight moves
    let knights = g.knights();
    for sq in knights & white {
        let idx = usize::from((KNIGHT_MOVES[sq as usize] & not_white).len());
        count[Piece::Knight as usize][idx] += 1;
    }
    for sq in knights & black {
        let idx = usize::from((KNIGHT_MOVES[sq as usize] & not_black).len());
        count[Piece::Knight as usize][idx] -= 1;
    }

    // count bishop moves
    let bishops = g.bishops();
    for sq in bishops & white {
        let idx = usize::from((bishop_moves(occupancy, sq) & not_white).len());
        count[Piece::Bishop as usize][idx] += 1;
    }
    for sq in bishops & black {
        let idx = usize::from((bishop_moves(occupancy, sq) & not_black).len());
        count[Piece::Bishop as usize][idx] -= 1;
    }

    // count rook moves
    let rooks = g.rooks();
    for sq in rooks & white {
        let idx = usize::from((rook_moves(occupancy, sq) & not_white).len());
        count[Piece::Rook as usize][idx] += 1;
    }
    for sq in rooks & black {
        let idx = usize::from((rook_moves(occupancy, sq) & not_black).len());
        count[Piece::Rook as usize][idx] -= 1;
    }

    // count queen moves
    let queens = g.queens();
    for sq in queens & white {
        let idx = usize::from(
            ((rook_moves(occupancy, sq) | bishop_moves(occupancy, sq)) & not_white).len(),
        );
        count[Piece::Queen as usize][idx] += 1;
    }
    for sq in rooks & black {
        let idx = usize::from(
            ((rook_moves(occupancy, sq) | bishop_moves(occupancy, sq)) & not_black).len(),
        );
        count[Piece::Queen as usize][idx] -= 1;
    }

    // count net pawn moves
    // pawns can't capture by pushing, so we only examine their capture squares
    let pawns = g.pawns();
    for sq in pawns & white {
        let idx = usize::from((PAWN_ATTACKS[Color::White as usize][sq as usize] & not_white).len());
        count[Piece::Pawn as usize][idx] += 1;
    }
    for sq in pawns & black {
        let idx = usize::from((PAWN_ATTACKS[Color::Black as usize][sq as usize] & not_black).len());
        count[Piece::Pawn as usize][idx] -= 1;
    }

    // king
    let white_king_idx =
        usize::from((KING_MOVES[g.king_sq(Color::White) as usize] & not_white).len());
    count[Piece::King as usize][white_king_idx] += 1;
    let black_king_idx =
        usize::from((KING_MOVES[g.king_sq(Color::Black) as usize] & not_black).len());
    count[Piece::King as usize][black_king_idx] -= 1;

    for pt in Piece::ALL {
        for idx in 0..MAX_MOBILITY {
            let num_mobile = count[pt as usize][idx];
            if num_mobile != 0 {
                let feature_idx = offset + (MAX_MOBILITY * pt as usize + idx);
                rules.push((feature_idx, f32::from(num_mobile)));
            }
        }
    }
}

impl Weights {
    /// Scale a `Weights` by a scaling value on the values and on the cutoffs.
    fn scale_by(&mut self, value_scale: f32, cutoff_scale: f32) {
        for v in &mut self.rule_values {
            v.0 *= value_scale;
            v.1 *= value_scale;
        }

        self.phase_cutoffs.0 *= cutoff_scale;
        self.phase_cutoffs.1 *= cutoff_scale;
    }

    /// Construct a new zero weights with the same dimension for rules as `w`.
    fn zero(w: &Weights) -> Weights {
        Weights {
            phase_cutoffs: (0.0, 0.0),
            rule_values: vec![(0.0, 0.0); w.rule_values.len()],
        }
    }
    /// Get the evaluation of a board with a given set of features.
    fn evaluate(&self, x: &BoardFeatures) -> f32 {
        let midgame_material = self
            .rule_values
            .iter()
            .zip(&x.piece_counts)
            .map(|(&(value, _), &count)| value * count)
            .sum::<f32>();
        let phase = (self.phase_cutoffs.1 - midgame_material)
            / (self.phase_cutoffs.1 - self.phase_cutoffs.0);

        let rule_values = x
            .rules
            .iter()
            .map(|&(idx, value)| {
                (
                    self.rule_values[idx].0 * value,
                    self.rule_values[idx].1 * value,
                )
            })
            .fold((0.0, 0.0), |(a, b), (c, d)| (a + c, b + d));

        phase * rule_values.0 + (1.0 - phase) * rule_values.1
    }

    /// Compute the gradient of the evaluation at a point `x`, multiply it by `scale`, and add it
    /// to `add_to`.
    fn eval_gradient(&self, add_to: &mut GradWeights, scale: f32, x: &BoardFeatures) {
        let midgame_material = self
            .rule_values
            .iter()
            .zip(&x.piece_counts)
            .map(|(&(value, _), &count)| value * count)
            .sum::<f32>();
        let phase = (self.phase_cutoffs.1 - midgame_material)
            / (self.phase_cutoffs.1 - self.phase_cutoffs.0);

        let inv_phase = 1.0 - phase;

        let rule_values = x
            .rules
            .iter()
            .map(|&(idx, rule)| {
                (
                    self.rule_values[idx].0 * rule,
                    self.rule_values[idx].1 * rule,
                )
            })
            .fold((0.0, 0.0), |(a, b), (c, d)| (a + c, b + d));

        // Compute gradient with respect to phase cutoffs.
        if 0.0 < phase && phase < 1.0 {
            let mult = scale
                * (rule_values.0 - rule_values.1)
                * (self.phase_cutoffs.1 - self.phase_cutoffs.0).powi(-2);
            add_to.phase_cutoffs.0 += mult * (self.phase_cutoffs.1 - midgame_material);
            add_to.phase_cutoffs.1 += mult * (midgame_material - self.phase_cutoffs.0);
        }

        // Compute gradient with respect to material weights

        // extra increase to partial with respect to material weight due to phase
        let phase_bonus = if 0.0 < phase && phase < 1.0 {
            (rule_values.0 - rule_values.1) / (self.phase_cutoffs.0 - self.phase_cutoffs.1)
        } else {
            0.0
        };
        // Compute gradient with respect to rule weights
        for &(rule_idx, rule_count) in &x.rules {
            let rv = &mut add_to.rule_values[rule_idx];
            rv.0 += scale * phase * rule_count;
            if rule_idx < 5 {
                rv.0 += scale * phase_bonus;
            }
            rv.1 += scale * inv_phase * rule_count;
        }
    }
}

impl SubAssign for Weights {
    fn sub_assign(&mut self, rhs: Self) {
        self.phase_cutoffs.0 -= rhs.phase_cutoffs.0;
        self.phase_cutoffs.1 -= rhs.phase_cutoffs.1;
        for (a, b) in self.rule_values.iter_mut().zip(rhs.rule_values) {
            a.0 -= b.0;
            a.1 -= b.1;
        }
    }
}
