/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Parsing and constructing Universal Chess Interface (UCI) messages.
//!
//! UCI is a MVC standard for writing chess engines.
//! The GUI sends commands to the engine, and the engine then thinks according to the messages sent
//! to it.
//! In general, the GUI dictates much of the pace of this interaction, and the engine just plays
//! along.
//!
//! `UciCommand` describes all the messages that can be received for a UCI engine.
//! Meanwhile, `UciMessage` describes all the messages that the engine can send back to the GUI.
//!
//! For a full specification of the UCI standard, see [here](https://backscattering.de/).

use std::{fmt::Display, str::FromStr};

use crate::base::{game::Game, Move};

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// An enum representing the set of all commands that the GUI can send to the engine via UCI.
pub enum Command {
    /// Command given at the start of UCI.
    ///
    /// The engine must reply with `UciMessage::Id` message and send the `UciMessage::Option`
    /// messages to tell the GUI which engine commands it supports.
    /// After that receipt, the engine must then send a `UciMessage::Ok` to complete the setup.
    /// If not, the GUI will kill the engine process.
    Uci,
    /// Switch whether the engine should turn on or off debug mode.
    ///
    /// If true, the engine should activate debug mode and send info strings to the GUI.
    /// By default, debug mode should be off.
    Debug(bool),
    /// Request an update from the engine as to whether it is ready to proceed.
    ///
    /// If the engine is busy thinking, it can wait until it is done thinking to reply to this.
    /// When it is ready, the engine must reply with `UciMessage::ReadyOk`.
    IsReady,
    /// Set a parameter of the engine, or send a custom command.
    SetOption {
        /// The name of the option to be set.
        name: String,
        /// The value of the option to be set.
        value: Option<String>,
    },
    /// Inform the engine that the next position it will be requested to evaluate will be from a
    /// new game.
    ///
    /// An engine should not, however, expect a `NewGame`.
    NewGame,
    /// Update the next position the engine will be asked to evaluate.
    ///
    /// The engine should set up the position starting from the given FEN, and then play the given
    /// list of moves to reach the position.
    Position {
        /// The FEN from which to set up the position.
        ///
        /// If `fen` is `None`, then start from the default start position for a normal game of
        /// chess.
        fen: Option<String>,
        /// The set of moves to play after setting up with the given FEN.
        moves: Vec<Move>,
    },
    /// A `Go` will always be given after a `Position` command.
    ///
    /// The options are given as a table of options.
    Go(Vec<GoOption>),
    /// Stop searching immediately, and when done, reply with a best move and
    /// potentially a ponder.
    Stop,
    /// While the engine was in pondering mode, the player opposing the engine selected to play the
    /// ponder-move.
    ///
    /// Continue searching, but know that the player chose the ponder-move.
    PonderHit,
    /// Quit the program as soon as possible.
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The options that can be given for a `UciCommand::Go` command.
pub enum GoOption {
    /// Restrict the search to these moves only.
    SearchMoves(Vec<Move>),
    /// Search in "ponder" mode.
    ///
    /// The engine must not exit the search until ordered, no matter the conditions.
    /// The last move which was sent in the previous `UciCommand::Position`should be considered the
    /// "ponder-move", which is the suggested move to consider.
    /// When a `UciCommand::PonderHit` is given, the engine will then execute the ponder command.
    Ponder,
    /// Inform the engine that White has the given number of milliseconds remaining.
    /// The remaining time may be negative in the case of overtime play.
    WhiteTime(i32),
    /// Inform the engine that Black has the given number of milliseconds remaining.
    /// The remaining time may be negative in the case of overtime play.
    BlackTime(i32),
    /// Inform the engine that White has the given number of milliseconds as their time increment.
    WhiteInc(u32),
    /// Inform the engine that Black has the given number of milliseconds as their time increment.
    BlackInc(u32),
    /// Inform the engine that there are the given number of moves remaining until the next time
    /// control.
    /// If `WhiteTime` and `BlackTime` are not given, this means the current game is sudden death.
    MovesToGo(u8),
    /// Search with the given depth, looking at only the given number of plies.
    Depth(u8),
    /// Search only the given number of nodes.
    Nodes(u64),
    /// Search for a mate in the given number of moves.
    Mate(u8),
    /// Search for the given number of milliseconds.
    MoveTime(u32),
    /// Search until a `UciCommand::Stop` is given.
    /// Do not exit the search until told to.
    Infinite,
}

/// The result type for processing a line from a UCI command.
///
/// According to the UCI protocol, these errors should generally be logged or ignored completely.
pub type ParseResult = Result<Command, String>;

impl Command {
    /// Perform a read of a single UCI instruction.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if the line is not a legal UCI command.
    pub fn parse_line(line: &str) -> ParseResult {
        let mut tokens = line.split_ascii_whitespace();
        let first_tok = tokens.next().ok_or("line contains no tokens")?;
        match first_tok {
            "uci" => Ok(Command::Uci),
            "debug" => match tokens.next() {
                Some("on") | None => Ok(Command::Debug(true)),
                Some("off") => Ok(Command::Debug(false)),
                _ => Err("unrecognized option".into()),
            },
            "isready" => Ok(Command::IsReady),
            "setoption" => Command::parse_set_option(&mut tokens),
            "ucinewgame" => Ok(Command::NewGame),
            "position" => Command::parse_position(&mut tokens),
            "go" => Command::parse_go(&mut tokens),
            "stop" => Ok(Command::Stop),
            "ponderhit" => Ok(Command::PonderHit),
            "quit" => Ok(Command::Quit),
            _ => Err("unrecognized UCI command".into()),
        }
    }

