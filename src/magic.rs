use crate::bitboard::Bitboard;
use crate::direction::{Direction, BISHOP_DIRECTIONS, ROOK_DIRECTIONS};
use crate::square::Square;
use rand::thread_rng;
use rand::Rng;
use std::mem::{transmute, MaybeUninit};
use std::vec::Vec;

#[derive(Clone)]
pub struct MagicTable {
    pub rook_magic: [Magic; 64],
    pub bishop_magic: [Magic; 64],
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
    pub shift: u8,
}

//number of times to try generating magics
const NUM_MAGIC_TRIES: u64 = 1_000_000;

//diagonal going from A1 to H8
const MAIN_DIAG: Bitboard = Bitboard(0x8040201008040201);

//diagonal going from A8 to H1
const ANTI_DIAG: Bitboard = Bitboard(0x0102040810204080);

//1s on the outside ring of the board
const RING_MASK: Bitboard = Bitboard(0xFF818181818181FF);

//Magics which were computed using make_magic_helper
const SAVED_ROOK_MAGICS: [Bitboard; 64] = [
    Bitboard(0x4080002040001480), //a1
    Bitboard(  0x40001001402000), //b1
    Bitboard( 0x300200018104100), //c1
    Bitboard(0x2100040901100120), //d1
    Bitboard(0x8a00060004082070), //e1
    Bitboard(  0x80014400020080), //f1
    Bitboard(0x11002500208a0004), //g1
    Bitboard( 0x900004222018100), //h1
    Bitboard( 0x208800228c00081), //a2
    Bitboard(0x2280401003402000), //b2
    Bitboard(   0x8801000200184), //c2
    Bitboard(   0x1002010000900), //d2
    Bitboard( 0x182000600106008), //e2
    Bitboard(0x2058800400800200), //f2
    Bitboard(   0x4800200800900), //g2
    Bitboard( 0x52d00120040a100), //h2
    Bitboard( 0x5400880008024c1), //a3
    Bitboard(0x2000848040022000), //b3
    Bitboard( 0x400410011006000), //c3
    Bitboard(  0x40a10030010108), //d3
    Bitboard(0x1204808008000402), //e3
    Bitboard( 0x802808004002201), //f3
    Bitboard(0x1002808052000500), //g3
    Bitboard(   0x40a0021124184), //h3
    Bitboard( 0x640012880088040), //a4
    Bitboard(0x841040008020008a), //b4
    Bitboard( 0x400200880100080), //c4
    Bitboard(0x2001012100091004), //d4
    Bitboard(0x12000d0100080010), //e4
    Bitboard(0x6004000401201008), //f4
    Bitboard(0x7500aa0400084110), //g4
    Bitboard( 0x100005200040981), //h4
    Bitboard(  0x40804002800020), //a5
    Bitboard( 0x470002006400240), //b5
    Bitboard(   0x1200080801000), //c5
    Bitboard(     0x81202002040), //d5
    Bitboard(  0xc0804400800800), //e5
    Bitboard(0x9000800a00800400), //f5
    Bitboard(   0x1000401000600), //g5
    Bitboard(  0x421088ca002401), //h5
    Bitboard(    0xc000228d8000), //a6
    Bitboard(0x6410042014404001), //b6
    Bitboard(0x1002004082260014), //c6
    Bitboard(0x206a008811c20021), //d6
    Bitboard(   0x2001810220024), //e6
    Bitboard(0x2001020004008080), //f6
    Bitboard(0x10000801100c001a), //g6
    Bitboard(  0x48008254020011), //h6
    Bitboard( 0x400800940230100), //a7
    Bitboard(   0x8401100208100), //b7
    Bitboard(   0x1801004a00080), //c7
    Bitboard( 0x25068401200e200), //d7
    Bitboard(   0x2800401480080), //e7
    Bitboard(0x8104800200040080), //f7
    Bitboard( 0x108025085080400), //g7
    Bitboard(0x8400085104028200), //h7
    Bitboard(0x8085008000102941), //a8
    Bitboard(  0x11020080104022), //b8
    Bitboard(   0x1004020031811), //c8
    Bitboard(0x1009002030009569), //d8
    Bitboard( 0x40100900a441801), //e8
    Bitboard( 0x822002408104502), //f8
    Bitboard(0x80a1002200040085), //g8
    Bitboard(0x2000040221448102), //h8
];

