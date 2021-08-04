use crate::bitboard::Bitboard;
use crate::square::Square;
use crate::direction::{Direction, ROOK_DIRECTIONS, BISHOP_DIRECTIONS};
use rand::thread_rng;
use rand::Rng;
use std::vec::Vec;

#[derive(Clone)]
pub struct MagicTable {
    pub rook_magic: [Magic; 64],
    pub bishop_magic: [Magic; 64]
}

//All the information needed to compute magic attacks coming from one square.
#[derive(Clone)]
pub struct Magic {
    //A mask which, when &ed with the occupancy bitboard, will give only the
    //bits that matter when computing moves.
    pub mask: Bitboard,
    //The magic number to multiply to hash the current board effectively
    pub magic: Bitboard,
    pub attacks: Vec<Bitboard>,
    pub shift: u8
}

//number of times to trie generating magics
const NUM_MAGIC_TRIES: u64 = 10_000;

//diagonal going from A1 to H8
const MAIN_DIAG: Bitboard = Bitboard(0x8040201008040201);

//diagonal going from A8 to H1
const ANTI_DIAG: Bitboard = Bitboard(0x0102040810204080);

const ROOK_SHIFTS: [u8; 64] = [
  12, 11, 11, 11, 11, 11, 11, 12,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  11, 10, 10, 10, 10, 10, 10, 11,
  12, 11, 11, 11, 11, 11, 11, 12
];

const BISHOP_SHIFTS: [u8; 64] = [
  6, 5, 5, 5, 5, 5, 5, 6,
  5, 5, 5, 5, 5, 5, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 9, 9, 7, 5, 5,
  5, 5, 7, 7, 7, 7, 5, 5,
  5, 5, 5, 5, 5, 5, 5, 5,
  6, 5, 5, 5, 5, 5, 5, 6
];


#[allow(dead_code)]
pub fn make_magic<'a>(table: &mut MagicTable) {
    make_magic_helper(&mut table.rook_magic, true);
    make_magic_helper(&mut table.bishop_magic, false);
}

#[inline]
fn compute_magic_key(occupancy: Bitboard, magic: Bitboard, shift: u8) -> usize {
    ((occupancy * magic).0 >> (64 - shift)) as usize
}

fn make_magic_helper<'a>(table: &'a mut [Magic; 64], is_rook: bool) {
    for i in 0..64 {
        //square of the piece making attacks
        let sq = Square(i as u8);
        if is_rook {
            table[i].mask = get_rook_mask(sq);
            table[i].shift = ROOK_SHIFTS[i];
        } else {
            table[i].mask = get_bishop_mask(sq);
            table[i].shift = BISHOP_SHIFTS[i];
        }
        //number of squares where occupancy matters
        let num_points = table[i].mask.0.count_ones();

        //we know that there are at most 12 pieces that will matter when it
        //comes to attack lookups
        let mut occupancies = [Bitboard(0); 1 << 12];
        let mut attacks = [Bitboard(0); 1 << 12];

        //compute every possible occupancy arrangement for attacking 
        for j in 0..(1 << num_points) {
            occupancies[j] = index_to_occupancy(j, table[i].mask);
            //compute attacks
            if is_rook {
                attacks[j] = directional_attacks(sq, ROOK_DIRECTIONS, occupancies[j])
            } else {
                attacks[j] = directional_attacks(sq, BISHOP_DIRECTIONS, occupancies[j])
            }
        }
        //try random magics until one works
        let mut found_magic = false;
        let mut used;
        for _ in 0..NUM_MAGIC_TRIES {
            let magic = random_sparse_bitboard();

            //repopulate the usage table with zeros
            used = [Bitboard(0); 1<< 12];
            found_magic = true;
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], magic, table[i].shift);
                if used[key] == Bitboard(0) {
                    used[key] = attacks[j];
                }
                else if used[key] != attacks[j]{
                    found_magic = false;
                    break;
                }
            }

            //found a working magic, we're done here
            if found_magic {
                println!("Found magic for square {}: {}", sq, magic);
                table[i].magic = magic;
                break;
            }
        }
        if !found_magic {
            println!("FAILED to find magic on square {}. is rook? {}", sq, is_rook);
        }
        else {
            // found a magic, populate the attack vector
            table[i].attacks.resize(1 << table[i].shift, Bitboard(0));
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], table[i].magic, table[i].shift);
                table[i].attacks[key] = attacks[j];
            }
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
fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut result = Bitboard(0);
    let num_points = mask.0.count_ones();

    //comment this out if you think you're clever
    /*if index >= (1 << num_points) + 1{
        return result;
    }*/

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

