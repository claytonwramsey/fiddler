pub const NUM_PIECE_TYPES: usize = 6;

pub const FILE_NAMES: [&str; 8] = ["a", "b", "c", "d", "e", "f", "g", "h"];

pub const RANK_NAMES: [&str; 8] = ["1", "2", "3", "4", "5", "6", "7", "8"];

pub type Color = usize;
//should match indices in board::bb_indices
pub const WHITE: Color = 0;
pub const BLACK: Color = 1;
pub const NO_COLOR: Color = 2;
