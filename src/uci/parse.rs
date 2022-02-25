use crate::base::Move;
use crate::uci::UciCommand;

use super::GoOption;

///
/// The result type for processing a line from a UCI command. According to the
/// UCI protocol, these errors should generally be logged or ignored.
///
pub type UciParseResult = Result<UciCommand, String>;

///
/// Perform a read of a single UCI instruction.
///
pub fn parse_line(line: &str) -> UciParseResult {
    let mut tokens = line.split_ascii_whitespace();
    let first_tok = tokens.next().ok_or("line contains no tokens")?;
    match first_tok {
        "uci" => Ok(UciCommand::Uci),
        "debug" => match tokens.next() {
            Some("on") | None => Ok(UciCommand::Debug(true)),
            Some("off") => Ok(UciCommand::Debug(false)),
            _ => Err(String::from("unrecognized option")),
        },
        "isready" => Ok(UciCommand::IsReady),
        "setoption" => parse_set_option(&mut tokens),
        "ucinewgame" => Ok(UciCommand::NewGame),
        "position" => parse_position(&mut tokens),
        "go" => parse_go(&mut tokens),
        "stop" => Ok(UciCommand::Stop),
        "ponderhit" => Ok(UciCommand::PonderHit),
        "quit" => Ok(UciCommand::Quit),
        _ => Err(String::from("unrecognized UCI command")),
    }
}

///
/// Parse a `setoption` line from a UCI string. Assumes that the `"setoption"`
/// token in the line has already been consumed (i.e. that the next token will
/// be `"name"`).
///
fn parse_set_option(tokens: &mut dyn Iterator<Item = &str>) -> UciParseResult {
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
        let key_tok = match tokens.next() {
            Some(tok) => tok,
            None => {
                return Ok(UciCommand::SetOption {
                    name: key,
                    value: None,
                })
            }
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
        let val_tok = match tokens.next() {
            Some(val) => val,
            None => {
                return Ok(UciCommand::SetOption {
                    name: key,
                    value: Some(value),
                })
            }
        };

        if !value.is_empty() {
            value += " ";
        }
        value += val_tok;
    }
}

///
/// Parse a `position` UCI command line. Assumes that the `"position"` token
/// has already been consumed, so the next token will either be `"fen"` or
/// `"startpos"`.
///
fn parse_position(tokens: &mut dyn Iterator<Item = &str>) -> UciParseResult {
    let start_fen = match tokens
        .next()
        .ok_or_else(|| "reached EOL while parsing position".to_string())?
    {
        "fen" => {
            // Extract
            let mut fen = String::new();
            let mut next_tok = tokens.next().ok_or("reached EOL while parsing FEN")?;
            loop {
                if next_tok == "moves" {
                    break;
                }
                if !fen.is_empty() {
                    fen += " ";
                }
                fen += next_tok;

                next_tok = tokens.next().ok_or("reached EOL while parsing FEN")?;
            }
            Some(fen)
        }
        "startpos" => {
            let moves_tok = tokens.next().ok_or("reached EOL while parsing position")?;
            if moves_tok != "moves" {
                return Err(format!(
                    "expected token `moves` after `startpos`, got {moves_tok}"
                ));
            }

            None
        }
        _ => return Err("illegal starting position token".to_string()),
    };

    let mut moves = Vec::new();
    for m_result in tokens.map(Move::from_uci) {
        match m_result {
            Ok(m) => moves.push(m),
            Err(e) => return Err(format!("could not parse UCI move: {e}")),
        };
    }

    Ok(UciCommand::Position {
        fen: start_fen,
        moves,
    })
}

///
/// Parse a `go` command from UCI. Assumes the token `go` has already been
/// consumed.
///
fn parse_go(tokens: &mut dyn Iterator<Item = &str>) -> UciParseResult {
    let mut opts = Vec::new();
    let mut peeks = tokens.peekable();
    // build the options
    while let Some(opt_tok) = peeks.next() {
        opts.push(match opt_tok {
            "searchmoves" => {
                let mut moves = Vec::new();
                // continually add moves to the set of moves to search until we
                // bump into a keyword
                loop {
                    let move_peek = peeks.peek();
                    match move_peek {
                        Some(m_tok) => {
                            if let Ok(m) = Move::from_uci(m_tok) {
                                moves.push(m);
                                // consume the token that we peeked
                                peeks.next()
                            } else {
                                break;
                            }
                        }
                        None => break,
                    };
                }

                GoOption::SearchMoves(moves)
            }
            "ponder" => GoOption::Ponder,
            "wtime" => GoOption::WhiteTime(parse_int(peeks.next())? as u32),
            "btime" => GoOption::BlackTime(parse_int(peeks.next())? as u32),
            "winc" => GoOption::WhiteInc(parse_int(peeks.next())? as u32),
            "binc" => GoOption::BlackInc(parse_int(peeks.next())? as u32),
            "movestogo" => GoOption::MovesToGo(parse_int(peeks.next())? as u8),
            "depth" => GoOption::Depth(parse_int(peeks.next())? as u8),
            "nodes" => GoOption::Nodes(parse_int(peeks.next())?),
            "mate" => GoOption::Mate(parse_int(peeks.next())? as u8),
            "movetime" => GoOption::MoveTime(parse_int(peeks.next())? as u32),
            "infinite" => GoOption::Infinite,
            _ => return Err(format!("unrecognized option {opt_tok} for `go`")),
        });
    }

    Ok(UciCommand::Go(opts))
}

