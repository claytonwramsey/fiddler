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

//! The main UCI procedure.
//!
//! This code handles the central logic of actually running an engine.
//! To keep the engine responsive, a new thread is created to process each time-intensive command
//! sent from the GUI.
//!
//! Many of the details of concurrency required to achieve this are finicky; I am hopeful that we
//! can develop more elegant solutions in the future.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{
    io::stdin,
    sync::RwLock,
    thread::{scope, Scope, ScopedJoinHandle},
    time::Duration,
};

use fiddler::{
    base::{game::Game, Color, Move},
    engine::{
        perft::perft,
        thread::MainSearch,
        time::get_search_time,
        uci::{Command, GoOption},
    },
};

/// The default size of the transposition table.
const DEFAULT_HASH_SIZE_MB: usize = 500;

/// Run the Fiddler UCI engine.
fn main() {
    // whether we are in debug mode
    let mut debug = false;
    let searcher = RwLock::new(MainSearch::new());
    let mut game = Game::default();
    searcher.write().unwrap().config.n_helpers = 0;
    searcher
        .write()
        .unwrap()
        .ttable
        .resize(DEFAULT_HASH_SIZE_MB);

    scope(|s| {
        let mut search_handle = None;
        loop {
            let mut buf = String::new();
            if stdin().read_line(&mut buf).is_err() {
                debug_info("failed to read line", debug);
            };
            let command = match Command::parse_line(&buf) {
                Ok(cmd) => cmd,
                Err(e) => {
                    // print out the error to the frontend and continue on to the
                    // next line
                    debug_info(&format!("failed to parse line: {e}"), debug);
                    continue;
                }
            };
            match command {
                Command::Uci => {
                    // identify the engine
                    println!("id name Fiddler 0.1.0 ({})", env!("GIT_HASH"));
                    println!("id author Clayton Ramsey");

                    // add options
                    println!(
                        "option name UCI_EngineAbout type string default \
                        Fiddler: Copyright (C) 2021-2023 Clayton Ramsey. \
                        This program comes with ABSOLUTELY NO WARRANTY. \
                        Licensed under the GNU GPLv3. \
                        Source code at https://github.com/claytonwramsey/fiddler."
                    );
                    println!("option name Thread Count type spin default 1 min 1 max 255");
                    println!(
                        "option name Hash type spin default {DEFAULT_HASH_SIZE_MB} min 0 max 128000"
                    );
                    println!("uciok");
                }
                Command::Debug(new_debug) => {
                    // activate or deactivate debug mode
                    debug = new_debug;
                }
                Command::IsReady => {
                    // we were born ready
                    println!("readyok");
                }
                Command::SetOption { name, value } => match name.as_str() {
                    "Thread Count" => match value {
                        None => debug_info("error: no value given for number of threads", debug),
                        Some(num_str) => match num_str.parse::<u8>() {
                            Ok(n) => searcher.write().unwrap().config.n_helpers = n - 1,
                            _ => debug_info("error: illegal parameter for `Thread Count`", debug),
                        },
                    },
                    "Hash" => match value {
                        None => debug_info("error: no value given for hashsize", debug),
                        Some(size_str) => match size_str.parse::<usize>() {
                            Ok(size_mb) => {
                                searcher.write().unwrap().ttable.resize(size_mb);
                            }
                            _ => debug_info("error: illegal parameter for hash size", debug),
                        },
                    },
                    _ => debug_info(&format!("error: unknown option key `{name}`"), debug),
                },
                Command::NewGame => {
                    game = Game::new();
                    // stop previous search
                    stop(&searcher, search_handle, debug);
                    search_handle = None;
                    // clear the transposititon table
                    let mut searcher_guard = searcher.write().unwrap();
                    searcher_guard.ttable.clear();
                }
                Command::Position { fen, moves } => match parse_game_str(fen.as_deref(), moves) {
                    Ok(new_game) => {
                        game = new_game;
                        debug_info(&format!("current game: {game}"), debug);
                    }
                    Err(e) => {
                        debug_info(&format!("invalid game: {e}"), debug);
                    }
                },
                Command::Go(opts) => {
                    // spawn a new thread to go search
                    debug_info("go command received", debug);
                    search_handle = go(&opts, &searcher, &game, s, debug);
                }
                Command::Stop => {
                    stop(&searcher, search_handle, debug);
                    search_handle = None;
                }
                Command::PonderHit => {
                    // mark that the timer has now begun
                }
                Command::Quit => {
                    // stop the ongoing search
                    stop(&searcher, search_handle, debug);
                    break;
                }
            }
        }
    });
}

