use crabchess::engine::Minimax;
use crabchess::fens::FRIED_LIVER_FEN;
use crabchess::Engine;
use crabchess::Game;
use crabchess::MoveGenerator;

fn main() {
    let mut g = Game::from_fen(FRIED_LIVER_FEN).unwrap();
    let mgen = MoveGenerator::new();
    let mut e = Minimax::default();

    let x = e.get_evals(&mut g, &mgen);
    println!("{:?}", x);
}
