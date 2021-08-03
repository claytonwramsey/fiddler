use crate::bitboard::Bitboard;
use std::vec::Vec;

pub struct MagicTable<'a> {
    pub rookMagic: [Magic<'a>; 64],
    pub bishopMagic: [Magic<'a>; 64]
}

//All the information needed to compute magic attacks coming from one square.
#[derive(Clone, Copy)]
pub struct Magic<'a> {
    //A mask which, when &ed with the occupancy bitboard, will give only the
    //bits that matter when computing moves.
    pub mask: Bitboard,
    //The magic number to multiply to hash the current board effectively
    pub magic: Bitboard,
    pub attacks: &'a Vec<Bitboard>,
    pub shift: u8
}

//remove bits on the outside ring of the board
const RING_MASK: Bitboard = Bitboard(0x007E7E7E7E7E7E00);

pub fn make_magic<'a>(table: MagicTable) {

}

pub fn make_rook_magic<'a>(rookTable :&'a mut [Magic; 64]) {
    for i in 0..64 {
        //sequence of 1s down the same row as the piece to move
        let row_mask = Bitboard(0xFF << (8 * (i / 8)));
        //sequence of 1s down the same col as the piece to move
        let col_mask = Bitboard(0x0101010101010101 << (i % 8));
        //in the col mask or row mask, but not the piece to move
        let mask = (row_mask | col_mask) & Bitboard(!(1 << i)) & RING_MASK;
        rookTable[i].mask = mask;
    }
}

pub fn make_bishop_magic<'a>(bishopTable: &'a mut [Magic; 64]) {
    for i in 0..64 {
        //sequence of 1s down the 
    }
}


#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    use super::*;

    #[test]
    fn testRookMagicMask() {
        let m_placeholder = Magic{
            mask: Bitboard(0), 
            magic: Bitboard(0), 
            attacks: &Vec::new(), 
            shift: 0,
        };
        let mut outArray = [m_placeholder; 64];
        make_rook_magic(&mut outArray);
        //println!("{:064b}", outArray[0].mask.0);
        assert_eq!(outArray[0].mask, Bitboard(0x01010101010101FE));
        assert_eq!(outArray[4].mask, Bitboard(0x10101010101010EF));
        assert_eq!(outArray[36].mask, Bitboard(0x101010EF10101010));
    }
}