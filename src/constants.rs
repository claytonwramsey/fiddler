pub const NUM_PIECE_TYPES: usize = 6;

pub const FILE_NAMES: [&str; 8] = [
    "h",
    "g",
    "f",
    "e",
    "d",
    "c",
    "b",
    "a"
];

pub const RANK_NAMES: [&str; 8] = [
    "8",
    "7",
    "6",
    "5",
    "4",
    "3",
    "2",
    "1"
];


pub type Color = usize;
//should match indices in board::bb_indices
pub const WHITE: Color = 0;
pub const BLACK: Color = 1;