use crate::base::algebraic::{algebraic_from_move, move_from_algebraic};
use crate::base::movegen::get_moves;
use crate::base::Game;
use crate::base::Move;
use crate::engine::limit::SearchLimit;
use crate::engine::pst::{pst_delta, pst_evaluate};
use crate::engine::thread::MainSearch;

use std::fmt;
use std::io;
use std::io::BufRead;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// A text-based application for running Fiddler.
pub struct FiddlerApp<'a> {
    /// The currently-played game.
    game: Game,

    /// The currently-running engine to play against.,
    engine: MainSearch,

    /// The input stream to receive messages from.
    input_stream: Box<dyn io::Read + 'a>,

    /// The output stream to send messages to.
    output_stream: Box<dyn io::Write + 'a>,

    /// The condition on which search will stop.
    limit: Arc<SearchLimit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The set of commands which this command line program can execute.
enum Command {
    /// Quit the currently-running application.
    Quit,

    /// Echo an error message to the output stream.
    EchoError(String),

    /// Select an engine to play against.
    EngineSelect(String),

    /// Play a move.
    PlayMove {
        /// The move to play.
        m: Move,
        /// Whether the engine should make an immediate reply to the move.
        engine_reply: bool,
    },

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
            Command::PlayMove {
                m,
                engine_reply: reply,
            } => write!(f, "play move {m}; reply? {reply}"),
            Command::LoadFen(s) => write!(f, "load fen {s}"),
            Command::Undo(n) => write!(f, "undo {n}"),
            Command::ListMoves => write!(f, "list moves"),
            Command::EngineMove => write!(f, "play engine move"),
            Command::SetTimeout(n) => write!(f, "set timeout {:.3}", *n as f32 / 1000.),
            Command::PrintHistory => write!(f, "print history"),
        }
    }
}

type CommandResult = Result<(), String>;

type ParseResult = Result<Command, String>;

impl<'a> FiddlerApp<'a> {
    /// Run the command line application.
    /// Will continue running until the user specifies to quit.
    pub fn run(&mut self) -> std::io::Result<()> {
        let mut has_quit = false;
        let mut user_input = String::with_capacity(64);
        while !has_quit {
            let board = self.game.board();
            writeln!(self.output_stream, "{board}")?;
            writeln!(self.output_stream, "Type out a move or enter a command.")?;

            let mut buf_reader = io::BufReader::new(&mut self.input_stream);

            buf_reader.read_line(&mut user_input)?;

            println!("user input: {:?}", &user_input);

            let parse_result = self.parse_command(&user_input);
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

            user_input.clear();
        }
        Ok(())
    }

    /// Parse the given text command, and create a new `Command` to describe it.
    /// Will return an `Err` if it cannot parse the given command.
    fn parse_command(&self, s: &str) -> ParseResult {
        let mut token_iter = s.split_ascii_whitespace();
        let first_token = token_iter.next();
        if first_token.is_none() {
            panic!();
        }
        let command_block = first_token.ok_or("no token given")?;
        let result: ParseResult = if command_block.starts_with('/') {
            let command_name = command_block.get(1..).ok_or("no command specified")?;

            match command_name {
                "q" | "quit" => Ok(Command::Quit),
                "e" | "engine" => {
                    let engine_opt = s[command_block.len()..].trim().into();
                    Ok(Command::EngineSelect(engine_opt))
                }
                "l" | "load" => {
                    let fen_str = s[command_block.len()..].trim().into();
                    Ok(Command::LoadFen(fen_str))
                }
                "u" | "undo" => {
                    let num_undo = token_iter
                        .next()
                        .map(|s| s.parse::<usize>())
                        .unwrap_or(Ok(1)) // no token given -> assune you wanted to undo 1
                        .map_err(|_| "could not parse number to undo")?;
                    match num_undo {
                        0 => Err("cannot undo 0 moves".into()),
                        n => Ok(Command::Undo(n)),
                    }
                }
                "m" | "move" => Ok(Command::EngineMove),
                "p" | "play" => {
                    self.parse_move_token(token_iter.next())
                        .map(|m| Command::PlayMove {
                            m,
                            engine_reply: false,
                        })
                }
                "t" | "timeout" => Ok(Command::SetTimeout(
                    token_iter
                        .next()
                        .ok_or("required number of milliseconds until timeout")?
                        .parse::<u64>()
                        .map_err(|_| "failed to parse timeout")?,
                )),
                "list" => Ok(Command::ListMoves),
                "h" | "history" => Ok(Command::PrintHistory),
                _ => return Err("unrecognized command".into()),
            }
        } else {
            //this is a move
            self.parse_move_token(first_token)
                .map(|m| Command::PlayMove {
                    m,
                    engine_reply: true,
                })
        };

        result
    }

    /// Parse a token for an algebraic move. Returns
    fn parse_move_token(&self, move_token: Option<&str>) -> Result<Move, String> {
        let m_str = move_token.ok_or("no move token given")?;
        Ok(move_from_algebraic(m_str, self.game.position())?)
    }

