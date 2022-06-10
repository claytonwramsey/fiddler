use std::{
    io::stdin,
    sync::{Arc, RwLock},
    time::Duration,
};

use fiddler_base::Game;
use fiddler_engine::{
    pst::{pst_delta, pst_evaluate},
    thread::MainSearch,
    time::get_search_time,
    uci::{build_message, parse_line, EngineInfo, GoOption, OptionType, UciCommand, UciMessage},
};

/// Run a UCI engine.
fn main() {
    // whether we are in debug mode
    let mut debug = false;
    let searcher = Arc::new(RwLock::new(MainSearch::new()));
    let mut game = Game::default();
    let mut search_handle = None;

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
                    UciMessage::Id {
                        name: Some("Fiddler 0.1.0"),
                        author: Some("Clayton Ramsey"),
                    }
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

                print!("{}", UciMessage::UciOk)
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
                        Ok(n) => searcher.write().unwrap().set_nhelpers(n),
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
                // whether the last move given in the position should be
                // considered the ponder-move
                let mut _ponder = false;

                // time remaining for players
                let (mut wtime, mut btime) = (None, None);

                // increments. by default assumed to be zero
                let (mut winc, mut binc) = (0, 0);

                // number of moves until increment achieved. if `None`, there
                // is no increment.
                let mut movestogo = None;

                let mut infinite = false; // whether to search infinitely

                let mut movetime = None;

                *searcher.read().unwrap().limit.nodes_cap.lock().unwrap() = None;
                for opt in opts {
                    match opt {
                        GoOption::SearchMoves(_) => {
                            unimplemented!("no implementation of searching move subsets")
                        }
                        GoOption::Ponder => {
                            infinite = true;
                        },
                        GoOption::WhiteTime(time) => {
                            wtime = Some(time);
                        }
                        GoOption::BlackTime(time) => {
                            btime = Some(time);
                        }
                        GoOption::WhiteInc(inc) => {
                            winc = inc;
                        }
                        GoOption::BlackInc(inc) => {
                            binc = inc;
                        }
                        GoOption::MovesToGo(n) => {
                            movestogo = Some(n);
                        }
                        GoOption::Depth(d) => {
                            searcher.write().unwrap().set_depth(d);
                        }
                        GoOption::Nodes(num) => {
                            *searcher.read().unwrap().limit.nodes_cap.lock().unwrap() = Some(num);
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

                // configure timeout condition
                if infinite {
                    *searcher
                        .read()
                        .unwrap()
                        .limit
                        .search_duration
                        .lock()
                        .unwrap() = None;
                } else if let Some(mt) = movetime {
                    *searcher
                        .read()
                        .unwrap()
                        .limit
                        .search_duration
                        .lock()
                        .unwrap() = Some(mt)
                } else {
                    *searcher
                        .read()
                        .unwrap()
                        .limit
                        .search_duration
                        .lock()
                        .unwrap() = Some(Duration::from_millis(get_search_time(
                        movestogo,
                        (winc, binc),
                        (wtime.unwrap(), btime.unwrap()),
                        game.board().player_to_move,
                    ) as u64));
                }

                searcher.read().unwrap().limit.start().unwrap();

                let cloned_game = game.clone();
                let searcher_new_arc = searcher.clone();
                search_handle = Some(std::thread::spawn(move || {
                    let (m, eval, depth) = searcher_new_arc
                        .read()
                        .unwrap()
                        .evaluate(&cloned_game)
                        .unwrap();
                    print!("{}", UciMessage::BestMove { m, ponder: None });
                    print!(
                        "{}",
                        UciMessage::Info(&[
                            EngineInfo::Score {
                                eval,
                                is_lower_bound: false,
                                is_upper_bound: false
                            },
                            EngineInfo::Depth(depth)
                        ])
                    );
                }));
            }
            UciCommand::Stop => {
                searcher.read().unwrap().limit.stop();
                if let Some(handle) = search_handle {
                    // wait for the previous search to die
                    handle.join().unwrap();
                    search_handle = None;
                }
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
        print!("{}", UciMessage::Info(&[EngineInfo::String(s)]));
    }
}

/// Send out a message to add an option for the frontend.
fn add_option(name: &str, opt: OptionType) {
    print!("{}", UciMessage::Option { name, opt })
}
