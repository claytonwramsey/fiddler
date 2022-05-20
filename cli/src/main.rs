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
                    "2r2r2/3p1p1k/p3p1p1/3P3n/q3P1Q1/1p5P/1PP2R2/1K4R1 w - - 0 30",
                    pst_evaluate,
                )
                .unwrap();

                let mut e = MainSearch::new();
                e.set_depth(9);
                e.set_nhelpers(15);

                let r = e.evaluate(&g);
                println!("{:?}", r);
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