    fn execute_command(&mut self, c: Command) -> CommandResult {
        match c {
            Command::EchoError(s) => self.echo_error(&s),
            Command::LoadFen(fen) => self.load_fen(&fen),
            Command::PlayMove { m, engine_reply } => self.try_move(m, engine_reply),
            Command::ListMoves => self.list_moves(),
            Command::Undo(n) => self.game.undo_n(n).map_err(String::from),
            Command::EngineSelect(s) => self.select_engine(s),
            Command::EngineMove => self.play_engine_move(),
            Command::SetTimeout(num) => {
                println!("{num} milliseconds");
                let mut limit = SearchLimit::new();
                limit.search_duration = Mutex::new(Some(Duration::from_millis(num)));
                self.set_limit(limit);
                Ok(())
            }
            Command::PrintHistory => match writeln!(self.output_stream, "{}", self.game) {
                Ok(()) => Ok(()),
                Err(_) => Err("write failed".into()),
            },
            _ => writeln!(self.output_stream, "the command type `{c}` is unsupported")
                .map_err(|_| "write failed".into()),
        }
    }

    /// Echo out an error string to the user.
    fn echo_error(&mut self, s: &str) -> CommandResult {
        writeln!(self.output_stream, "error: {s}").map_err(|_| "write failed".into())
    }

    /// Attempt to load a FEN string into the game.
    fn load_fen(&mut self, fen: &str) -> CommandResult {
        self.game = Game::from_fen(fen, pst_evaluate)?;
        Ok(())
    }

    /// Attempt to play a move.
    fn try_move(&mut self, m: Move, engine_reply: bool) -> CommandResult {
        self.game.try_move(m, pst_delta(self.game.board(), m))?;
        if engine_reply {
            self.play_engine_move()?;
        }

        Ok(())
    }

    /// Print out a list of the available moves in this position.
    fn list_moves(&mut self) -> CommandResult {
        let moves = get_moves(self.game.position());
        for m in moves.iter() {
            writeln!(
                self.output_stream,
                "{}",
                algebraic_from_move(*m, self.game.position())
            )
            .map_err(|_| "failed to write move list")?;
        }
        Ok(())
    }

    /// Select an engine based on the given options string.
    fn select_engine(&mut self, opts: String) -> CommandResult {
        // For now, we just use it to set the depth, as there are no engines to
        // select.
        self.engine.set_depth(
            opts.parse()
                .map_err(|_| "could not parse engine selection")?,
        );

        Ok(())
    }

    /// Have the engine play a move.
    fn play_engine_move(&mut self) -> CommandResult {
        self.limit
            .start()
            .map_err(|_| String::from("poisoned limit locks"))?;
        let search_data = self
            .engine
            .evaluate(&self.game)
            .map_err(|_| "evaluation failed")?;

        writeln!(
            self.output_stream,
            "depth {}: the engine played {}: {}",
            search_data.2,
            algebraic_from_move(search_data.0, self.game.position()),
            search_data.1
        )
        .map_err(|_| "failed to write to output")?;
        self.game
            .make_move(search_data.0, pst_delta(self.game.board(), search_data.0));

        Ok(())
    }

    /// Set the internal search limit of this CLI, and update the searcher to
    /// match.
    fn set_limit(&mut self, limit: SearchLimit) {
        let arc_limit = Arc::new(limit);
        self.limit = arc_limit.clone();
        self.engine.limit = arc_limit;
    }
}

impl<'a> Default for FiddlerApp<'a> {
    fn default() -> FiddlerApp<'a> {
        let arc_limit = {
            let mut limit = SearchLimit::new();
            limit.search_duration = Mutex::new(Some(Duration::from_secs(5)));
            Arc::new(limit)
        };
        let mut app = FiddlerApp {
            game: Game::default(),
            engine: MainSearch::new(),
            input_stream: Box::new(io::stdin()),
            output_stream: Box::new(io::stdout()),
            limit: arc_limit.clone(),
        };
        app.engine.limit = arc_limit;
        app.engine.set_nhelpers(15);
        app
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Square;

    #[test]
    /// Test that the quit input yields a quit command.
    fn test_parse_quit() {
        let app = FiddlerApp::default();
        assert_eq!(app.parse_command("/q"), Ok(Command::Quit));
    }

    #[test]
    /// Test that move input yields a move command.
    fn test_parse_move() {
        let app = FiddlerApp::default();

        assert_eq!(
            app.parse_command("e4"),
            Ok(Command::PlayMove {
                m: Move::new(Square::E2, Square::E4, None),
                engine_reply: true,
            })
        );
    }

    #[test]
    /// Test that load input yields a load fen command.
    fn test_parse_load() {
        let app = FiddlerApp::default();
        assert_eq!(
            app.parse_command("/l r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7"),
            Ok(Command::LoadFen(
                "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7".into()
            ))
        );
    }

    #[test]
    /// Test that executing a FEN load is successful.
    fn test_execute_load() {
        let mut app = FiddlerApp::default();
        assert_eq!(
            app.execute_command(Command::LoadFen(
                "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7".into()
            )),
            Ok(())
        );
        assert_eq!(
            app.game,
            Game::from_fen(
                "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7",
                pst_evaluate
            )
            .unwrap()
        );
    }

    #[test]
    /// Test that we can parse an engine selection command.
    fn test_parse_engine() {
        let app = FiddlerApp::default();
        assert_eq!(
            app.parse_command("/e m 8"),
            Ok(Command::EngineSelect("m 8".into()))
        );
    }

    #[test]
    /// Test that a garbage input does not parse correctly.
    fn test_garbage_failure() {
        let app = FiddlerApp::default();
        assert!(app.parse_command("garbage").is_err());
    }
}