///
/// A helper function for `parse_go` which will attempt to parse an int out of
/// a token if it is `Some`, and fail if it cannot parse the int or if it is
/// given `None`.
///
fn parse_int(x: Option<&str>) -> Result<u64, String> {
    match x {
        None => Err(String::from("reached EOF while parsing int")),
        Some(s) => s
            .parse()
            .map_err(|e| format!("could not parse int due to error: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::Square;
    #[test]
    ///
    /// Test that an ordinary "startpos" UCI position command is parsed
    /// correctly.
    ///
    fn test_position_starting() {
        assert_eq!(
            parse_line("position startpos moves\n"),
            Ok(UciCommand::Position {
                fen: None,
                moves: Vec::new()
            })
        );
    }

    #[test]
    ///
    /// Test that a FEN is properly loaded from a UCI position command.
    ///
    fn test_position_fen() {
        assert_eq!(
            parse_line(
                "position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves\n"
            ),
            Ok(UciCommand::Position {
                fen: Some(String::from(
                    "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
                )),
                moves: Vec::new()
            })
        );
    }

    #[test]
    ///
    /// Test that a FEN is properly loaded from a UCI position command.
    ///
    fn test_position_fen_then_moves() {
        assert_eq!(
            parse_line("position fen rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1 moves c7c5 g1f3\n"), 
            Ok(UciCommand::Position {
                fen: Some(String::from("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")), 
                moves: vec![
                    Move::normal(Square::C7, Square::C5),
                    Move::normal(Square::G1, Square::F3)
                ]
            })
        );
    }

    #[test]
    ///
    /// Test that an option with no value is correctly set.
    ///
    fn test_setoption_key_only() {
        assert_eq!(
            parse_line("setoption name MyOption\n"),
            Ok(UciCommand::SetOption {
                name: String::from("MyOption"),
                value: None
            })
        );
    }

    #[test]
    ///
    /// Test that a key-value pair for a setoption is correct.
    ///
    fn test_setoption_key_value() {
        assert_eq!(
            parse_line("setoption name my option value 4 or 5\n"),
            Ok(UciCommand::SetOption {
                name: String::from("my option"),
                value: Some(String::from("4 or 5"))
            })
        );
    }

    #[test]
    ///
    /// Test that a simple `go` command is parsed correctly.
    ///
    fn test_go_simple() {
        assert_eq!(
            parse_line("go depth 7 nodes 25\n"),
            Ok(UciCommand::Go(vec![
                GoOption::Depth(7),
                GoOption::Nodes(25),
            ]))
        );
    }

    #[test]
    ///
    /// Test that a `go` command with every option is parsed correctly. In
    /// practice this command would be invalid since the `infinite` option
    /// would remove the validity of all others.
    ///
    fn test_go_all() {
        assert_eq!(
            parse_line("go depth 7 nodes 250 infinite searchmoves e2e4 wtime 1 btime 2 winc 3 binc 4 movestogo 5 mate 6 movetime 7 ponder\n"),
            Ok(UciCommand::Go(vec![
                GoOption::Depth(7),
                GoOption::Nodes(250),
                GoOption::Infinite,
                GoOption::SearchMoves(vec![Move::normal(Square::E2, Square::E4)]),
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
    ///
    /// Test that a `go searchmoves` does not cause the moves to eat future
    /// options.
    ///
    fn test_go_searchmoves() {
        assert_eq!(
            parse_line("go searchmoves e2e4 infinite\n"),
            Ok(UciCommand::Go(vec![
                GoOption::SearchMoves(vec![Move::normal(Square::E2, Square::E4)]),
                GoOption::Infinite,
            ]))
        );
    }

    #[test]
    ///
    /// Test that a `uci` command is parsed correctly.
    ///
    fn test_uci() {
        assert_eq!(parse_line("uci\n"), Ok(UciCommand::Uci));
    }

    #[test]
    ///
    /// Test that the `debug` commands are parsed correctly.
    ///
    fn test_debug() {
        assert_eq!(parse_line("debug on\n"), Ok(UciCommand::Debug(true)));

        assert_eq!(parse_line("debug off\n"), Ok(UciCommand::Debug(false)));
    }
}