const SAVED_BISHOP_MAGICS: [Bitboard; 64] = [
    Bitboard(0x8002043000860080), //a1
    Bitboard( 0x220680c8082800a), //b1
    Bitboard(0x2450248081000008), //c1
    Bitboard( 0x128a08604210013), //d1
    Bitboard(0xb23404a001010022), //e1
    Bitboard(  0x51901028020030), //f1
    Bitboard(  0x22060202400002), //g1
    Bitboard( 0x802806108204486), //h1
    Bitboard( 0x104108202140c00), //a2
    Bitboard(0x1008080808006040), //b2
    Bitboard(  0x28100080810422), //c2
    Bitboard(0x5120840c20816210), //d2
    Bitboard(  0x40040461004000), //e2
    Bitboard( 0x202008220202000), //f2
    Bitboard(    0x248808225204), //g2
    Bitboard(0x8044208401829040), //h2
    Bitboard( 0x2a0244008070110), //a3
    Bitboard(0x300800041000c200), //b3
    Bitboard(  0x90104904008830), //c3
    Bitboard(0x1288000082004008), //d3
    Bitboard(0x132c0212011c0002), //e3
    Bitboard(0x8000802102600204), //f3
    Bitboard(0x4013008201100200), //g3
    Bitboard(0x8502004022020202), //h3
    Bitboard(  0x22080020089006), //a4
    Bitboard(0x1404120004480802), //b4
    Bitboard( 0x429100006408200), //c4
    Bitboard(0x2001080004014210), //d4
    Bitboard(   0x1010040104008), //e4
    Bitboard( 0x400810402004a01), //f4
    Bitboard( 0x228030022050108), //g4
    Bitboard(    0x882006020200), //h4
    Bitboard(0x8090182020d40420), //a5
    Bitboard(0x4209282004020440), //b5
    Bitboard(0x2404020200130400), //c5
    Bitboard( 0x900020082880080), //d5
    Bitboard(0x1104080200912008), //e5
    Bitboard( 0x90101020001004a), //f5
    Bitboard(0x40090e2084860803), //g5
    Bitboard(0x40020620c0002401), //h5
    Bitboard(  0xe1108805006000), //a6
    Bitboard(0x2000880808008200), //b6
    Bitboard( 0x838420050000104), //c6
    Bitboard(0x9400002018000908), //d6
    Bitboard(  0x810801040110c2), //e6
    Bitboard( 0x4102000a2200102), //f6
    Bitboard(0x1234181244108041), //g6
    Bitboard(0x2004080200300046), //h6
    Bitboard(  0x81421210c20108), //a7
    Bitboard( 0x29022211008040c), //b7
    Bitboard(  0x10090409112080), //c7
    Bitboard(0x8006000e42062000), //d7
    Bitboard(  0x804240128208c0), //e7
    Bitboard(0x4010401005124a03), //f7
    Bitboard(0x30d0200501120800), //g7
    Bitboard(  0x3805a400820004), //h7
    Bitboard(    0x120310221014), //a8
    Bitboard( 0x224003404040452), //b8
    Bitboard(0xd008018200841140), //c8
    Bitboard( 0x310400a06209801), //d8
    Bitboard(    0x248060820480), //e8
    Bitboard( 0x400040a20241420), //f8
    Bitboard( 0x880410408420040), //g8
    Bitboard(0x8140500201404080), //h8
];
//target shift size for rook move enumeration. smaller is better
const ROOK_SHIFTS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, //rank 1
    11, 10, 10, 10, 10, 10, 10, 11, //2
    11, 10, 10, 10, 10, 10, 10, 11, //3
    11, 10, 10, 10, 10, 10, 10, 11, //4
    11, 10, 10, 10, 10, 10, 10, 11, //5
    11, 10, 10, 10, 10, 10, 10, 11, //6
    11, 10, 10, 10, 10, 10, 10, 11, //7
    12, 11, 11, 11, 11, 11, 11, 12, //8
];

