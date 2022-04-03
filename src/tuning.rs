use crate::{
    base::{Bitboard, Board, Color, Eval, Piece},
    engine::evaluate::phase_of,
};
use libm::expf;

/// The dimension of the feature vector.
const FEATURE_DIM: usize = 772;

fn main(args: &[String]) {
    
}

/// Perform one step of PST training, and update the weights to reflect this.
/// `inputs` is a vector containing the input vector and the expected
/// evaluation. `weights` is the weight vector to train on. `sigmoid_scale` is
/// the x-scaling of the sigmoid activation function, and `learn_rate` is a
/// coefficient on the speed at which the engine learns. Each element of
/// `inputs` must be the same length as `weights`.
fn train_step(
    inputs: &[(&[f32], Eval)],
    weights: &mut Vec<f32>,
    sigmoid_scale: f32,
    learn_rate: f32,
) {
    for feature_vec in inputs {
        let eval = evaluate(feature_vec.0, weights);
        let sigmoid = 2. / (1. + expf(-eval)) - 1.; // ranges from -1 to 1
    }
}

/// Extract a feature vector from a board. The resulting vector will have
/// dimension 772 (=4 + (64 * 6 * 2)). A pawn is always worth 100 centipawns,
/// so not included in the feature vector. The PST values can be up to 1 for a
/// white piece on the given PST square, -1 for a black piece, or 0 for both or
/// neither. The PST values are then pre-blended by game phase.
///
/// The elements of the vector are listed by their indices as follows:
///
/// * 0: Knight quantity
/// * 1: Bishop quantity
/// * 2: Rook quantity
/// * 3: Queen quantity
/// * 4..132: Pawn PST, paired (midgame, endgame) element-wise
/// * 132..260: Knight PST
/// * 260..388: Bishop PST
/// * 388..516: Rook PST
/// * 516..644: Queen PST
/// * 644..772: King PST
///
/// Ranges given above are lower-bound inclusive.
fn extract(b: &Board) -> Vec<f32> {
    let mut features = Vec::new();
    // Indices 0..4: non-king piece values
    for pt in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let n_white = (b[pt] & b[Color::White]).count_ones() as i8;
        let n_black = (b[pt] & b[Color::Black]).count_ones() as i8;
        features.push((n_white - n_black) as f32);
    }

    let offset = 4; // offset added to PST positions
    let phase = phase_of(b);

    features.extend([0.; 64 * 2 * 6]);
    for sq in Bitboard::ALL {
        if let Some(pt) = b.type_at_square(sq) {
            let (sq_idx, increment) = match b.color_at_square(sq).unwrap() {
                Color::White => (sq as usize, 1.),
                Color::Black => (sq.opposite() as usize, -1.),
            };
            let pt_idx = pt as usize;
            let idx = offset + 128 * pt_idx + 2 * sq_idx;
            features[idx] += phase * increment;
            features[idx + 1] += (1. - phase) * increment;
        }
    }

    features
}

#[inline]
/// Given the extracted feature vector of a position, and a weight vector, get
/// the final evaluation.
///
/// # Panics
///
/// if `features` and `weights` are not the same length.
fn evaluate(features: &[f32], weights: &[f32]) -> f32 {
    assert_eq!(features.len(), weights.len());

    features
        .iter()
        .zip(weights.iter())
        .map(|(f_i, w_i)| f_i * w_i)
        .sum()
}
