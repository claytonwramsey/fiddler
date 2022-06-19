use std::{env, path::Path, sync::Arc, time::Instant};

use fiddler_base::{Board, Color, Piece, Square};
use fiddler_engine::{
    evaluate::{net_doubled_pawns, net_open_rooks, phase_of, DOUBLED_PAWN_VALUE, OPEN_ROOK_VALUE},
    material,
    pst::PST,
};
use libm::expf;
use rand::Rng;
use rusqlite::Connection;

/// A datum of training data.
struct TrainingDatum {
    /// The string describing the FEN of the position.
    fen: String,
    /// The evaluation of the position.
    eval: f32,
}

/// The input feature set of a board. Each element is a (key, value) pair where
/// the key is the index of the value in the full feature vector.
type BoardFeatures = Vec<(usize, f32)>;

/// Run the main training function.
///
/// # Panics
///
/// Panics if we are unable to locate the database file.
pub fn main() {
    let args: Vec<String> = env::args().collect();
    // first argument is the name of the binary
    let path_str = &args[1..].join(" ");
    println!("path is `{path_str}`");
    let path = Path::new(path_str);
    let db = Connection::open(path).unwrap();
    let mut weights = load_weights();
    fuzz(&mut weights, 0.05);
    let mut learn_rate = 5.;
    let beta = 0.6;

    let nthreads = 16;
    let tic = Instant::now();
    let mut statement = db.prepare("SELECT fen, eval FROM evaluations").unwrap();

    // construct the datasets.
    // Outer vector: each element for one datum
    // Inner vector: each element for one feature-quantity pair
    let input_sets: Vec<(BoardFeatures, f32)> = statement
        .query_map([], |row| {
            Ok(TrainingDatum {
                fen: row.get(0)?,
                eval: row.get(1)?,
            })
        })
        .unwrap()
        .map(|res| res.unwrap())
        .map(|datum| (extract(&Board::from_fen(&datum.fen).unwrap()), datum.eval))
        .collect();

    let input_arc = Arc::new(input_sets);
    let toc = Instant::now();
    println!("extracted data in {} secs", (toc - tic).as_secs());
    for i in 0..500 {
        weights = train_step(
            input_arc.clone(),
            Arc::new(weights),
            learn_rate,
            beta,
            nthreads,
        );
        learn_rate *= 0.999;
        println!("iteration {i}...")
    }

    print_weights(&weights);
}

/// Perform one step of PST training, and update the weights to reflect this.
/// `inputs` is a vector containing the input vector and the expected
/// evaluation. `weights` is the weight vector to train on. `sigmoid_scale` is
/// the x-scaling of the sigmoid activation function, and `learn_rate` is a
/// coefficient on the speed at which the engine learns. Each element of
/// `inputs` must be the same length as `weights`. Returns the MSE of the
/// current epoch.
fn train_step(
    inputs: Arc<Vec<(BoardFeatures, f32)>>,
    weights: Arc<Vec<f32>>,
    learn_rate: f32,
    sigmoid_scale: f32,
    nthreads: usize,
) -> Vec<f32> {
    let tic = Instant::now();
    let mut grads = Vec::new();

    let chunk_size = inputs.len() / nthreads;
    for thread_id in 0..nthreads {
        // start the parallel work
        let start = chunk_size * thread_id;
        let inclone = inputs.clone();
        let wclone = weights.clone();
        grads.push(std::thread::spawn(move || {
            train_thread(&inclone[start..start + chunk_size], &wclone, sigmoid_scale)
        }));
    }
    let mut new_weights: Vec<f32> = weights.to_vec();
    let mut sum_se = 0.;
    for grad_handle in grads {
        let (sub_grad, se) = grad_handle.join().unwrap();
        sum_se += se;
        for i in 0..new_weights.len() {
            new_weights[i] -= learn_rate * sub_grad[i] / inputs.len() as f32;
        }
    }
    let toc = Instant::now();
    println!(
        "{} nodes in {} sec: {:.0} nodes/sec; mse {}",
        inputs.len(),
        (toc - tic).as_secs(),
        inputs.len() as f32 / (toc - tic).as_secs_f32(),
        sum_se / inputs.len() as f32
    );

    new_weights
}

/// Construct the gradient vector for a subset of the input data. Also provides
/// the sum of the squared error.
fn train_thread(
    input: &[(BoardFeatures, f32)],
    weights: &[f32],
    sigmoid_scale: f32,
) -> (Vec<f32>, f32) {
    let mut grad = vec![0.; weights.len()];
    let mut sum_se = 0.;
    for datum in input {
        let features = &datum.0;
        let sigm_expected = sigmoid(datum.1, sigmoid_scale);
        let sigm_eval = sigmoid(evaluate(features, weights), sigmoid_scale);
        let err = 2. * (sigm_expected - sigm_eval);
        let coeff = -sigmoid_scale * sigm_eval * (1. - sigm_eval) * err;
        // gradient descent
        for &(idx, feat_val) in features {
            grad[idx] += feat_val * coeff;
        }
        sum_se += err * err;
    }

    (grad, sum_se)
}

#[inline(always)]
/// Compute the  sigmoid function of a variable. `beta` is the
/// horizontal scaling of the sigmoid.
///
/// The mathematical function is given by the LaTeX expression
/// `f(x) = \frac{1}{1 - \exp (- \beta x)}`.
fn sigmoid(x: f32, beta: f32) -> f32 {
    1. / (1. + expf(-x * beta))
}

