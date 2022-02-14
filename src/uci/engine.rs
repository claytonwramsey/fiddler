use crate::base::Game;

use super::UciModel;
use super::UciCommand::*;
use super::GoOption;
use super::EngineInfo;
use super::OptionType;

///
/// A fully-featued UCI engine which can send and receive commands effectively.
/// 
pub trait UciEngine {

    fn set_next_position(&mut self, g: Game);

    fn identify(&self) -> (Option<String>, Option<String>);

    fn get_opts(&self) -> Vec<(String, OptionType)>;

    fn go(&mut self, opts: Vec<GoOption>) -> (Vec<EngineInfo>, );

    fn set_option(&mut self, name: String, value: Option<String>);

}

impl UciModel for dyn UciEngine {
    fn receive(&mut self, command: super::UciCommand, _gui: &dyn super::UciGui) {
        match command {
            Uci => todo!(),
            Debug(_) => todo!(),
            IsReady => todo!(),
            SetOption { name, value } => self.set_option(name, value),
            NewGame => { /* do nothing? */},
            Position { fen, moves } => {
                let mut g = match fen {
                    Some(s) => Game::from_fen(&s).unwrap(),
                    None => Game::default(),
                };
                for m in moves {
                    g.make_move(m);
                }
                self.set_next_position(g);

            }
            Go(opts) => {
                self.go(opts);
            },
            Stop => todo!(),
            PonderHit => todo!(),
            Quit => todo!(),
        }
    }
}