fn directional_attacks(sq: Square, dirs: [Direction; 4], occupancy: Bitboard) -> Bitboard {
    let mut result = Bitboard(0);
    for dir in dirs {
        let mut current_square = sq;
        for _ in 0..7 {
            current_square += dir;
            if !current_square.is_inbounds() {
                break;
            }
            //current square is occupied
            result |= Bitboard::from(current_square);
            if occupancy.is_square_occupied(current_square){
                break;
            }
        }
    }
    return result;
}

#[inline]
fn random_sparse_bitboard() -> Bitboard {
    let mut result = Bitboard(thread_rng().gen::<u64>());
    for _ in 0..2 {   
        result &= Bitboard(thread_rng().gen::<u64>());
    }
    return result;
}

#[cfg(test)]
mod tests {
    #[allow(dead_code)]
    use super::*;

    #[allow(dead_code)]
    use crate::square::*;
    
    use std::mem::{MaybeUninit, transmute};

    #[test]
    fn test_rook_mask() {
        let mut rtable = {
            let mut data: [MaybeUninit<Magic>; 64] = unsafe {
                MaybeUninit::uninit().assume_init()
            };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(Magic{
                    mask: Bitboard(0), 
                    magic: Bitboard(0), 
                    attacks: Vec::new(), 
                    shift: 0,
                });
            }
            unsafe { transmute::<_, [Magic; 64]>(data)  }
        };
        make_magic_helper(&mut rtable, true);
        println!("{:064b}", rtable[0].mask.0);
        assert_eq!(rtable[00].mask, Bitboard(0x000101010101017E));
        
        println!("{:064b}", rtable[4].mask.0);
        assert_eq!(rtable[04].mask, Bitboard(0x001010101010106E));
        
        println!("{:064b}", rtable[36].mask.0);
        assert_eq!(rtable[36].mask, Bitboard(0x0010106E10101000));
    }

    #[test]
    fn test_bishop_mask() {
        //println!("{:064b}", btable[0].mask.0);
        assert_eq!(get_bishop_mask(A1), Bitboard(0x8040201008040200));
    
        //println!("{:064b}", btable[4].mask.0);
        assert_eq!(get_bishop_mask(E1), Bitboard(0x0000000182442800));
        
        //println!("{:064b}", btable[36].mask.0);
        assert_eq!(get_bishop_mask(E5), Bitboard(0x8244280028448201));

        println!("{:064b}", get_bishop_mask(F4).0);

    }

    #[test]
    fn test_index_to_occupancy() {
        let mask = Bitboard(0b1111);
        for i in 0..16 {
            let occu = index_to_occupancy(i, mask);
            assert_eq!(occu, Bitboard(i as u64));
        }
    }

    #[test]
    fn test_successful_magic_creation() {
        let rtable = {
            let mut data: [MaybeUninit<Magic>; 64] = unsafe {
                MaybeUninit::uninit().assume_init()
            };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(Magic{
                    mask: Bitboard(0), 
                    magic: Bitboard(0), 
                    attacks: Vec::new(), 
                    shift: 0,
                });
            }
            unsafe { transmute::<_, [Magic; 64]>(data)  }
        };
        let btable = {
            let mut data: [MaybeUninit<Magic>; 64] = unsafe {
                MaybeUninit::uninit().assume_init()
            };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(Magic{
                    mask: Bitboard(0), 
                    magic: Bitboard(0), 
                    attacks: Vec::new(), 
                    shift: 0,
                });
            }
            unsafe { transmute::<_, [Magic; 64]>(data)  }
        };
        let mut mtable = MagicTable{
            rook_magic: rtable,
            bishop_magic: btable
        };
        make_magic(&mut mtable);

    }
}