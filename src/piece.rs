use std::fmt::{Display, Formatter, Result};


#[derive(Copy, Clone, PartialEq, Eq, Debug)]
/**
 * A struct representing the type of a piece. Only the rightmost 3 bits are
 * used.
 */
pub struct PieceType(pub u8);

impl PieceType {

        
    /**
     * Total number of piece types.
     */
    pub const NUM_TYPES: usize = 6;

    #[allow(dead_code)]
    pub const NO_TYPE: PieceType = PieceType(7);

    //these piece types should match that of the indices in board::bb_indices
    #[allow(dead_code)]
    pub const PAWN: PieceType = PieceType(0);
    #[allow(dead_code)]
    pub const KNIGHT: PieceType = PieceType(1);
    #[allow(dead_code)]
    pub const BISHOP: PieceType = PieceType(2);
    #[allow(dead_code)]
    pub const ROOK: PieceType = PieceType(3);
    #[allow(dead_code)]
    pub const QUEEN: PieceType = PieceType(4);
    #[allow(dead_code)]
    pub const KING: PieceType = PieceType(5);

    pub const ALL_TYPES: [PieceType; PieceType::NUM_TYPES] = [
        PieceType::PAWN,
        PieceType::KNIGHT,
        PieceType::BISHOP,
        PieceType::ROOK,
        PieceType::QUEEN,
        PieceType::KING,
    ];

    pub const PROMOTE_TYPES: [PieceType; 4] = [
        PieceType::KNIGHT,
        PieceType::BISHOP,
        PieceType::ROOK,
        PieceType::QUEEN,
    ];

    /**
     * Get the FEN code of this piece as an uppercase string.
     */
    pub fn get_code(self) -> &'static str {
        match self {
            PieceType::NO_TYPE => "_",
            PieceType::PAWN => "P",
            PieceType::KNIGHT => "N",
            PieceType::BISHOP => "B",
            PieceType::ROOK => "R",
            PieceType::QUEEN => "Q",
            PieceType::KING => "K",
            _ => "?",
        }
    }

    /**
     * Given a FEN character, convert it to a piece type. Must be uppercase.
     */
    pub fn from_code(c: char) -> PieceType {
        match c {
            'P' => PieceType::PAWN,
            'N' => PieceType::KNIGHT,
            'B' => PieceType::BISHOP,
            'R' => PieceType::ROOK,
            'Q' => PieceType::QUEEN,
            'K' => PieceType::KING,
            _ => PieceType::NO_TYPE,
        }
    }
}

impl Display for PieceType {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.get_code())
    }
}