    /// Parse a `setoption` line from a UCI string.
    ///
    /// Assumes that the `"setoption"` token in the line has already been consumed (i.e. that the
    /// next token will be `"name"`).
    fn parse_set_option(tokens: &mut dyn Iterator<Item = &str>) -> ParseResult {
        // consume `name` token
        let name_tok = tokens
            .next()
            .ok_or("reached end of line while searching for `name` field in `setoption`")?;
        if name_tok != "name" {
            return Err(format!(
                "expected token `name` for `setoption`, got `{name_tok}`"
            ));
        }

        // parse key
        let mut key = String::new();
        loop {
            let Some(key_tok) = tokens.next() else {
                return Ok(Command::SetOption {
                    name: key,
                    value: None,
                });
            };
            if key_tok == "value" {
                // we now expect a value string
                break;
            }
            if !key.is_empty() {
                key += " ";
            }
            key += key_tok;
        }

        // optionally parse value
        let mut value = String::new();
        loop {
            let Some(val_tok) = tokens.next() else {
                return Ok(Command::SetOption {
                    name: key,
                    value: Some(value),
                })
            };

            if !value.is_empty() {
                value += " ";
            }
            value += val_tok;
        }
    }

    /// Parse a `position` UCI command line.
    ///
    /// Assumes that the `"position"` token has already been consumed, so the next token will either
    /// be `"fen"` or `"startpos"`.
    fn parse_position(tokens: &mut dyn Iterator<Item = &str>) -> ParseResult {
        let (start_fen, next_move_tok) = match tokens
            .next()
            .ok_or_else(|| "reached EOL while parsing position".to_string())?
        {
            "fen" => {
                // Extract full FEN
                let mut fen = String::new();
                let mut next_tok = tokens.next();
                loop {
                    if next_tok == Some("moves") || next_tok.is_none() {
                        break;
                    }

                    let fen_tok = next_tok.unwrap();
                    if !fen.is_empty() {
                        fen += " ";
                    }
                    fen += fen_tok;

                    next_tok = tokens.next();
                }
                (Some(fen), None)
            }
            "startpos" => {
                let move_tok = tokens.next();
                if move_tok == Some("moves") {
                    (None, None)
                } else {
                    (None, move_tok)
                }
            }
            _ => return Err("illegal starting position token".to_string()),
        };

        let mut game = Game::from_fen(
            start_fen
                .as_deref()
                .unwrap_or("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"),
        )?;

        let mut moves = Vec::new();
        if let Some(m_tok) = next_move_tok {
            let m = Move::from_uci(m_tok, &game)?;
            game.make_move(m);
            moves.push(m);
        }
        for tok in tokens {
            let m = Move::from_uci(tok, &game)?;
            game.make_move(m);
            moves.push(m);
        }

        Ok(Command::Position {
            fen: start_fen,
            moves,
        })
    }

