/*use crabchess::engine::search::Minimax;
use crabchess::fens::FRIED_LIVER_FEN;
use crabchess::Engine;
use crabchess::Game;
use crabchess::MoveGenerator;
*/

use crabchess::cli;
fn main() {
    /*let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
    let mgen = MoveGenerator::new();
    let mut e = Minimax::default();

    let x = e.get_evals(&mut g, &mgen);
    println!("{:?}", x);*/
    println!("running!");
    let mut app = cli::CrabchessApp::default();
    if let Err(_) = app.run() {
        println!("app failed!");
    }
}
