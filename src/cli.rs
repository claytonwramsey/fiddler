use crate::base::algebraic::{algebraic_from_move, move_from_algebraic};
use crate::base::Game;
use crate::base::Move;
use crate::base::MoveGenerator;
use crate::engine::search::PVSearch;
use crate::engine::{ElapsedTimeout, TimeoutCondition};

use std::fmt;
use std::io;
use std::io::BufRead;
use std::time::Duration;

/// A text-based application for running CrabChess.
pub struct CrabchessApp<'a> {
    /// The currently-played game.
    game: Game,

    /// The generator for moves.
    mgen: MoveGenerator,

    /// The currently-running engine to play against.,
    engine: PVSearch,

    /// The input stream to receive messages from.
    input_stream: Box<dyn io::Read + 'a>,

    /// The output stream to send messages to.
    output_stream: Box<dyn io::Write + 'a>,

    /// The condition on which search will stop.
    timeout_condition: Box<dyn TimeoutCondition + 'a>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The set of commands which this command line program can execute.
///
enum Command {
    /// Quit the currently-running application.
    Quit,

    /// Echo an error message to the output stream.
    EchoError(&'static str),

    /// Select an engine to play against.
    EngineSelect(String),

    /// Play a move.
    PlayMove(Move),

    /// Load a FEN (Forsyth-Edwards Notation) string of a board.
    LoadFen(String),

    /// Undo the most recent moves.
    Undo(usize),

    /// List the available moves to the user.
    ListMoves,

    /// Request that the engine play the next move.
    EngineMove,

    /// Set the amount of time for which an engine can run. The number is the
    /// number of milliseconds on the timeout.
    SetTimeout(u64),

    /// Print out the history of the game currently being played.
    PrintHistory,
}

impl fmt::Display for Command {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Command::Quit => write!(f, "quit"),
            Command::EchoError(s) => write!(f, "echo error {s}"),
            Command::EngineSelect(s) => write!(f, "select engine {s}"),
            Command::PlayMove(m) => write!(f, "play move {m}"),
            Command::LoadFen(s) => write!(f, "load fen {s}"),
            Command::Undo(n) => write!(f, "undo {n}"),
            Command::ListMoves => write!(f, "list moves"),
            Command::EngineMove => write!(f, "play engine move"),
            Command::SetTimeout(n) => write!(f, "set timeout {:.3}", *n as f32 / 1000f32),
            Command::PrintHistory => write!(f, "print history"),
        }
    }
}

type CommandResult = Result<(), &'static str>;

