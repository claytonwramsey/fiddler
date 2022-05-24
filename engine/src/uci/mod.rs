use std::{fmt, time::Duration};

use crate::{Eval, Move};

mod send;
pub use send::build_message;

mod parse;
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

impl<'a> fmt::Display for UciMessage<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", build_message(self))
    }
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