//target shift size for bishop move enumeration. smaller is better
const BISHOP_SHIFTS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6, //rank 1
    5, 5, 5, 5, 5, 5, 5, 5, //2
    5, 5, 7, 7, 7, 7, 5, 5, //3
    5, 5, 7, 9, 9, 7, 5, 5, //4
    5, 5, 7, 9, 9, 7, 5, 5, //5
    5, 5, 7, 7, 7, 7, 5, 5, //6
    5, 5, 5, 5, 5, 5, 5, 5, //7
    6, 5, 5, 5, 5, 5, 5, 6, //8
];

#[allow(dead_code)]
pub fn create_empty_magic() -> MagicTable {
    let rtable = {
        let mut data: [MaybeUninit<Magic>; 64] = unsafe { MaybeUninit::uninit().assume_init() };
        for elem in &mut data[..] {
            *elem = MaybeUninit::new(Magic {
                mask: Bitboard(0),
                magic: Bitboard(0),
                attacks: Vec::new(),
                shift: 0,
            });
        }
        unsafe { transmute::<_, [Magic; 64]>(data) }
    };
    let btable = {
        let mut data: [MaybeUninit<Magic>; 64] = unsafe { MaybeUninit::uninit().assume_init() };
        for elem in &mut data[..] {
            *elem = MaybeUninit::new(Magic {
                mask: Bitboard(0),
                magic: Bitboard(0),
                attacks: Vec::new(),
                shift: 0,
            });
        }
        unsafe { transmute::<_, [Magic; 64]>(data) }
    };
    return MagicTable {
        rook_magic: rtable,
        bishop_magic: btable,
    };
}

#[allow(dead_code)]
pub fn make_magic(mtable: &mut MagicTable) {
    make_magic_helper(&mut mtable.rook_magic, true);
    make_magic_helper(&mut mtable.bishop_magic, false);
}

#[allow(dead_code)]
pub fn load_magic(mtable: &mut MagicTable) {
    load_magic_helper(&mut mtable.rook_magic, true);
    load_magic_helper(&mut mtable.bishop_magic, true);
}

fn load_magic_helper(table: &mut [Magic; 64], is_rook: bool) {
    for i in 0..64 {
        //square of the piece making attacks
        let sq = Square(i as u8);
        if is_rook {
            table[i].mask = get_rook_mask(sq);
            table[i].magic = SAVED_ROOK_MAGICS[i];
            table[i].shift = ROOK_SHIFTS[i];
        } else {
            table[i].mask = get_bishop_mask(sq);
            table[i].magic = SAVED_BISHOP_MAGICS[i];
            table[i].shift = BISHOP_SHIFTS[i];
        }
        table[i].attacks.resize(1 << table[i].shift, Bitboard(0));
        let num_points = table[i].mask.0.count_ones();
        for j in 0..(1 << num_points) {
            let occupancy = index_to_occupancy(j, table[i].mask);
            let attack = match is_rook {
                true => directional_attacks(sq, ROOK_DIRECTIONS, occupancy),
                false => directional_attacks(sq, BISHOP_DIRECTIONS, occupancy),
            };
            let key = compute_magic_key(occupancy, table[i].magic, table[i].shift);
            table[i].attacks[key] = attack;
        }
    }
}

fn get_attacks(occupancy: Bitboard, sq: Square, table: &[Magic; 64]) -> Bitboard {
    let idx = sq.0 as usize;
    let masked_occupancy = occupancy & table[idx].mask;
    let key = compute_magic_key(masked_occupancy, table[idx].magic, table[idx].shift);
    return table[idx].attacks[key];
}

#[allow(dead_code)]
pub fn get_rook_attacks(occupancy: Bitboard, sq: Square, mtable: &MagicTable) -> Bitboard {
    get_attacks(occupancy, sq, &mtable.rook_magic)
}

#[allow(dead_code)]
pub fn get_bishop_attacks(occupancy: Bitboard, sq: Square, mtable: &MagicTable) -> Bitboard {
    get_attacks(occupancy, sq, &mtable.bishop_magic)
}

#[inline]
fn compute_magic_key(occupancy: Bitboard, magic: Bitboard, shift: u8) -> usize {
    ((occupancy * magic).0 >> (64 - shift)) as usize
}

