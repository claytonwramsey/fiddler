use crate::bitboard::Bitboard;
use crate::square::Square;
use std::vec::Vec;

pub struct MagicTable<'a> {
    pub rook_magic: [Magic<'a>; 64],
    pub bishop_magic: [Magic<'a>; 64]
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

//number of times to trie generating magics
const NUM_MAGIC_TRIES: u64 = 1_000_000;

//diagonal going from A1 to H8
const MAIN_DIAG: Bitboard = Bitboard(0x8040201008040201);

//diagonal going from A8 to H1
const ANTI_DIAG: Bitboard = Bitboard(0x0102040810204080);

#[allow(dead_code)]
pub fn make_magic<'a>(table: &mut MagicTable) {
    make_magic_helper(&mut table.rook_magic, true);
    make_magic_helper(&mut table.bishop_magic, false);
}

fn make_magic_helper<'a>(table: &'a mut [Magic; 64], is_rook: bool) {
    for i in 0..64 {
        if is_rook {
            table[i].mask = get_rook_mask(Square(i as u8));
        } else {
            table[i].mask = get_bishop_mask(Square(i as u8));
        }
        //number of squares where occupancy matters
        let num_points = table[i].mask.0.count_ones();

        //we know that there are at most 12 pieces that will matter when it
        //comes to attack lookups
        let mut occupancies = [Bitboard(0); 1 << 12];
        let mut attacks = [0; 1 << 12];

        //compute every possible occupancy arrangement for attacking 
        for j in 0..(1 << num_points) {
            occupancies[j] = index_to_occupancy(j, num_points, table[i].mask);
            //TODO compute attacks and then try generating magics for this attack
        }
    }
}


//Create the mask for the relevant bits in magic of a rook
fn get_rook_mask(sq: Square) -> Bitboard {
    let index = sq.0 as i8;
    //sequence of 1s down the same row as the piece to move, except on the
    //ends
    let row_mask = Bitboard(0x7E << (8 * (index / 8)));
    //sequence of 1s down the same col as the piece to move, except on the
    //ends
    let col_mask = Bitboard(0x0001010101010100 << (index % 8));
    //note: pieces at the end of the travel don't matter, which is why the
    //masks arent' uniform

    //in the col mask or row mask, but not the piece to move
    //xor operation will remove the square the piece is on
    return row_mask ^ col_mask;
}

//Create the mask for the relevant bits in magic of a bishop
fn get_bishop_mask(sq: Square) -> Bitboard{
    //thank u chessprogramming wiki for this code
    let i = sq.0 as i32;
    let main_diag =  8 * (i & 7) - (i as i32 & 56);
    let main_lshift = -main_diag & ( main_diag >> 31);
    let main_rshift =  main_diag & (-main_diag >> 31);
    let main_diag_mask = (MAIN_DIAG >> main_rshift) << main_lshift;

    let anti_diag = 56 - 8 * (i & 7) - (i & 56);
    let anti_lshift = -anti_diag & (anti_diag >> 31);
    let anti_rshift = anti_diag & (-anti_diag >> 31);
    let anti_diag_mask = (ANTI_DIAG >> anti_rshift) << anti_lshift;

    return main_diag_mask ^ anti_diag_mask;
}

//Given some mask, create the occupancy bitboard according to this index
fn index_to_occupancy(index: usize, num_points: u32, mask: Bitboard) -> Bitboard {
    let mut result = Bitboard(0);
    let mut editable_mask = mask;
    //go from right to left in the bits of num_points,
    //and add an occupancy if something is there
    for i in 0..num_points {
        let shift_size = editable_mask.0.trailing_zeros();
        //make a bitboard which only occupies the rightmost square
        let occupier = Bitboard(1 << shift_size);
        //remove the occupier from the mask
        editable_mask &= !occupier;
        if (index & (1 << i)) != 0 {
            //the bit corresponding to the occupier is nonzero
            result |= occupier;
        }

    }

    return result;
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
        make_magic_helper(&mut rtable, true);
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
        make_magic_helper(&mut btable, false);
        //println!("{:064b}", btable[0].mask.0);
        assert_eq!(btable[00].mask, Bitboard(0x8040201008040200));
        
        //println!("{:064b}", btable[4].mask.0);
        assert_eq!(btable[04].mask, Bitboard(0x0000000182442800));
        
        //println!("{:064b}", btable[36].mask.0);
        assert_eq!(btable[36].mask, Bitboard(0x8244280028448201));
    }
}