/// Load the weight value constants from the ones defined in the PST evaluation.
fn load_weights() -> Vec<f32> {
    let mut weights = Vec::new();
    for pt in Piece::NON_KING_TYPES {
        let val = material::value(pt);
        weights.push(val.0.float_val());
        weights.push(val.1.float_val());
    }

    for pt in Piece::ALL_TYPES {
        for rank in 0..8 {
            for file in 0..8 {
                let sq_idx = 8 * rank + file;
                let score = PST[pt as usize][sq_idx];
                weights.push(score.0.float_val());
                weights.push(score.1.float_val());
            }
        }
    }

    weights.push(DOUBLED_PAWN_VALUE.0.float_val());
    weights.push(DOUBLED_PAWN_VALUE.1.float_val());

    weights.push(OPEN_ROOK_VALUE.0.float_val());
    weights.push(OPEN_ROOK_VALUE.1.float_val());

    weights
}

/// Add random values, ranging from +/- `amplitude`, to each element of `v`.
fn fuzz(v: &mut [f32], amplitude: f32) {
    let mut rng = rand::thread_rng();
    for elem in v.iter_mut() {
        *elem += amplitude * (2. * rng.gen::<f32>() - 1.);
    }
}

/// Print out a weights vector so it can be used as code.
fn print_weights(weights: &[f32]) {
    let paired_val = |name: &str, start: usize| {
        println!(
            "const {name}: Score = (Eval::centipawns({}), Eval::centipawns({}));",
            (weights[start] * 100.) as i16,
            (weights[start + 1] * 100.) as i16,
        );
    };

    paired_val("KNIGHT_VAL", 0);
    paired_val("BISHOP_VAL", 2);
    paired_val("ROOK_VAL", 4);
    paired_val("QUEEN_VAL", 6);
    paired_val("PAWN_VAL", 8);

    paired_val("DOUBLED_PAWN_VAL", 778);
    paired_val("OPEN_ROOK_VAL", 780);

    println!("const PST: Pst = expand_table([");
    for pt in Piece::ALL_TYPES {
        println!("    [ // {pt}");
        let pt_idx = 10 + (128 * pt as usize);
        for rank in 0..8 {
            print!("        ");
            for file in 0..8 {
                let sq = Square::new(rank, file).unwrap();
                let mg_idx = pt_idx + (2 * sq as usize);
                let eg_idx = mg_idx + 1;
                let mg_val = (weights[mg_idx] * 100.) as i16;
                let eg_val = (weights[eg_idx] * 100.) as i16;
                print!("({mg_val}, {eg_val}), ");
            }
            println!();
        }
        println!("    ],");
    }
    println!("])");
}

/// Extract a feature vector from a board. The resulting vector will have
/// dimension 773. The PST values can be up to 1 for a
/// white piece on the given PST square, -1 for a black piece, or 0 for both or
/// neither. The PST values are then pre-blended by game phase.
///
/// The elements of the vector are listed by their indices as follows:
///
/// * 0-2: Knight quantity
/// * 2-4: Bishop quantity
/// * 4-6: Rook quantity
/// * 6-8: Queen quantity
/// * 8-10: Pawn quantity
/// * 10..138: Knight PST, paired (midgame, endgame) element-wise
/// * 138..266: Bishop PST
/// * 266..394: Rook PST
/// * 394..522: Queen PST
/// * 522..650: Pawn PST
/// * 650..778: King PST
/// * 778..780: Number of doubled pawns (mg, eg) weighted
///     (e.g. 1 if White has 2 doubled pawns and Black has 1)
/// * 780..782: Net number of open rooks
///
/// Ranges given above are lower-bound inclusive.
/// The representation is sparse, so each usize corresponds to an index in the
/// true vector. Zero entries will not be in the output.
fn extract(b: &Board) -> BoardFeatures {
    let mut features = Vec::with_capacity(28);
    let phase = phase_of(b);
    // Indices 0..4: non-king piece values
    for pt in Piece::NON_KING_TYPES {
        let n_white = (b[pt] & b[Color::White]).count_ones() as i8;
        let n_black = (b[pt] & b[Color::Black]).count_ones() as i8;
        let net = n_white - n_black;
        if net != 0 {
            let idx = 2 * (pt as usize);
            features.push((idx, phase * (net as f32)));
            features.push((idx, (1. - phase) * (net as f32)));
        }
    }

    let mut offset = 10; // offset added to PST positions

    let bocc = b[Color::Black];
    let wocc = b[Color::White];

    // Get piece-square quantities
    for pt in Piece::ALL_TYPES {
        let pt_idx = pt as usize;
        for sq in b[pt] {
            let alt_sq = sq.opposite();
            let increment = match (wocc.contains(sq), bocc.contains(alt_sq)) {
                (true, false) => 1.,
                (false, true) => -1.,
                _ => continue,
            };
            let idx = offset + 128 * pt_idx + 2 * (sq as usize);

            features.push((idx, phase * increment));
            features.push((idx + 1, (1. - phase) * increment));
        }
    }

    // New offset after everything in the PST.
    offset = 778;

    // Doubled pawns
    let doubled_count = net_doubled_pawns(b);
    if doubled_count != 0 {
        features.push((offset, (doubled_count as f32) * phase));
        features.push((offset + 1, (doubled_count as f32) * (1. - phase)));
    }

    let open_rook_count = net_open_rooks(b);
    if open_rook_count != 0 {
        features.push((offset + 2, (open_rook_count as f32) * phase));
        features.push((offset + 3, (open_rook_count as f32) * (1. - phase)));
    }

    features
}

#[inline(always)]
/// Given the extracted feature vector of a position, and a weight vector, get
/// the final evaluation.
///
/// # Panics
///
/// if `features` and `weights` are not the same length.
fn evaluate(features: &[(usize, f32)], weights: &[f32]) -> f32 {
    features.iter().map(|&(idx, val)| val * weights[idx]).sum()
}
