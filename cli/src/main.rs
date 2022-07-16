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

use std::env;

use fiddler_base::perft::perft;
use fiddler_cli::FiddlerApp;
use fiddler_engine::{evaluate::ScoredGame, thread::MainSearch};

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
                let g = ScoredGame::from_fen(
                    "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
                )
                .unwrap();

                let mut e = MainSearch::new();
                e.config.depth = 10;
                e.config.n_helpers = 7;

                let r = e.evaluate(&g);
                let info = r.unwrap();
                println!("depth {}: {} gives {}", info.depth, info.pv[0], info.eval);
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