fn parse_game_str(fen: Option<&str>, moves: Vec<Move>) -> Result<Game, &str> {
    let mut game = fen.map_or_else(|| Ok(Game::new()), Game::from_fen)?;
    for m in moves {
        game.try_move(m).map_err(|()| "illegal move")?;
    }

    Ok(game)
}

#[allow(clippy::too_many_lines)]
/// Execute a UCI `go` command.
/// This function has been broken out for readability.
/// Will spawn a new thread to search and return its handle.
fn go<'a>(
    opts: &[GoOption],
    searcher: &'a RwLock<MainSearch>,
    game: &Game,
    thread_scope: &'a Scope<'a, '_>,
    debug: bool,
) -> Option<ScopedJoinHandle<'a, ()>> {
    let (mut wtime, mut btime) = (None, None); // time remaining for players
    let (mut winc, mut binc) = (0, 0); // increments. by default assumed to be zero

    // number of moves until increment achieved. if `None`, there is no increment.
    let mut movestogo = None;
    let mut infinite = false; // whether to search infinitely
    let mut movetime = None;
    let mut perft_depth = None;

    // do not hold onto guard as option parsing will involve a write
    *searcher.read().unwrap().limit.nodes_cap.write().unwrap() = None;

    // by default, set the depth to search to be 99, so that the timer is the sole limiting factor
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
                *searcher.read().unwrap().limit.nodes_cap.write().unwrap() = Some(num);
            }
            GoOption::Mate(_) => unimplemented!(),
            &GoOption::MoveTime(msecs) => {
                movetime = Some(Duration::from_millis(u64::from(msecs)));
            }
            GoOption::Infinite => {
                // on an infinite search, we will go as deep as we want
                // 99 is basically infinite in exponential growth
                searcher.write().unwrap().config.depth = 99;
                infinite = true;
            }
            GoOption::Perft(n) => {
                perft_depth = Some(n);
            }
        }
    }

    // perft
    if let Some(d) = perft_depth {
        let mut cloned_game = game.clone();
        perft(&mut cloned_game, *d);
        return None;
    }
    let (increment, remaining) = match game.meta().player {
        Color::White => (winc, wtime),
        Color::Black => (binc, btime),
    };
    let sguard = searcher.read().unwrap();
    // configure timeout condition
    let mut search_duration_guard = sguard.limit.search_duration.lock().unwrap();
    if infinite {
        *search_duration_guard = None;
    } else if let Some(mt) = movetime {
        *search_duration_guard = Some(mt);
    } else if let Some(rem) = remaining {
        *search_duration_guard = Some(Duration::from_millis(if rem > 0 {
            #[allow(clippy::cast_sign_loss)]
            u64::from(get_search_time(movestogo, increment, rem as u32))
        } else {
            100
        }));
    } else {
        *search_duration_guard = None;
    }
    debug_info(&format!("search time: {:?}", *search_duration_guard), debug);
    drop(search_duration_guard); // prevent deadlock when starting the limit

    sguard.limit.start();
    drop(sguard);

    let cloned_game = game.clone();

    debug_info("spawning main search thread", debug);
    Some(thread_scope.spawn(move || {
        let searcher_guard = searcher.read().unwrap();
        // this step will block
        debug_info("starting evaluation", debug);
        let search_result = searcher_guard.evaluate(&cloned_game);
        debug_info("finished evaluation", debug);

        match search_result {
            Ok(info) => {
                print!("bestmove {}", info.pv[0].to_uci());
                if let Some(pondermove) = info.pv.get(1).copied() {
                    print!(" ponder {}", pondermove.to_uci());
                }
                println!();
            }
            Err(e) => {
                // search failed :(
                // notify the GUI in debug mode, otherwise there's not much we can do
                debug_info(&format!("search failed: {e:?}"), debug);
            }
        }
        drop(searcher_guard);
        // clean up after ourselves by aging up the transposition table.
        // this prevents the table from being polluted with useless entries.
        // (~30 El0)
        searcher.write().unwrap().ttable.age_up(3);
    }))
}

/// Notify any active searches to stop, and then block until they are all stopped.
fn stop(searcher: &RwLock<MainSearch>, search_handle: Option<ScopedJoinHandle<()>>, debug: bool) {
    debug_info("now stopping search", debug);
    searcher.read().unwrap().limit.stop();
    if let Some(handle) = search_handle {
        handle.join().unwrap();
    }
    debug_info("search stopped", debug);
}

/// Print out a debug info message to the console.
/// Will have no effect if `debug` is `false`.
fn debug_info(s: &str, debug: bool) {
    if debug {
        println!("info string {s}");
    }
}
