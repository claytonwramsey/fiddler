use crate::uci::OptionType;
use crate::uci::UciMessage;

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
        UciMessage::Info(_) => todo!(),
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
