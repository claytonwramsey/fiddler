#[allow(unused_imports)]
use crabchess::base::fens::FRIED_LIVER_FEN;
#[allow(unused_imports)]
use crabchess::base::Game;
#[allow(unused_imports)]
use crabchess::base::MoveGenerator;
#[allow(unused_imports)]
use crabchess::engine::search::PVSearch;
#[allow(unused_imports)]
use crabchess::engine::Engine;

#[allow(unused_imports)]
use crabchess::cli;
fn main() {
    let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
    let mgen = MoveGenerator::new();
    let mut e = PVSearch::default();
    e.set_depth(6);

    let x = e.get_evals(&mut g, &mgen);
    println!("{:?}", x);

    /*println!("running!");
    let mut app = cli::CrabchessApp::default();
    if let Err(_) = app.run() {
        println!("app failed!");
    }*/
}