//Populate a magic table.
//is_rook is whether the table should be populated with rook moves (as opposed
//to bishop moves)
fn make_magic_helper(table: &mut [Magic; 64], is_rook: bool) {
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
        //table[i].shift = table[i].mask.0.count_ones() as u8;
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
        for _trial in 0..NUM_MAGIC_TRIES {
            let magic = random_sparse_bitboard();

            //repopulate the usage table with zeros
            used = [Bitboard(0); 1 << 12];
            found_magic = true;
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], magic, table[i].shift);
                if used[key] == Bitboard(0) {
                    used[key] = attacks[j];
                } else if used[key] != attacks[j] {
                    found_magic = false;
                    break;
                }
            }

            //found a working magic, we're done here
            if found_magic {
                /*println!(
                    "Found magic for square {}: {} in {} tries",
                    sq, magic, _trial
                );*/

                //use this print to generate a list of magics
                println!("\t{}, //{}", magic, sq);
                table[i].magic = magic;
                break;
            }
        }
        if !found_magic {
            println!(
                "FAILED to find magic on square {}. is rook? {}",
                sq, is_rook
            );
        } else {
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
    return (row_mask ^ col_mask) & !Bitboard::from(sq);
}

//Create the mask for the relevant bits in magic of a bishop
fn get_bishop_mask(sq: Square) -> Bitboard {
    //thank u chessprogramming wiki for this code
    let i = sq.0 as i32;
    let main_diag = 8 * (i & 7) - (i as i32 & 56);
    let main_lshift = -main_diag & (main_diag >> 31);
    let main_rshift = main_diag & (-main_diag >> 31);
    let main_diag_mask = (MAIN_DIAG >> main_rshift) << main_lshift;

    let anti_diag = 56 - 8 * (i & 7) - (i & 56);
    let anti_lshift = -anti_diag & (anti_diag >> 31);
    let anti_rshift = anti_diag & (-anti_diag >> 31);
    let anti_diag_mask = (ANTI_DIAG >> anti_rshift) << anti_lshift;

    return (main_diag_mask ^ anti_diag_mask) & !RING_MASK;
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
            if occupancy.is_square_occupied(current_square) {
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
    #[allow(unused_imports)]
    use super::*;

    #[allow(unused_imports)]
    use crate::square::*;

    #[test]
    fn test_rook_mask() {
        //println!("{:064b}", get_rook_mask(A1).0);
        assert_eq!(get_rook_mask(A1), Bitboard(0x000101010101017E));

        //println!("{:064b}", get_rook_mask(E1).0);
        assert_eq!(get_rook_mask(E1), Bitboard(0x001010101010106E));

        //println!("{:064b}", get_rook_mask(E5).0);
        assert_eq!(get_rook_mask(E5), Bitboard(0x0010106E10101000));
    }

    #[test]
    fn test_bishop_mask() {
        //println!("{:064b}", get_bishop_mask(A1).0);
        assert_eq!(get_bishop_mask(A1), Bitboard(0x0040201008040200));

        //println!("{:064b}", get_bishop_mask(E1).0);
        assert_eq!(get_bishop_mask(E1), Bitboard(0x0000000002442800));

        //println!("{:064b}", get_bishop_mask(E5).0);
        assert_eq!(get_bishop_mask(E5), Bitboard(0x0044280028440200));
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
    fn test_magic_creation() {
        let mut mtable = create_empty_magic();
        make_magic(&mut mtable);
    }

    #[test]
    fn test_rook_attacks() {
        let mut mtable = create_empty_magic();
        load_magic(&mut mtable);
        //cases in order:
        //rook on A1 blocked by other pieces, so it only attacks its neighbors
        //likewise, but there are other pieces on the board to be masked out
        let occupancies = [Bitboard(0x103), Bitboard(0x1FC3)];
        let squares = [A1, A1];
        let attacks = [Bitboard(0x102), Bitboard(0x102)];
        for i in 0..1 {
            let resulting_attack = get_rook_attacks(occupancies[i], squares[i], &mtable);
            assert_eq!(attacks[i], resulting_attack);
        }
    }
}
