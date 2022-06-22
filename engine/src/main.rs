/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

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

//! The main UCI procedure.
//!
//! This code handles the central logic of actually running an engine. We
//! function quickly by constantly listening for new commands from the GUI, and
//! then spinning up threads to execute each command.
//!
//! Many of the details of concurrency required to achieve this are finicky; I
//! am hopeful that we can develop more elegant solutions in the future.

use std::{
    io::stdin,
    sync::{Arc, RwLock},
    thread::JoinHandle,
    time::Duration,
};

use fiddler_base::Game;
use fiddler_engine::{
    evaluate::{static_evaluate, value_delta},
    thread::MainSearch,
    time::get_search_time,
    transposition::TTable,
    uci::{parse_line, EngineInfo, GoOption, OptionType, UciCommand, UciMessage},
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
                println!(
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

                println!("{}", UciMessage::UciOk)
            }
            UciCommand::Debug(new_debug) => {
                // activate or deactivate debug mode
                debug = new_debug;
            }
            UciCommand::IsReady => {
                // we were born ready
                println!("{}", UciMessage::ReadyOk);
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
                    Some(fen) => Game::from_fen(&fen, static_evaluate).unwrap(),
                };
                for m in moves {
                    game.make_move(m, value_delta(game.board(), m));
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
    debug_info(&format!("search time: {:?}", *search_duration_guard), debug);
    drop(search_duration_guard); // prevent deadlock when starting the limit

    searcher_guard.limit.start().unwrap();

    let cloned_game = game.clone();
    let searcher_new_arc = searcher.clone();

    debug_info("spawning main search thread", debug);
    Some(std::thread::spawn(move || {
        let searcher_guard = searcher_new_arc.read().unwrap();
        // this step will block
        debug_info("starting evaluation", debug);
        let search_result = searcher_guard.evaluate(&cloned_game);
        debug_info("finished evaluation", debug);

        match search_result {
            Ok(info) => {
                println!(
                    "{}",
                    UciMessage::BestMove {
                        m: info.pv[0],
                        ponder: info.pv.get(1).copied(),
                    }
                );
            }
            Err(e) => {
                // search failed :(
                // notify the GUI in debug mode, otherwise there's not much we can
                // do
                debug_info(&format!("search failed: {:?}", e), debug);
            }
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
        println!("{}", UciMessage::Info(&[EngineInfo::String(s)]));
    }
}

/// Send out a message to add an option for the frontend.
fn add_option(name: &str, opt: OptionType) {
    println!("{}", UciMessage::Option { name, opt })
}
