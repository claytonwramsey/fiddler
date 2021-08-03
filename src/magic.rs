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

pub fn make_magic<'a>(table: MagicTable) {

}

pub fn make_rook_magic<'a>(rtable: &'a mut [Magic; 64]) {
    for i in 0..64 {
        //sequence of 1s down the same row as the piece to move, except on the
        //ends
        let row_mask = Bitboard(0x7E << (8 * (i / 8)));
        //sequence of 1s down the same col as the piece to move, except on the
        //ends
        let col_mask = Bitboard(0x0001010101010100 << (i % 8));
        //note: pieces at the end of the travel don't matter, which is why the
        //masks arent' uniform

        //in the col mask or row mask, but not the piece to move
        rtable[i].mask = (row_mask | col_mask) & Bitboard(!(1 << i));
    }
}

pub fn make_bishop_magic<'a>(btable: &'a mut [Magic; 64]) {
    for i in 0..64 {
        //sequence of 1s down the 
    }
}


#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    use super::*;

    #[test]
    fn test_rook_mask() {
        let m_placeholder = Magic{
            mask: Bitboard(0), 
            magic: Bitboard(0), 
            attacks: &Vec::new(), 
            shift: 0,
        };
        let mut outArray = [m_placeholder; 64];
        make_rook_magic(&mut outArray);
        //println!("{:064b}", outArray[0].mask.0);
        assert_eq!(outArray[00].mask, Bitboard(0x000101010101017E));
        
        //println!("{:064b}", outArray[4].mask.0);
        assert_eq!(outArray[04].mask, Bitboard(0x001010101010106E));
        
        //println!("{:064b}", outArray[36].mask.0);
        assert_eq!(outArray[36].mask, Bitboard(0x0010106E10101000));
    }
}