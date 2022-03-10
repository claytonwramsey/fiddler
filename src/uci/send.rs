use crate::engine::Eval;
use crate::uci::OptionType;
use crate::uci::UciMessage;

use super::EngineInfo;

pub fn build_message(message: UciMessage) -> String {
    match message {
        UciMessage::Id { name, author } => {
            let mut result = String::new();
            if let Some(n) = name {
                result += &format!("name {n}\n");
            }
            if let Some(a) = author {
                result += &format!("author {a}\n");
            }
            result
        }
        UciMessage::UciOk => String::from("uciok\n"),
        UciMessage::ReadyOk => String::from("readyok\n"),
        UciMessage::Option { name, opt } => build_option(name, opt),
        UciMessage::BestMove { m, ponder } => {
            let mut result = format!("bestmove {m} ");
            if let Some(pondermove) = ponder {
                result += &format!("ponder {pondermove}");
            }
            result += "\n";
            result
        }
        UciMessage::Info(info) => build_info(info),
    }
}

///
/// Helper function to build an output line to inform the GUI of an option.
///
fn build_option(name: String, opt: OptionType) -> String {
    let mut result = format!("option name {name} ");
    match opt {
        OptionType::Spin { default, min, max } => {
            result += &format!("type spin default {default} min {min} max {max}");
        }
        OptionType::String(s) => {
            result += "type string ";
            if let Some(st) = s {
                result += &format!("default {st} ");
            }
        }
        OptionType::Check(opt_default) => {
            result += "type check ";
            if let Some(default) = opt_default {
                result += &format!("default {default} ");
            }
        }
        OptionType::Combo { default, vars } => {
            result += "type combo ";
            if let Some(def_opt) = default {
                result += &format!("default {def_opt}");
            }
            for var in vars {
                result += &format!("var {var} ");
            }
        }
        OptionType::Button => {
            result += "type button ";
        }
    }
    result += "\n";

    result
}

///
/// Build a set of messages for informing the GUI about facts of the engine.
///
fn build_info(infos: Vec<EngineInfo>) -> String {
    let mut result = String::from("info ");
    for info in infos {
        match info {
            EngineInfo::Depth(depth) => result += &format!("depth {depth} "),
            EngineInfo::SelDepth(sd) => result += &format!("seldepth {sd} "),
            EngineInfo::Time(t) => result += &format!("time {} ", t.as_millis()),
            EngineInfo::Nodes(n) => result += &format!("nodes {n} "),
            EngineInfo::Pv(pv) => {
                result += "pv ";
                for m in pv {
                    result += &format!("{m} ");
                }
            }
            EngineInfo::MultiPv(id) => result += &format!("multipv {id} "),
            EngineInfo::Score {
                eval,
                is_lower_bound,
                is_upper_bound,
            } => {
                result += "score ";
                result += &match eval.plies_to_mate() {
                    Some(pl) => match eval > Eval::DRAW {
                        true => format!("{pl} "),
                        false => format!("-{pl} "),
                    },
                    None => format!("{} ", eval.centipawn_val()),
                };
                if is_lower_bound & !is_upper_bound {
                    result += "lowerbound ";
                }
                else if is_upper_bound {
                    result += "upperbound ";
                }
            },
            EngineInfo::CurrMove(m) => result += &format!("currmove {m}"),
            EngineInfo::CurrMoveNumber(num) => result += &format!("currmovenumber {num} "),
            EngineInfo::HashFull(load) => result += &format!("hashfull {load} "),
            EngineInfo::NodeSpeed(speed) => result += &format!("nps {speed} "),
            EngineInfo::String(s) => result += &format!("string {s} "),
        };
    }
    result
}
