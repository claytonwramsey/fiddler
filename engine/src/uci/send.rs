use std::{fmt, time::Duration};

use crate::Eval;
use fiddler_base::Move;

#[derive(Clone, Eq, PartialEq, Hash)]
/// The set of messages that the engine can send to the GUI.
pub enum UciMessage<'a> {
    /// The engine identifies itself. Must be sent after receiving a
    /// `UciCommand::Uci` message.
    Id {
        /// The name of the engine.
        name: Option<&'a str>,
        /// The author of the engine.
        author: Option<&'a str>,
    },
    /// Sent after `id` and additional options are given to inform the GUI that
    /// the engine is ready in UCI mode.
    UciOk,
    /// Must be sent after a `UciCommand::IsReady` command and the engine has
    /// processed all input. Typically only for commands that take some time,
    /// but can actually be sent at any time.
    ReadyOk,
    /// Request that the GUI display an option to the user.
    /// Not to be confused with the standard `Option`.
    Option { name: &'a str, opt: OptionType<'a> },
    /// Inform the GUI that the engine has found a move. `m` is the best move
    /// that it found, and `ponder` may optionally be the opponent's reply to
    /// the best move that the engine would like to think about. Directly
    /// before a `BestMove`, the engine should send an `Info` command with the
    /// final search information.
    BestMove { m: Move, ponder: Option<Move> },
    /// Give the GUI some information about what the engine is thinking.
    Info(&'a [EngineInfo<'a>]),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// Information about an engine's search state.
pub enum EngineInfo<'a> {
    /// The depth to which this information was created.
    Depth(u8),
    /// The selective search depth.
    SelDepth(u8),
    /// The time searched.
    Time(Duration),
    /// The number of nodes searched.
    Nodes(u64),
    /// The principal variation.
    Pv(&'a [Move]),
    /// Optional. The number of principal variations given.
    MultiPv(u8),
    /// The evaluation of the position.
    Score {
        /// A numeric evaluation of the position.
        eval: Eval,
        /// Whether the evaluation given is only a lower bound.
        is_lower_bound: bool,
        /// Whether the evaluation given is only an upper bound.
        is_upper_bound: bool,
    },
    /// The current move being examined.
    CurrMove(Move),
    /// The number of the move currently being searched. For the first move
    /// searched, this would be 1, etc.
    CurrMoveNumber(u8),
    /// The hash fill rate of the transposition table. Measured out of 1000.
    HashFull(u16),
    /// The number of nodes searched per second by the engine.
    NodeSpeed(u64),
    /// Any string which should be displayed to the GUI. The string may not
    /// contain any newlines (`\n`).
    String(&'a str),
    /* Other infos omitted for now */
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OptionType<'a> {
    /// A spin box which takes an integer. The internal value is its default
    /// parameter.
    Spin { default: i64, min: i64, max: i64 },
    /// A string which the user can input. The default is the given value.
    String(Option<&'a str>),
    /// A checkbox which will either be true (checked) or false (unchecked).
    Check(Option<bool>),
    /// A set of selectable options for a mode.
    Combo {
        /// The default selection on the combination box.
        default: Option<&'a str>,
        /// The variations on the combinations. Need not include the value of
        /// the `default` part of this struct.
        vars: &'a [&'a str],
    },
    /// A button which can be pressed to send a command.
    Button,
}

impl<'a> fmt::Display for UciMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UciMessage::Id { name, author } => {
                write!(f, "id")?;
                if let Some(n) = name {
                    write!(f, " name {n}")?;
                }
                if let Some(a) = author {
                    if name.is_some() {
                        // we must break this into multiple lines
                        write!(f, "\nid")?;
                    }
                    write!(f, " author {a}")?;
                }
            }
            UciMessage::UciOk => write!(f, "uciok")?,
            UciMessage::ReadyOk => write!(f, "readyok")?,
            UciMessage::Option { name, ref opt } => write_option(f, name, opt)?,
            UciMessage::BestMove { m, ponder } => {
                write!(f, "bestmove {}", m.to_uci())?;
                if let Some(pondermove) = ponder {
                    write!(f, " ponder {pondermove}")?;
                }
            }
            UciMessage::Info(info) => write_info(f, info)?,
        };

        Ok(())
    }
}

/// Helper function to build an output line to inform the GUI of an option.
fn write_option(f: &mut fmt::Formatter, name: &str, opt: &OptionType) -> fmt::Result {
    write!(f, "option name {name} ")?;
    match opt {
        OptionType::Spin { default, min, max } => {
            write!(f, "type spin default {default} min {min} max {max}")?;
        }
        OptionType::String(s) => {
            write!(f, "type string")?;
            if let Some(st) = s {
                write!(f, " default {st}")?;
            }
        }
        OptionType::Check(opt_default) => {
            write!(f, "type check")?;
            if let Some(default) = opt_default {
                write!(f, " default {default}")?;
            }
        }
        OptionType::Combo { default, vars } => {
            write!(f, "type combo")?;
            if let Some(def_opt) = default {
                write!(f, " default {def_opt}")?;
            }
            for var in vars.iter() {
                write!(f, " var {var}")?;
            }
        }
        OptionType::Button => {
            write!(f, "type button")?;
        }
    }

    Ok(())
}

