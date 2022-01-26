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
    /*let mut g =
        Game::from_fen("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7").unwrap();
    let mgen = MoveGenerator::new();
    let mut e = PVSearch::default();
    e.set_depth(7);

    let m = e.get_best_move(&mut g, &mgen);
    println!("{:?}", m);*/

    println!("running!");
    let mut app = cli::CrabchessApp::default();
    if let Err(_) = app.run() {
        println!("app failed!");
    }
}
