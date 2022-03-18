#[allow(unused_imports)]
use crabchess::base::Game;
#[allow(unused_imports)]
use crabchess::base::MoveGenerator;
#[allow(unused_imports)]
use crabchess::engine::search::PVSearch;

#[allow(unused_imports)]
use crabchess::cli;
fn main() {
    /*crabchess::base::perft::perft(
        "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
        5,
    );*/
    /*let mut g =
        Game::from_fen("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7").unwrap();

    let mut e = PVSearch::default();
    e.set_depth(10);

    let m = e.best_move(&mut g, &mgen, &crabchess::engine::NoTimeout);
    println!("{:?}", m);*/

    println!("running!");
    let mut app = cli::CrabchessApp::default();
    if app.run().is_err() {
        println!("app failed!");
    }
}
