use std::{io::stdin, time::Duration};

use fiddler_base::Game;
use fiddler_engine::{
    pst::{pst_delta, pst_evaluate},
    thread::MainSearch,
    uci::{build_message, parse_line, EngineInfo, GoOption, OptionType, UciCommand, UciMessage},
};

/// Run a UCI engine.
fn main() {
    // whether we are in debug mode
    let mut debug = false;
    let mut searcher = MainSearch::new();
    let mut game = Game::default();

    loop {
        let mut buf = String::new();
        if stdin().read_line(&mut buf).is_err() {
            debug_info("failed to read line", debug);
        };
        let command = match parse_line(&buf, game.board()) {
            Ok(cmd) => cmd,
            Err(e) => {
                // print out the error to the frontend and continue on to the
                // next line
                debug_info(&format!("failed to parse line: {e}"), debug);
                continue;
            }
        };
        match command {
            UciCommand::Uci => {
                // identify the engine
                print!(
                    "{}",
                    build_message(&UciMessage::Id {
                        name: Some("Fiddler 0.1.0"),
                        author: Some("Clayton Ramsey"),
                    })
                );

                // add options

                add_option(
                    "Thread Count",
                    OptionType::Spin {
                        default: 16,
                        min: 1,
                        max: 255,
                    },
                );
            }
            UciCommand::Debug(new_debug) => {
                // activate or deactivate debug mode
                debug = new_debug;
            }
            UciCommand::IsReady => {
                // we were born ready
                print!("{}", build_message(&UciMessage::ReadyOk))
            }
            UciCommand::SetOption { name, value } => match name.as_str() {
                "Thread Count" => match value {
                    None => debug_info("error: no value given for number of threads", debug),
                    Some(num_str) => match num_str.parse() {
                        Ok(n) => searcher.set_nhelpers(n),
                        _ => debug_info("error: illegal parameter for `Thread Count`", debug),
                    },
                },
                _ => debug_info(&format!("error: unknown option key `{}`", name), debug),
            },
            UciCommand::NewGame => game = Game::default(),
            UciCommand::Position { fen, moves } => match fen {
                None => game = Game::default(),
                Some(fen) => match Game::from_fen(&fen, pst_evaluate) {
                    Ok(g) => {
                        game = g;
                        for m in moves {
                            game.make_move(m, pst_delta(game.board(), m));
                        }
                    }
                    Err(e) => debug_info(&format!("error: unable to load FEN: `{}`", e), debug),
                },
            },
            UciCommand::Go(opts) => {
                let mut ponder = false; // whether the last move given in the position should be considered the ponder-move

                // time remaining for players
                let (mut wtime, mut btime) = (None, None);

                // increments
                let (mut winc, mut binc) = (None, None);

                // number of moves until increment achieved. if `None`, there
                // is no increment.
                let mut movestogo = None;

                let mut infinite = false; // whether to search infinitely

                let mut movetime = None;

                *searcher.limit.nodes_cap.lock().unwrap() = None;
                for opt in opts {
                    match opt {
                        GoOption::SearchMoves(_) => {
                            unimplemented!("no implementation of searching move subsets")
                        }
                        GoOption::Ponder => todo!(),
                        GoOption::WhiteTime(time) => {
                            wtime = Some(time);
                        }
                        GoOption::BlackTime(time) => {
                            btime = Some(time);
                        }
                        GoOption::WhiteInc(inc) => {
                            winc = Some(inc);
                        }
                        GoOption::BlackInc(inc) => {
                            binc = Some(inc);
                        }
                        GoOption::MovesToGo(n) => {
                            movestogo = Some(n);
                        }
                        GoOption::Depth(d) => {
                            searcher.set_depth(d);
                        }
                        GoOption::Nodes(num) => {
                            *searcher.limit.nodes_cap.lock().unwrap() = Some(num);
                        }
                        GoOption::Mate(_) => unimplemented!(),
                        GoOption::MoveTime(msecs) => {
                            movetime = Some(Duration::from_millis(msecs as u64));
                        }
                        GoOption::Infinite => {
                            infinite = true;
                        }
                    }
                }
            }
            UciCommand::Stop => {
                searcher.limit.stop();
            }
            UciCommand::PonderHit => todo!(),
            UciCommand::Quit => break,
        }
    }
}

/// Print out a debug info message to the console. Will have no effect if
/// `debug` is `false`.
fn debug_info(s: &str, debug: bool) {
    if debug {
        print!(
            "{}",
            build_message(&UciMessage::Info(&[EngineInfo::String(s)]))
        );
    }
}

/// Send out a message to add an option for the frontend.
fn add_option(name: &str, opt: OptionType) {}
