#[allow(unused_imports)]
use crabchess::base::Game;
#[allow(unused_imports)]
use crabchess::cli;
#[allow(unused_imports)]
use crabchess::engine::pst::pst_evaluate;
#[allow(unused_imports)]
use crabchess::engine::thread::MainSearch;

fn main() {
    /*crabchess::base::perft::perft(
        "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
        5,
    );*/
    /*let g = Game::from_fen(
        "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
        pst_evaluate,
    )
    .unwrap();

    let mut e = MainSearch::new();
    e.set_depth(10);
    e.set_nhelpers(15);

    let r = e.evaluate(&g);
    println!("{:?}", r);*/

    println!("running!");
    let mut app = cli::CrabchessApp::default();
    if app.run().is_err() {
        println!("app failed!");
    }
}
