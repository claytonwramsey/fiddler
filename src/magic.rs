use crate::bitboard::Bitboard;
use crate::square::Square;
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

fn make_rook_magic<'a>(rtable: &'a mut [Magic; 64]) {
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
        //xor operation will remove the square the piece is on
        rtable[i].mask = row_mask ^ col_mask;
    }
}

//diagonal going from A1 to H8
const MAIN_DIAG: Bitboard = Bitboard(0x8040201008040201);

//diagonal going from A8 to H1
const ANTI_DIAG: Bitboard = Bitboard(0x0102040810204080);

fn make_bishop_magic<'a>(btable: &'a mut [Magic; 64]) {
    for i in 0..64 {

        btable[i].mask = get_bishop_mask(Square(i as u8));
    }
}

fn get_bishop_mask(sq: Square) -> Bitboard{
    //thank u chessprogramming wiki for this code
    let i = sq.0 as i32;
    let main_diag: i32 =  8 * (i & 7) - (i as i32 & 56);
    let main_lshift = -main_diag & ( main_diag >> 31);
    let main_rshift =  main_diag & (-main_diag >> 31);
    let main_diag_mask = (MAIN_DIAG >> main_rshift) << main_lshift;

    let anti_diag: i32 = 56 - 8 * (i & 7) - (i & 56);
    let anti_lshift = -anti_diag & (anti_diag >> 31);
    let anti_rshift = anti_diag & (-anti_diag >> 31);
    let anti_diag_mask = (ANTI_DIAG >> anti_rshift) << anti_lshift;

    return main_diag_mask ^ anti_diag_mask;
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
        let mut rtable = [m_placeholder; 64];
        make_rook_magic(&mut rtable);
        //println!("{:064b}", rtable[0].mask.0);
        assert_eq!(rtable[00].mask, Bitboard(0x000101010101017E));
        
        //println!("{:064b}", rtable[4].mask.0);
        assert_eq!(rtable[04].mask, Bitboard(0x001010101010106E));
        
        //println!("{:064b}", rtable[36].mask.0);
        assert_eq!(rtable[36].mask, Bitboard(0x0010106E10101000));
    }

    #[test]
    fn test_bishop_mask() {
        let m_placeholder = Magic{
            mask: Bitboard(0), 
            magic: Bitboard(0), 
            attacks: &Vec::new(), 
            shift: 0,
        };
        let mut btable = [m_placeholder; 64];
        make_bishop_magic(&mut btable);
        //println!("{:064b}", btable[0].mask.0);
        assert_eq!(btable[00].mask, Bitboard(0x8040201008040200));
        
        //println!("{:064b}", btable[4].mask.0);
        assert_eq!(btable[04].mask, Bitboard(0x0000000182442800));
        
        //println!("{:064b}", btable[36].mask.0);
        assert_eq!(btable[36].mask, Bitboard(0x8244280028448201));
    }
}