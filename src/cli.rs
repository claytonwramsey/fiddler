use crate::engine::search::Minimax;
use crate::Engine;
use crate::Game;
use crate::MoveGenerator;
use crate::Move;
use crate::algebraic::move_from_algebraic;

use std::io;
use std::fmt;

/**
 * A text-based application for running CrabChess.
 */
pub struct CrabchessApp<'a> {
    /**
     * The currently-played game.
     */
    game: Game,
    /**
     * The generator for moves.
     */
    mgen: MoveGenerator,
    /**
     * The currently-running engine to play against.
     */
    engine: Box<dyn Engine + 'a>,
    /**
     * The input stream to receive messages from.
     */
    input_stream: Box<dyn io::Read + 'a>,
    /**
     * The output stream to send messages to.
     */
    output_stream: Box<dyn io::Write + 'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/**
 * The set of commands which this command line program can execute.
 */
enum Command {
    /**
     * Quit the currently-running application.
     */
    Quit,
    /**
     * Echo an error message to the output stream.
     */
    EchoError(&'static str),
    /**
     * Select an engine to play against.
     */
    EngineSelect(Box<str>),
    /**
     * Play a move.
     */
    PlayMove(Move),
    /**
     * Load a FEN (Forsyth-Edwards Notation) string of a board position.
     */
    LoadFen(String),
    /**
     * Undo the most recent moves.
     */
    Undo(usize),
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Quit => write!(f, "quit"),
            Command::EchoError(s) => write!(f, "echo error {}", s),
            Command::EngineSelect(s) => write!(f, "select engine {}", s),
            Command::PlayMove(m) => write!(f, "play move {}", m),
            Command::LoadFen(s) => write!(f, "load fen {}", s),
            _ => write!(f, "undisplayable command"),
        }
    }
}

type CommandResult = Result<(), &'static str>;

impl<'a> CrabchessApp<'a> {
    /**
     * Run the command line application.
     * Will continue running until the user specifies to quit.
     */
    pub fn run(&mut self) {
        let mut has_quit = false;
        while !has_quit {
            let mut user_input = String::new();
            self.input_stream.read_to_string(&mut user_input);

            let parse_result = self.parse_command(user_input);
            let command = match parse_result {
                Ok(cmd) => cmd,
                Err(s) => Command::EchoError(s),
            };

            let execution_result = match command {
                Quit => {
                    has_quit = true;
                    writeln!(self.output_stream, "Now quitting.");
                    Ok(())
                },
                _ => self.execute_command(command),
            };

            if let Err(s) = execution_result {
                writeln!(self.output_stream, "an error occurred while executing the command: {}", s);
            }
        }
    }

    /**
     * Parse the given text command, and create a new `Command` to describe it.
     * Will return an `Err` if it cannot parse the given command.
     */
    fn parse_command(&self, s: String) -> Result<Command, &'static str> {
        let mut token_iter = s.split_ascii_whitespace();
        let first_token = token_iter.next();
        if first_token.is_none() {
            return Err("no token given");
        }
        let command_block = first_token.unwrap();
        if command_block.starts_with("/") {
            //this is a "pure" command
            return Err("no commands yet");
        } else {
            //this is a move
            let move_token = first_token;
            if move_token.is_none() {
                return Err("no move given to play");
            }
            let move_result = move_from_algebraic(move_token.unwrap(), self.game.get_board(), &self.mgen)?;
            
            Ok(Command::PlayMove(move_result))
        }

    }

    fn execute_command(&mut self, c: Command) -> CommandResult {
        match c {
            Command::EchoError(s) => self.echo_error(s),
            
            _ => {
                if let Err(e) = writeln!(self.output_stream, "the command type {} is unsupported", c) {
                    return Err("write failed");
                }
                Ok(())},
        }
    }

    fn echo_error(&mut self, s: &str) -> CommandResult {
        if let Err(e) = writeln!(self.output_stream, "error: {}", s) {
            return Err("failed to write error to output stream");
        }
        Ok(())
    }
}

impl<'a> Default for CrabchessApp<'a> {
    fn default() -> CrabchessApp<'a> {
        CrabchessApp {
            game: Game::default(),
            mgen: MoveGenerator::new(),
            engine: Box::new(Minimax::default()),
            input_stream: Box::new(io::stdin()),
            output_stream: Box::new(io::stdout()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PieceType;
    use crate::square::*;

    #[test]
    /**
     * Test that the quit input yields a quit command.
     */
    fn test_parse_quit() {
        let app = CrabchessApp::default();
        assert_eq!(
            app.parse_command(String::from("/q")), 
            Ok(Command::Quit));
    }

    #[test]
    /**
     * Test that move input yields a move command.
     */
    fn test_parse_move() {
        let app = CrabchessApp::default();
        
        assert_eq!(
            app.parse_command(String::from("e4")), 
            Ok(Command::PlayMove(Move::new(E2, E4, PieceType::NO_TYPE))));
    }
    
    #[test]
    /**
     * Test that load input yields a load fen command.
     */
    fn test_parse_load() {
        let app = CrabchessApp::default();
        assert_eq!(
            app.parse_command(String::from("/l r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7")), 
            Ok(Command::LoadFen(String::from("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7"))));
    }

    #[test]
    /**
     * Test that a garbage input does not parse correctly.
     */
    fn test_garbage_failure() {
        let app = CrabchessApp::default();
        assert!(app.parse_command(String::from("garbage")).is_err());
    }
}