    #[allow(clippy::cast_possible_truncation)]
    /// Parse a `go` command from UCI.
    ///
    /// Assumes the token `go` has already been consumed.
    fn parse_go(tokens: &mut dyn Iterator<Item = &str>) -> ParseResult {
        /// A helper function for `parse_go` which will attempt to parse an int out of a token if it
        /// is `Some`, and fail if it cannot parse the int or if it is given `None`.
        fn parse_int<F: FromStr>(x: Option<&str>) -> Result<F, String>
        where
            F::Err: Display,
        {
            x.ok_or_else(|| "reached EOF while parsing int".to_string())?
                .parse()
                .map_err(|e| format!("could not parse int due to error: {e}"))
        }

        let mut opts = Vec::new();
        // build the options
        while let Some(opt_tok) = tokens.next() {
            opts.push(match opt_tok {
                "searchmoves" => return Err("go option `searchmoves` is not supported".to_string()),
                "ponder" => GoOption::Ponder,
                "wtime" => GoOption::WhiteTime(parse_int(tokens.next())?),
                "btime" => GoOption::BlackTime(parse_int(tokens.next())?),
                "winc" => GoOption::WhiteInc(parse_int(tokens.next())?),
                "binc" => GoOption::BlackInc(parse_int(tokens.next())?),
                "movestogo" => GoOption::MovesToGo(parse_int(tokens.next())?),
                "depth" => GoOption::Depth(parse_int(tokens.next())?),
                "nodes" => GoOption::Nodes(parse_int(tokens.next())?),
                "mate" => GoOption::Mate(parse_int(tokens.next())?),
                "movetime" => GoOption::MoveTime(parse_int(tokens.next())?),
                "infinite" => GoOption::Infinite,
                _ => return Err(format!("unrecognized option {opt_tok} for `go`")),
            });
        }

        Ok(Command::Go(opts))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Square;

    #[test]
    /// Test that an ordinary "startpos" UCI position command is parsed  correctly.
    fn position_starting() {
        assert_eq!(
            Command::parse_line("position startpos moves\n"),
            Ok(Command::Position {
                fen: None,
                moves: Vec::new()
            })
        );
    }

    #[test]
    /// Test that a position string can still pe parsed from startpos without a  moves token.
    fn position_starting_no_moves_tok() {
        assert_eq!(
            Command::parse_line("position startpos\n"),
            Ok(Command::Position {
                fen: None,
                moves: Vec::new(),
            })
        );
    }

    #[test]
    /// Test that a FEN is properly loaded from a UCI position command.
    fn position_fen() {
        assert_eq!(
            Command::parse_line(
                "position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves\n",
            ),
            Ok(Command::Position {
                fen: Some("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".into()),
                moves: Vec::new()
            })
        );
    }

    #[test]
    fn position_not_castle() {
        assert_eq!(
            Command::parse_line(
                "position fen 1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R3R1K1 w - - 0 26 moves e1c1\n",
            ),
            Ok(Command::Position {
                fen: Some("1rr3k1/5pp1/3pp2p/p2n3P/1q1P4/1P1Q1N2/5PP1/R3R1K1 w - - 0 26".into()), 
                moves: vec![Move::normal(Square::E1, Square::C1)],
            })
        );
    }

    #[test]
    /// Test that a FEN is properly loaded from a UCI position command.
    fn position_fen_then_moves() {
        assert_eq!(
            Command::parse_line("position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves c7c5 g1f3\n"), 
            Ok(Command::Position {
                fen: Some("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1".into()), 
                moves: vec![
                    Move::normal(Square::C7, Square::C5),
                    Move::normal(Square::G1, Square::F3)
                ]
            })
        );
    }

    #[test]
    /// Test that a position command with no `moves` token is parsed correctly.
    fn position_fen_no_moves() {
        assert_eq!(
            Command::parse_line(
                "position fen rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2\n",
            ),
            Ok(Command::Position {
                fen: Some("rnbqkbnr/pppp1ppp/8/4p3/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2".into()),
                moves: Vec::new()
            })
        );
    }

    #[test]
    /// Test that an option with no value is correctly set.
    fn setoption_key_only() {
        assert_eq!(
            Command::parse_line("setoption name MyOption\n"),
            Ok(Command::SetOption {
                name: "MyOption".into(),
                value: None
            })
        );
    }

    #[test]
    /// Test that a key-value pair for a setoption is correct.
    fn setoption_key_value() {
        assert_eq!(
            Command::parse_line("setoption name my option value 4 or 5\n"),
            Ok(Command::SetOption {
                name: "my option".into(),
                value: Some("4 or 5".into())
            })
        );
    }

    #[test]
    /// Test that a simple `go` command is parsed correctly.
    fn go_simple() {
        assert_eq!(
            Command::parse_line("go depth 7 nodes 25\n"),
            Ok(Command::Go(vec![GoOption::Depth(7), GoOption::Nodes(25),]))
        );
    }

    #[test]
    /// Test that a `go` command with every option is parsed correctly.
    /// In practice, this command would be invalid since the `infinite` option would remove the
    /// validity of all others.
    fn go_all() {
        assert_eq!(
            Command::parse_line(
                "go depth 7 nodes 250 infinite wtime 1 btime 2 winc 3 binc 4 movestogo 5 mate 6 movetime 7 ponder\n", 
        ),
            Ok(Command::Go(vec![
                GoOption::Depth(7),
                GoOption::Nodes(250),
                GoOption::Infinite,
                GoOption::WhiteTime(1),
                GoOption::BlackTime(2),
                GoOption::WhiteInc(3),
                GoOption::BlackInc(4),
                GoOption::MovesToGo(5),
                GoOption::Mate(6),
                GoOption::MoveTime(7),
                GoOption::Ponder,
            ]))
        );
    }

    #[test]
    /// Test that a `go searchmoves` does not cause the moves to eat future options.
    fn go_searchmoves() {
        assert!(Command::parse_line("go searchmoves e2e4 infinite\n").is_err());
    }

    #[test]
    /// Test that a `uci` command is parsed correctly.
    fn uci() {
        assert_eq!(Command::parse_line("uci\n"), Ok(Command::Uci));
    }

    #[test]
    /// Test that the `debug` commands are parsed correctly.
    fn debug() {
        assert_eq!(Command::parse_line("debug on\n"), Ok(Command::Debug(true)));

        assert_eq!(
            Command::parse_line("debug off\n"),
            Ok(Command::Debug(false))
        );
    }

    #[test]
    /// Test that negative numbers for remaining time are accepted in a go option.
    fn negative_time() {
        assert_eq!(
            Command::parse_line("go wtime 100 btime -100\n"),
            Ok(Command::Go(vec![
                GoOption::WhiteTime(100),
                GoOption::BlackTime(-100)
            ]))
        );
    }
}
