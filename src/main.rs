use crabchess::base::fens::FRIED_LIVER_FEN;
use crabchess::base::Game;
use crabchess::base::MoveGenerator;
use crabchess::engine::search::PVSearch;
use crabchess::engine::Engine;

use crabchess::cli;
fn main() {
    let mut g =
        Game::from_fen("rnbqkbnr/ppp1pp1p/6p1/3pP3/3P4/8/PPP2PPP/RNBQKBNR b KQkq - 0 3").unwrap();
    let mgen = MoveGenerator::new();
    let mut e = PVSearch::default();

    let x = e.get_evals(&mut g, &mgen);
    println!("{:?}", x);

    /*println!("running!");
    let mut app = cli::CrabchessApp::default();
    if let Err(_) = app.run() {
        println!("app failed!");
    }*/
}
