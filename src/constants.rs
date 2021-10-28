/**
 * Total number of piece types. TODO: should this be moved to piece.rs?
 */
pub const NUM_PIECE_TYPES: usize = 6;

/**
 * The names of the files on the board.
 */
pub const FILE_NAMES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

/**
 * The names of the ranks on the board.
 */
pub const RANK_NAMES: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];

/**
 * The color of a player (0 for white, 1 for black)
 */
pub type Color = usize;

/**
 * Represents White as a player, and can be used to index a Board's color table.
 */
pub const WHITE: Color = 0;

/**
 * Represents Black as a player, and can be used to index a Board's color table.
 */
pub const BLACK: Color = 1;

/**
 * A spare sentinel color that can be given when WHITE or BLACK are invalid 
 * colors.
 */
pub const NO_COLOR: Color = 2;
