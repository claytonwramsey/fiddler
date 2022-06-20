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

use crate::Move;

mod parse;
mod send;
pub use parse::*;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
/// An enum representing the set of all commands that the GUI can send to the
/// engine via UCI.
pub enum UciCommand {
    /// Command given at the start of UCI. The engine must reply with
    /// `UciMessage::Id` message and send the `UciMessage::Option` commands to
    /// tell the GUI which engine commands it supports. After that receipt, the
    /// engine must then send a `UciMessage::Ok` to complete the setup. If not,
    /// the GUI will kill the engine process.
    Uci,
    /// Switch whether the engine should turn on or off debug mode. If true,
    /// the engine should activate debug mode and send info strings to the GUI.
    /// By default, debug mode should be off.
    Debug(bool),
    /// Request an update from the engine as to whether it is ready to proceed.
    /// If the engine is busy thinking, it can wait until it is done thinking
    /// to reply to this. When it is ready, the engine must reply with
    /// `UciMessage::ReadyOk`.
    IsReady,
    /// Set a parameter of the engine, or send a custom command. `name` is the
    /// name of the key for the option, and `value` is an optional parameter
    /// for the given value to set.
    SetOption { name: String, value: Option<String> },
    /// Inform the engine that the next position it will be requested to
    /// evaluate will be from a new game. An engine should not, however, expect
    /// a `NewGame`.
    NewGame,
    /// Update the next position the engine will be asked to evaluate. The
    /// engine should set up the position starting from the given FEN, and then
    /// play the given list of moves to reach the position.
    Position {
        ///
        /// The FEN from which to set up the position. If `fen` is `None`, then
        /// start from the default start position for a normal game of chess.
        ///
        fen: Option<String>,
        ///
        /// The set of moves to play after setting up with the given FEN.
        ///
        moves: Vec<Move>,
    },
    /// A `Go` will always be given after a `Position` command. The options are
    /// given as a table of options.
    Go(Vec<GoOption>),
    /// Stop searching immediately, and when done, reply with a best move and
    /// potentially a ponder.
    Stop,
    /// While the engine was in pondering mode, the player opposing the engine
    /// selected to play the ponder-move. Continue searching, but know that the
    /// player chose the ponder-move.
    PonderHit,
    /// Quit the program as soon as possible.
    Quit,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// The options that can be given for a `UciCommand::Go` command.
pub enum GoOption {
    /// Restrict the search to these moves only.
    SearchMoves(Vec<Move>),
    /// Search in "ponder" mode. The engine must not exit the search until
    /// ordered, no matter the conditions. The last move which was sent in the
    /// previous `UciCommand::Position` should be considered the "ponder-move",
    /// which is the suggested move to consider. When a `UciCommand::PonderHit`
    /// is given, the engine will then execute the ponder command.
    Ponder,
    /// Inform the engine that White has the given number of milliseconds
    /// remaining.
    WhiteTime(u32),
    /// Inform the engine that Black has the given number of milliseconds
    /// remaining.
    BlackTime(u32),
    /// Inform the engine that White has the given number of milliseconds as
    /// their time increment.
    WhiteInc(u32),
    /// Inform the engine that Black has the given number of milliseconds as
    /// their time increment.
    BlackInc(u32),
    /// Inform the engine that there are the given number of moves remaining
    /// until the next time control. If `WhiteTime` and `BlackTime` are not
    /// given, this means the current game is sudden death.
    MovesToGo(u8),
    /// Search with the given depth, looking at only the given number of plies.
    Depth(u8),
    /// Search only the given number of nodes.
    Nodes(u64),
    /// Search for a mate in the given number of moves.
    Mate(u8),
    /// Search for the given number of milliseconds.
    MoveTime(u32),
    /// Search until a `UciCommand::Stop` is given. Do not exit the search
    /// until told to.
    Infinite,
}

pub use send::{EngineInfo, OptionType, UciMessage};
