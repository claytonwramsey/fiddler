use crate::engine::search::Minimax;
use crate::Engine;
use crate::Game;
use crate::MoveGenerator;
use std::io;

/**
 * A text-based application for running CrabChess.
 */
pub struct CrabchessApp<'a> {
    /**
     * The currently-played game.
     */
    game: Game,
    /**
     * The generator for moves.
     */
    mgen: MoveGenerator,
    /**
     * The currently-running engine to play against.
     */
    engine: Box<dyn Engine + 'a>,
    /**
     * The input stream to receive messages from.
     */
    input_stream: Box<dyn io::Read + 'a>,
    /**
     * The output stream to send messages to.
     */
    output_stream: Box<dyn io::Write + 'a>,
}

/**
 * The set of commands which this command line program can execute.
 */
enum Command {
    /**
     * Quit the currently-running application.
     */
    Quit,
    /**
     * Echo back a string to the output stream.
     */
    Echo(&'static str),
}

impl<'a> CrabchessApp<'a> {
    /**
     * Run the command line application.
     * Will continue running until the user specifies to quit.
     */
    pub fn run(&mut self) {
        let mut has_quit = false;
        let mut user_input = String::new();
        let input_stream = io::stdin();
        let output_stream = io::stdout();
        while !has_quit {
            input_stream.read_line(&mut user_input);

            let command_result = self.parse_command(user_input.as_str());
            let command = match command_result {
                Ok(cmd) => cmd,
                Err(s) => Command::Echo(s),
            };

            match command {
                Quit => has_quit = true,
                _ => self.execute_command(command),
            };
        }
    }

    /**
     * Parse the given text command, and create a new `Command` to describe it.
     * Will return an `Err` if it cannot parse the given command.
     */
    fn parse_command(&self, s: &str) -> Result<Command, &'static str> {
        Err("haven't implemented command parsing yet")
    }

    fn execute_command(&self, c: Command) {
        match c {
            Command::Echo(s) => writeln!(self.output_stream, "{}", s),
        }
    }
}

impl<'a> Default for CrabchessApp<'a> {
    fn default() -> CrabchessApp<'a> {
        CrabchessApp {
            game: Game::default(),
            mgen: MoveGenerator::new(),
            engine: Box::new(Minimax::default()),
            input_stream: Box::new(io::stdin()),
            output_stream: Box::new(io::stdout()),
        }
    }
}
