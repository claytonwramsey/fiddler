use crate::bitboard::Bitboard;
use std::vec::Vec;

pub struct MagicTable {
    pub rookMagic: [Magic; 64],
    pub bishopMagic: [Magic; 64]
}

//All the information needed to compute magic attacks coming from one square.
pub struct Magic {
    //A mask which, when &ed with the occupancy bitboard, will give only the
    //bits that matter when computing moves.
    pub mask: Bitboard,
    //The magic number to multiply to hash the current board effectively
    pub magic: Bitboard,
    pub attacks: Vec<Bitboard>,
    pub shift: u8
}

pub fn makeRookMagic() -> [Magic; 64] {
    let m_placeholder = Magic{
        mask: Bitboard(0), 
        magic: Bitboard(0), 
        attacks: Vec::new(), 
        shift: 0,
    };
    let mut outArray = [m_placeholder; 64];
    let mut obsBbValue = 1u64;
    for i in 0..64 {
        //make the board corresponding to this

    }
    return outArray;
}

pub const fn makeBishopMagic() -> [Magic; 64] {

}

