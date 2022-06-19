use std::{
    io::stdin,
    sync::{Arc, RwLock},
    thread::JoinHandle,
    time::{Duration, Instant},
};

use fiddler_base::Game;
use fiddler_engine::{
    pst::{pst_delta, pst_evaluate},
    thread::MainSearch,
    time::get_search_time,
    transposition::TTable,
    uci::{build_message, parse_line, EngineInfo, GoOption, OptionType, UciCommand, UciMessage},
};

/// Run a UCI engine.
fn main() {
    // whether we are in debug mode
    let mut debug = false;
    let searcher = Arc::new(RwLock::new(MainSearch::new()));
    let mut game = Game::new();
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
                searcher.write().unwrap().config.n_helpers = 15;

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
                        Ok(n) => searcher.write().unwrap().config.n_helpers = n,
                        _ => debug_info("error: illegal parameter for `Thread Count`", debug),
                    },
                },
                _ => debug_info(&format!("error: unknown option key `{}`", name), debug),
            },
            UciCommand::NewGame => {
                game = Game::new();
                // stop previous search
                stop(&searcher, search_handle, debug);
                search_handle = None;
                // clear the transposititon table
                // (in actuality, just make a new one to get around Arc
                // immutability)
                let mut searcher_guard = searcher.write().unwrap();
                let old_bit_size = searcher_guard.ttable.bit_size();
                searcher_guard.ttable = Arc::new(TTable::with_capacity(old_bit_size));
            }
            UciCommand::Position { fen, moves } => {
                game = match fen {
                    None => Game::new(),
                    Some(fen) => Game::from_fen(&fen, pst_evaluate).unwrap(),
                };
                for m in moves {
                    game.make_move(m, pst_delta(game.board(), m));
                }
            }
            UciCommand::Go(opts) => {
                // spawn a new thread to go search
                debug_info("go command received", debug);
                search_handle = go(&opts, &searcher, &game, debug);
            }
            UciCommand::Stop => {
                stop(&searcher, search_handle, debug);
                search_handle = None;
            }
            UciCommand::PonderHit => todo!(),
            UciCommand::Quit => {
                // stop the ongoing search
                stop(&searcher, search_handle, debug);
                break;
            }
        }
    }
}

/// Execute a UCI `go` command. This function has been broken out for
/// readability. Will spawn a new thread to search and return its handle.
fn go(
    opts: &[GoOption],
    searcher: &Arc<RwLock<MainSearch>>,
    game: &Game,
    debug: bool,
) -> Option<JoinHandle<()>> {
    // whether the last move given in the position should be considered the
    // ponder-move
    // unused for now
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

    // do not hold onto guard as option parsing will involve a write
    *searcher.read().unwrap().limit.nodes_cap.lock().unwrap() = None;

    // by default, set the depth to search to be 99, so that the timer is the
    // sole limiting factor
    searcher.write().unwrap().config.depth = 99;
    for opt in opts {
        match opt {
            GoOption::SearchMoves(_) => {
                unimplemented!("no implementation of searching move subsets")
            }
            GoOption::Ponder => {
                infinite = true;
            }
            &GoOption::WhiteTime(time) => {
                wtime = Some(time);
            }
            &GoOption::BlackTime(time) => {
                btime = Some(time);
            }
            &GoOption::WhiteInc(inc) => {
                winc = inc;
            }
            &GoOption::BlackInc(inc) => {
                binc = inc;
            }
            GoOption::MovesToGo(n) => {
                movestogo = Some(*n);
            }
            &GoOption::Depth(d) => {
                searcher.write().unwrap().config.depth = d;
            }
            &GoOption::Nodes(num) => {
                *searcher.read().unwrap().limit.nodes_cap.lock().unwrap() = Some(num);
            }
            GoOption::Mate(_) => unimplemented!(),
            &GoOption::MoveTime(msecs) => {
                movetime = Some(Duration::from_millis(msecs as u64));
            }
            GoOption::Infinite => {
                // on an infinite search, we will go as deep as we want
                // 99 is basically infinite in exponential growth
                searcher.write().unwrap().config.depth = 99;
                infinite = true;
            }
        }
    }

    let searcher_guard = searcher.read().unwrap();
    // configure timeout condition
    let mut search_duration_guard = searcher_guard.limit.search_duration.lock().unwrap();
    if infinite {
        *search_duration_guard = None;
    } else if let Some(mt) = movetime {
        *search_duration_guard = Some(mt)
    } else {
        *search_duration_guard = Some(Duration::from_millis(get_search_time(
            movestogo,
            (winc, binc),
            (wtime.unwrap(), btime.unwrap()),
            game.board().player_to_move,
        ) as u64));
    }
    drop(search_duration_guard); // prevent deadlock when starting the limit

    searcher_guard.limit.start().unwrap();

    let cloned_game = game.clone();
    let searcher_new_arc = searcher.clone();

    debug_info("spawning main search thread", debug);
    Some(std::thread::spawn(move || {
        let searcher_guard = searcher_new_arc.read().unwrap();
        let tic = Instant::now();
        // this step will block
        debug_info("starting evaluation", debug);
        let search_result = searcher_guard.evaluate(&cloned_game);
        debug_info("finished evaluation", debug);
        let elapsed = Instant::now() - tic;

        if let Ok(info) = search_result {
            print!(
                "{}",
                UciMessage::BestMove {
                    m: info.best_move,
                    ponder: None
                }
            );
            let nodes = info.num_nodes_evaluated;
            let nps = nodes * 1000 / (elapsed.as_millis() as u64);

            print!(
                "{}",
                UciMessage::Info(&[
                    EngineInfo::Score {
                        eval: info.eval.in_perspective(cloned_game.board().player_to_move),
                        is_lower_bound: false,
                        is_upper_bound: false
                    },
                    EngineInfo::Depth(info.highest_successful_depth),
                    EngineInfo::Nodes(nodes),
                    EngineInfo::NodeSpeed(nps),
                    EngineInfo::Time(elapsed),
                    EngineInfo::HashFull(searcher_guard.ttable.fill_rate_permill())
                ])
            );
        } else {
            // search failed :(
            // notify the GUI in debug mode, otherwise there's not much we can
            // do
            debug_info("search failed", debug);
        }
    }))
}

/// Notify any active searches to stop, and then block until they are all
/// stopped.
fn stop(searcher: &Arc<RwLock<MainSearch>>, search_handle: Option<JoinHandle<()>>, debug: bool) {
    debug_info("now stopping search", debug);
    searcher.read().unwrap().limit.stop();
    if let Some(handle) = search_handle {
        handle.join().unwrap();
    }
    debug_info("search stopped", debug);
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