impl<'a> CrabchessApp<'a> {
    /// Run the command line application.
    /// Will continue running until the user specifies to quit.
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut has_quit = false;
        while !has_quit {
            let board = self.game.board();
            writeln!(self.output_stream, "{board}")?;
            writeln!(self.output_stream, "Type out a move or enter a command.")?;
            let mut user_input = String::new();
            let mut buf_reader = io::BufReader::new(&mut self.input_stream);

            if let Err(e) = buf_reader.read_line(&mut user_input) {
                writeln!(
                    self.output_stream,
                    "failed to read off of input stream with error {e}"
                )?;
            };

            let parse_result = self.parse_command(user_input);
            let command = match parse_result {
                Ok(cmd) => cmd,
                Err(s) => Command::EchoError(s),
            };

            let execution_result = match command {
                Command::Quit => {
                    has_quit = true;
                    writeln!(self.output_stream, "Now quitting.")?;
                    Ok(())
                }
                _ => self.execute_command(command),
            };

            if let Err(s) = execution_result {
                writeln!(
                    self.output_stream,
                    "an error occurred while executing the command: {s}"
                )?;
            }
        }
        Ok(())
    }

    /// Parse the given text command, and create a new `Command` to describe it.
    /// Will return an `Err` if it cannot parse the given command.
    fn parse_command(&self, s: String) -> Result<Command, &'static str> {
        let mut token_iter = s.split_ascii_whitespace();
        let first_token = token_iter.next();
        if first_token.is_none() {
            return Err("no token given");
        }
        let command_block = first_token.unwrap();
        if command_block.starts_with('/') {
            let command_name = command_block.get(1..);
            if command_name == None {
                return Err("no command specified");
            }

            match command_name.unwrap() {
                "q" | "quit" => Ok(Command::Quit),
                "e" | "engine" => {
                    let engine_opt = String::from(s[command_block.len()..].trim());
                    Ok(Command::EngineSelect(engine_opt))
                }
                "l" | "load" => {
                    let fen_str = String::from(s[command_block.len()..].trim());
                    Ok(Command::LoadFen(fen_str))
                }
                "u" | "undo" => {
                    let num_undo_token = token_iter.next();
                    match num_undo_token {
                        None => Ok(Command::Undo(1)),
                        Some(num_undo_str) => match num_undo_str.parse::<usize>() {
                            Ok(num) => {
                                if num > 0 {
                                    return Ok(Command::Undo(num));
                                }
                                Err("cannot undo 0 moves")
                            }
                            Err(_) => Err("could not parse number of moves to undo"),
                        },
                    }
                }
                "m" | "move" => Ok(Command::EngineMove),
                "t" | "timeout" => {
                    let n_msecs_token = token_iter.next();
                    match n_msecs_token {
                        None => Err("required number of milliseconds until timeout"),
                        Some(t) => match t.parse::<u64>() {
                            Ok(num) => Ok(Command::SetTimeout(num)),
                            Err(_) => Err("no number given for timeout"),
                        },
                    }
                }
                "list" => Ok(Command::ListMoves),
                "h" | "history" => Ok(Command::PrintHistory),
                _ => Err("unrecognized command"),
            }
        } else {
            //this is a move
            let move_token = first_token;
            if move_token.is_none() {
                return Err("no move given to play");
            }
            let move_result =
                move_from_algebraic(move_token.unwrap(), self.game.board(), &self.mgen)?;

            Ok(Command::PlayMove(move_result))
        }
    }

    fn execute_command(&mut self, c: Command) -> CommandResult {
        match c {
            Command::EchoError(s) => self.echo_error(s),
            Command::LoadFen(fen) => self.load_fen(fen),
            Command::PlayMove(m) => self.try_move(m),
            Command::ListMoves => self.list_moves(),
            Command::Undo(n) => self.game.undo_n(n),
            Command::EngineSelect(s) => self.select_engine(s),
            Command::EngineMove => self.play_engine_move(),
            Command::SetTimeout(num) => {
                println!("{num} milliseconds");
                self.timeout_condition = Box::new(ElapsedTimeout::new(Duration::from_millis(num)));
                Ok(())
            }
            Command::PrintHistory => match writeln!(self.output_stream, "{}", self.game) {
                Ok(()) => Ok(()),
                Err(_) => Err("write failed"),
            },
            _ => {
                if writeln!(self.output_stream, "the command type `{c}` is unsupported").is_err() {
                    return Err("write failed");
                }
                Ok(())
            }
        }
    }

    /// Echo out an error string to the user.
    fn echo_error(&mut self, s: &str) -> CommandResult {
        if writeln!(self.output_stream, "error: {s}").is_err() {
            return Err("failed to write error to output stream");
        }
        Ok(())
    }

    /// Attempt to load a FEN string into the game.
    fn load_fen(&mut self, fen: String) -> CommandResult {
        match Game::from_fen(fen.as_str()) {
            Ok(game) => {
                self.game = game;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Attempt to play a move.
    fn try_move(&mut self, m: Move) -> CommandResult {
        self.game.try_move(&self.mgen, m)?;
        self.play_engine_move()?;

        Ok(())
    }

    /// Print out a list of the available moves in this position.
    fn list_moves(&mut self) -> CommandResult {
        let moves = self.mgen.get_moves(self.game.board());
        for m in moves.iter() {
            if writeln!(
                self.output_stream,
                "{}",
                algebraic_from_move(*m, self.game.board(), &self.mgen)
            )
            .is_err()
            {
                return Err("failed to write move list");
            }
        }
        Ok(())
    }

    /// Select an engine based on the given options string.
    fn select_engine(&mut self, opts: String) -> CommandResult {
        // For now, we just use it to set the depth, as there are no engines to
        // select.
        match opts.parse::<usize>() {
            Ok(num) => {
                self.engine.set_depth(num);
                Ok(())
            }
            Err(_) => Err("could not parse engine selection"),
        }
    }

    /// Have the engine play a move.
    fn play_engine_move(&mut self) -> CommandResult {
        self.timeout_condition.start();
        let m = self
            .engine
            .best_move(&mut self.game, &self.mgen, self.timeout_condition.as_ref());
        println!(
            "the engine played {}",
            algebraic_from_move(m, self.game.board(), &self.mgen)
        );
        self.game.make_move(m);

        Ok(())
    }
}

impl<'a> Default for CrabchessApp<'a> {
    fn default() -> CrabchessApp<'a> {
        CrabchessApp {
            game: Game::default(),
            mgen: MoveGenerator::default(),
            engine: PVSearch::default(),
            input_stream: Box::new(io::stdin()),
            output_stream: Box::new(io::stdout()),
            timeout_condition: Box::new(ElapsedTimeout::new(Duration::from_secs(5))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Square;

    #[test]
    /// Test that the quit input yields a quit command.
    fn test_parse_quit() {
        let app = CrabchessApp::default();
        assert_eq!(app.parse_command(String::from("/q")), Ok(Command::Quit));
    }

    #[test]
    /// Test that move input yields a move command.
    fn test_parse_move() {
        let app = CrabchessApp::default();

        assert_eq!(
            app.parse_command(String::from("e4")),
            Ok(Command::PlayMove(Move::new(Square::E2, Square::E4, None)))
        );
    }

    #[test]
    /// Test that load input yields a load fen command.
    fn test_parse_load() {
        let app = CrabchessApp::default();
        assert_eq!(
            app.parse_command(String::from(
                "/l r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7"
            )),
            Ok(Command::LoadFen(String::from(
                "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7"
            )))
        );
    }

    #[test]
    /// Test that executing a FEN load is successful.
    fn test_execute_load() {
        let mut app = CrabchessApp::default();
        assert_eq!(
            app.execute_command(Command::LoadFen(String::from(
                "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7"
            ))),
            Ok(())
        );
        assert_eq!(
            app.game,
            Game::from_fen("r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7").unwrap()
        );
    }

    #[test]
    /// Test that we can parse an engine selection command.
    fn test_parse_engine() {
        let app = CrabchessApp::default();
        assert_eq!(
            app.parse_command(String::from("/e m 8")),
            Ok(Command::EngineSelect(String::from("m 8")))
        );
    }

    #[test]
    /// Test that a garbage input does not parse correctly.
    fn test_garbage_failure() {
        let app = CrabchessApp::default();
        assert!(app.parse_command(String::from("garbage")).is_err());
    }
}
