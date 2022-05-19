use std::{env, path::Path, sync::Arc, time::Instant};

use fiddler_base::{Board, Color, Piece, Square};
use fiddler_engine::evaluate::phase_of;
use libm::expf;
use rand::{thread_rng, Rng};
use rusqlite::Connection;

/// The dimension of the feature vector.
const FEATURE_DIM: usize = 772;

/// A datum of training data.
struct TrainingDatum {
    /// The string describing the FEN of the position.
    fen: String,
    /// The evaluation of the position.
    eval: f32,
}

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
    let mut weights = vec![3., 3., 5., 9.];
    let mut rng = thread_rng();
    for _ in 4..FEATURE_DIM {
        weights.push(rng.gen_range(-0.1..0.1));
    }
    let mut learn_rate = 10.;
    let beta = 0.6;

    let nthreads = 8;
    let tic = Instant::now();
    // construct the datasets.
    // Outermost vector: each element for one thread
    // Middle vector: each element for one datum
    // Inner vector: each element for one piece-square pair
    let mut input_sets = vec![];
    let mut statement = db.prepare("SELECT fen, eval FROM evaluations").unwrap();
    let data = statement
        .query_map([], |row| {
            Ok(TrainingDatum {
                fen: row.get(0)?,
                eval: row.get(1)?,
            })
        })
        .unwrap()
        .map(|res| res.unwrap());
    for datum in data {
        input_sets.push((extract(&Board::from_fen(&datum.fen).unwrap()), datum.eval));
    }
    let input_arc = Arc::new(input_sets);
    let toc = Instant::now();
    println!("extracted data in {} secs", (toc - tic).as_secs());
    for i in 0..10000 {
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

/// The input feature set of a board. Each element is a (key, value) pair where
/// the key is the index of the value in the full feature vector.
type BoardFeatures = Vec<(usize, f32)>;

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
            new_weights[i] -= learn_rate * sub_grad[i] / chunk_size as f32;
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
    input: &[(Vec<(usize, f32)>, f32)],
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

#[inline]
/// Compute the  sigmoid function of a variable. `beta` is the
/// horizontal scaling of the sigmoid.
///
/// The mathematical function is given by the LaTeX expression
/// `f(x) = \frac{1}{1 - \exp (- \beta x)}`.
fn sigmoid(x: f32, beta: f32) -> f32 {
    1. / (1. + expf(-x * beta))
}

/// Print out a weights vector so it can be used as code.
fn print_weights(weights: &[f32]) {
    let piece_val = |name: &str, val: f32| {
        println!(
            "const {name}_VAL: Eval = Eval::centipawns({})",
            (val * 100.) as i16
        );
    };

    piece_val("PAWN", 1.);
    piece_val("KNIGHT", weights[0]);
    piece_val("BISHOP", weights[1]);
    piece_val("ROOK", weights[2]);
    piece_val("QUEEN", weights[3]);

    println!("const PTABLE: Pst = expand_table([");
    for pt in Piece::ALL_TYPES {
        println!("\t[ // {pt}");
        let pt_idx = 4 + (128 * pt as usize);
        for rank in 0..8 {
            print!("\t\t");
            for file in 0..8 {
                let sq = Square::new(rank, file).unwrap();
                let mg_idx = pt_idx + (2 * sq as usize);
                let eg_idx = mg_idx + 1;
                let mg_val = (weights[mg_idx] * 100.) as i16;
                let eg_val = (weights[eg_idx] * 100.) as i16;
                print!("({mg_val}, {eg_val}), ");
            }
            println!("// rank {rank}");
        }
        println!("\t],");
    }
    println!("])");
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
/// The representation is sparse, so each usize corresponds to an index in the
/// true vector. Zero entries will not be in the output.
fn extract(b: &Board) -> Vec<(usize, f32)> {
    let mut features: Vec<(usize, f32)> = Vec::new();
    // Indices 0..4: non-king piece values
    for pt in [Piece::Knight, Piece::Bishop, Piece::Rook, Piece::Queen] {
        let n_white = (b[pt] & b[Color::White]).count_ones() as i8;
        let n_black = (b[pt] & b[Color::Black]).count_ones() as i8;
        features.push((pt as usize - 1, (n_white - n_black) as f32));
    }

    let offset = 4; // offset added to PST positions
    let phase = phase_of(b);

    let bocc = b[Color::Black];
    let wocc = b[Color::White];

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

    features
}

#[inline]
/// Given the extracted feature vector of a position, and a weight vector, get
/// the final evaluation.
///
/// # Panics
///
/// if `features` and `weights` are not the same length.
fn evaluate(features: &[(usize, f32)], weights: &[f32]) -> f32 {
    features.iter().map(|&(idx, val)| val * weights[idx]).sum()
}
