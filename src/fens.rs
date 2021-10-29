/**
 * A file storing a list of FENs for testing purposes.
 */

#[allow(dead_code)]
/**
 * The FEN of the official starting position for any chess game.
 */
pub const BOARD_START_FEN: &'static str =
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[allow(dead_code)]
/**
 * A board with a black king on H8 and white king on A1, and nothing else.
 * White to  move.
 */
pub const TWO_KINGS_BOARD_FEN: &'static str = "7k/8/8/8/8/8/8/K7 w - - 0 1";

#[allow(dead_code)]
/**
 * A board where White can play exf6 as en passant.
 */
pub const EN_PASSANT_READY_FEN: &'static str =
    "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3";
