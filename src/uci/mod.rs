use crate::base::Move;

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
///
/// An enum representing the set of all commands that the GUI can send to the
/// engine via UCI.
///
pub enum UciCommand<'a> {
    ///
    /// Command given at the start of UCI. The engine must reply with
    /// `UciMessage::Id` message and send the `UciMessage::Option` commands to
    /// tell the GUI which engine commands it supports. After that receipt, the
    /// engine must then send a `UciMessage::Ok` to complete the setup. If not,
    /// the GUI will kill the engine process.
    ///
    Uci,
    ///
    /// Switch whether the engine should turn on or off debug mode. If true,
    /// the engine should activate debug mode and send info strings to the GUI.
    /// By default, debug mode should be off.
    ///
    Debug(bool),
    ///
    /// Request an update from the engine as to whether it is ready to proceed.
    /// If the engine is busy thinking, it can wait until it is done thinking
    /// to reply to this. When it is ready, the engine must reply with
    /// `UciMessage::ReadyOk`.
    ///
    IsReady,
    ///
    /// Set a parameter of the engine, or send a custom command. `name` is the
    /// name of the key for the option, and `value` is an optional parameter
    /// for the given value to set.
    ///
    SetOption {
        name: &'a str,
        value: Option<&'a str>,
    },
    ///
    /// A command that gives the engine login information requested from the
    /// GUI. This will typically be a reply to a request for registration from
    /// the GUI. `name` is a username, and `code` is a password or login code.
    /// If both options are `None`, then this should be seen as a notification
    /// that the GUI will log in later.
    ///
    Register {
        name: Option<&'a str>,
        code: Option<&'a str>,
    },
    ///
    /// Inform the engine that the next position it will be requested to
    /// evaluate will be from a new game. An engine should not, however, expect
    /// a `NewGame`.
    ///
    NewGame,
    ///
    /// Update the next position the engine will be asked to evaluate. The
    /// engine should set up the position starting from the given FEN, and then
    /// play the given list of moves to reach the position.
    ///
    Position {
        ///
        /// The FEN from which to set up the position. If `fen` is `None`, then
        /// start from the default start position for a normal game of chess.
        ///
        fen: Option<&'a str>,
        ///
        /// The set of moves to play after setting up with the given FEN.
        ///
        moves: &'a [Move],
    },
    ///
    /// A `Go` will always be given after a `Position` command. The options are
    /// given as a table of options.
    ///
    Go(&'a [GoOption<'a>]),
    ///
    /// While the engine was in pondering mode, the player opposing the engine selected to play the ponder-move. Continue searching, but know that the player chose the ponder-move.
    PonderHit,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
///
/// The options that can be given for a `UciCommand::Go` command.
///
pub enum GoOption<'a> {
    ///
    /// Restrict the search to these moves only.
    ///
    SearchMoves(&'a [Move]),
    ///
    /// Search in "ponder" mode. The engine must not exit the search until
    /// ordered, no matter the conditions. The last move which was sent in the
    /// previous `UciCommand::Position` should be considered the "ponder-move",
    /// which is the suggested move to consider. When a `UciCommand::PonderHit`
    /// is given, the engine will then execute the ponder command.
    ///
    Ponder,
    ///
    /// Inform the engine that White has the given number of milliseconds
    /// remaining.
    ///
    WhiteTime(u32),
    ///
    /// Inform the engine that Black has the given number of milliseconds
    /// remaining.
    ///
    BlackTime(u32),
    ///
    /// Inform the engine that White has the given number of milliseconds as
    /// their time increment.
    ///
    WhiteInc(u32),
    ///
    /// Inform the engine that Black has the given number of milliseconds as
    /// their time increment.
    ///
    BlackInc(u32),
    ///
    /// Inform the engine that there are the given number of moves remaining
    /// until the next time control. If `WhiteTime` and `BlackTime` are not
    /// given, this means the current game is sudden death.
    ///
    MovesToGo(u8),
    ///
    /// Search with the given depth, looking at only the given number of plies.
    ///
    Depth(u8),
    ///
    /// Search only the given number of nodes.
    ///
    Nodes(u64),
    ///
    /// Search for a mate in the given number of moves.
    ///
    Mate(u8),
    ///
    /// Search for the given number of milliseconds.
    ///
    MoveTime(u32),
    ///
    /// Search until a `UciCommand::Stop` is given. Do not exit the search
    /// until told to.
    ///
    Infinite,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
///
/// The set of messages that the engine can send to the GUI.
///
pub enum UciMessage<'a> {
    ///
    /// The engine identifies itself. Must be sent after receiving a
    /// `UciCommand::Uci` message.
    ///
    Id {
        ///
        /// The name of the engine.
        ///
        name: Option<&'a str>,
        ///
        /// The author of the engine.
        ///
        author: Option<&'a str>,
    },
    ///
    /// Request that the GUI display an option to the user.
    /// Not to be confused with the standard `Option`.
    ///
    Option,
    UciOk,
    ReadyOk,
}

///
/// A trait representing something that can send and receive messages as if it
/// is a UCI GUI.
///
pub trait UciGui {
    ///
    /// Send a message to the UCI GUI.
    ///
    fn send(&mut self, message: UciMessage);
}

///
/// A trait representing an engine for UCI. It must be able to tolerate
/// messages from the UCI GUI.
///
pub trait UciModel {
    ///
    /// Receive a Uci command. The processor must then send all its replies to
    /// the given GUI.
    ///
    fn receive(&mut self, command: UciCommand, gui: &dyn UciGui);
}
