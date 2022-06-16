use std::env;

use fiddler_base::{perft::perft, Game};
use fiddler_cli::FiddlerApp;
use fiddler_engine::{pst::pst_evaluate, thread::MainSearch};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        match args[1].as_str() {
            "perft" => {
                if args.len() >= 3 {
                    // args[1] is the depth, args[2..] is the FEN
                    let depth = args[2].parse::<u8>().unwrap();
                    let fen = args[3..].join(" ");
                    perft(&fen, depth);
                } else {
                    println!("please specify a depth and a FEN");
                }
            }
            "cli" => {
                // Run the CLI application.
                let mut app = FiddlerApp::default();
                if app.run().is_err() {
                    println!("app failed!");
                }
            }
            "bench" => {
                // for now, just run a benchmark on the fried liver fen
                let g = Game::from_fen(
                    "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
                    pst_evaluate,
                )
                .unwrap();

                let mut e = MainSearch::new();
                e.config.depth = 11;
                e.config.n_helpers = 7;
                e.config.max_transposition_depth = 99;

                let r = e.evaluate(&g);
                let info = r.unwrap();
                println!(
                    "depth {}: {} gives {}",
                    info.highest_successful_depth, info.best_move, info.eval
                );
            }
            _ => {
                println!("unrecognized mode of operation {:?}", args[0]);
            }
        };
    } else {
        // no arguments; for now just exit
        println!("modes: cli, perft, bench");
    }
}