/// Build a set of messages for informing the GUI about facts of the engine.
fn write_info(f: &mut fmt::Formatter, infos: &[EngineInfo]) -> fmt::Result {
    let mut new_line = false;
    write!(f, "info")?;
    for info in infos {
        if new_line {
            write!(f, "\ninfo")?;
            new_line = false;
        }
        match info {
            EngineInfo::Depth(depth) => write!(f, " depth {depth}")?,
            EngineInfo::SelDepth(sd) => write!(f, " seldepth {sd}")?,
            EngineInfo::Time(t) => write!(f, " time {}", t.as_millis())?,
            EngineInfo::Nodes(n) => write!(f, " nodes {n}")?,
            EngineInfo::Pv(pv) => {
                write!(f, " pv")?;
                for m in pv.iter() {
                    write!(f, " {}", m.to_uci())?;
                }
            }
            EngineInfo::MultiPv(id) => write!(f, " multipv {id}")?,
            EngineInfo::Score {
                eval,
                is_lower_bound,
                is_upper_bound,
            } => {
                write!(f, " score ")?;
                match eval.moves_to_mate() {
                    Some(pl) => match eval > &Eval::DRAW {
                        true => write!(f, "mate {pl}")?,
                        false => write!(f, "mate -{pl}")?,
                    },
                    None => write!(f, "cp {}", eval.centipawn_val())?,
                };
                if is_lower_bound & !is_upper_bound {
                    write!(f, " lowerbound")?;
                } else if *is_upper_bound {
                    write!(f, " upperbound")?;
                }
            }
            EngineInfo::CurrMove(m) => write!(f, " currmove {}", m.to_uci())?,
            EngineInfo::CurrMoveNumber(num) => write!(f, " currmovenumber {num}")?,
            EngineInfo::HashFull(load) => write!(f, " hashfull {load}")?,
            EngineInfo::NodeSpeed(speed) => write!(f, " nps {speed}")?,
            // We split this info into two lines if
            EngineInfo::String(s) => {
                write!(f, " string {s}")?;
                new_line = true;
            }
        };
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use fiddler_base::{Eval, Move, Piece, Square};

    use std::time::Duration;

    #[test]
    /// Test an info message describing the current move.
    fn test_info_currmove() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Info(&[
                    EngineInfo::CurrMove(Move::normal(Square::E2, Square::E4)),
                    EngineInfo::CurrMoveNumber(1),
                ])
            ),
            "info currmove e2e4 currmovenumber 1"
        );
    }

    #[test]
    /// Test an info message describing a current move which is also a
    /// promotion.
    fn test_info_currmove_promotion() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Info(&[
                    EngineInfo::CurrMove(Move::promoting(Square::E7, Square::E8, Piece::Queen)),
                    EngineInfo::CurrMoveNumber(7),
                ])
            ),
            "info currmove e7e8q currmovenumber 7"
        );
    }

    #[test]
    /// Test an info message which is composed of many different pieces of
    /// information.
    fn test_info_composed() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Info(&[
                    EngineInfo::Depth(2),
                    EngineInfo::Score {
                        eval: Eval::pawns(2.14),
                        is_lower_bound: false,
                        is_upper_bound: false,
                    },
                    EngineInfo::Time(Duration::from_millis(1242)),
                    EngineInfo::Nodes(2124),
                    EngineInfo::NodeSpeed(34928),
                    EngineInfo::Pv(&[
                        Move::normal(Square::E2, Square::E4),
                        Move::normal(Square::E7, Square::E5),
                        Move::normal(Square::G1, Square::F3),
                    ]),
                ])
            ),
            "info depth 2 score cp 214 time 1242 nodes 2124 nps 34928 pv e2e4 e7e5 g1f3"
        )
    }

    #[test]
    /// Test an id message.
    fn test_id() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Id {
                    name: Some("Fiddler"),
                    author: Some("Clayton Ramsey"),
                }
            ),
            "id name Fiddler\nid author Clayton Ramsey"
        )
    }

    #[test]
    /// Test an option message for a checkbox.
    fn test_option_check() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Option {
                    name: "Nullmove",
                    opt: OptionType::Check(Some(true)),
                }
            ),
            "option name Nullmove type check default true"
        );
    }

    #[test]
    /// Test an option message for a spin-wheel.
    fn test_option_spin() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Option {
                    name: "Selectivity",
                    opt: OptionType::Spin {
                        default: 2,
                        min: 0,
                        max: 4
                    },
                }
            ),
            "option name Selectivity type spin default 2 min 0 max 4"
        )
    }

    #[test]
    /// Test an option message for a combo-box.
    fn test_option_combo() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Option {
                    name: "Style",
                    opt: OptionType::Combo {
                        default: Some("Normal"),
                        vars: &["Solid", "Normal", "Risky"],
                    }
                }
            ),
            "option name Style type combo default Normal var Solid var Normal var Risky"
        )
    }

    #[test]
    /// Test an option message for string input.
    fn test_option_string() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Option {
                    name: "NalimovPath",
                    opt: OptionType::String(Some("c:\\")),
                }
            ),
            "option name NalimovPath type string default c:\\"
        )
    }

    #[test]
    /// Test an option message for a button.
    fn test_option_button() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::Option {
                    name: "Clear Hash",
                    opt: OptionType::Button,
                }
            ),
            "option name Clear Hash type button"
        )
    }

    #[test]
    /// Test that best-moves are formatted correctly.
    fn test_bestmove() {
        assert_eq!(
            format!(
                "{}",
                UciMessage::BestMove {
                    m: Move::normal(Square::E2, Square::E4),
                    ponder: None
                }
            ),
            "bestmove e2e4"
        );
    }
}
