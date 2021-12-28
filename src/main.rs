use crabchess::base::fens::FRIED_LIVER_FEN;
use crabchess::base::Game;
use crabchess::base::MoveGenerator;
use crabchess::engine::search::PVSearch;
use crabchess::engine::Engine;

#[allow(unused_imports)]
use crabchess::cli;
fn main() {
    let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
    let mgen = MoveGenerator::new();
    let mut e = PVSearch::default();
    e.set_depth(5);

    let x = e.get_evals(&mut g, &mgen);
    println!("{:?}", x);

    /*println!("running!");
    let mut app = cli::CrabchessApp::default();
    if let Err(_) = app.run() {
        println!("app failed!");
    }*/
}
