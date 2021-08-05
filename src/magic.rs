use crate::bitboard::Bitboard;
use crate::direction::{Direction, BISHOP_DIRECTIONS, ROOK_DIRECTIONS};
use crate::square::Square;
use rand::thread_rng;
use rand::Rng;
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
    Bitboard(0xc80008020400010),
    Bitboard(0x240084010002000),
    Bitboard(0x100104100200089),
    Bitboard(0x4080100008008006),
    Bitboard(0x9200200200100904),
    Bitboard(0x1000c0018a22100),
    Bitboard(0x880088025001200),
    Bitboard(0x300002900008242),
    Bitboard(0xc2801280400060),
    Bitboard(0x3002004022008100),
    Bitboard(0x880801000a000),
    Bitboard(0x8000800800801004),
    Bitboard(0x1000508001100),
    Bitboard(0xd802000a00100408),
    Bitboard(0x2000104820008),
    Bitboard(0x8024800060800100),
    Bitboard(0x4e0800080c000),
    Bitboard(0x8050004020004000),
    Bitboard(0x6040c10010200102),
    Bitboard(0x9028018010000880),
    Bitboard(0x9800808014002801),
    Bitboard(0x280806c000200),
    Bitboard(0x10040010080182),
    Bitboard(0x2a0010408401),
    Bitboard(0x40014080112081),
    Bitboard(0x10c0400180200080),
    Bitboard(0x10a004200241080),
    Bitboard(0x88100400400800c0),
    Bitboard(0x20140080080080),
    Bitboard(0x2802000200100408),
    Bitboard(0x84100402080200),
    Bitboard(0x1050004200048405),
    Bitboard(0x440008002802040),
    Bitboard(0x800402000401002),
    Bitboard(0xe802000801008),
    Bitboard(0x8032001222000840),
    Bitboard(0x3000800c00800802),
    Bitboard(0x8220080800400),
    Bitboard(0x3400100324000248),
    Bitboard(0x800041801100),
    Bitboard(0xac03400481208000),
    Bitboard(0x8c0042810002000),
    Bitboard(0xc28200120020),
    Bitboard(0x5003000b0020),
    Bitboard(0x841011088010014),
    Bitboard(0x400410080120),
    Bitboard(0x104211040008),
    Bitboard(0xa280110648aa0004),
    Bitboard(0x40008001a180),
    Bitboard(0x861001a008400a40),
    Bitboard(0x800900220008880),
    Bitboard(0x2000100180080080),
    Bitboard(0x4080084008080),
    Bitboard(0xc000201004040),
    Bitboard(0x1080502100400),
    Bitboard(0x10480010001e380),
    Bitboard(0x4300c080081161),
    Bitboard(0x4810040002291),
    Bitboard(0x200041021019),
    Bitboard(0x1a00241040201a),
    Bitboard(0xc05000210080005),
    Bitboard(0x101009804004201),
    Bitboard(0x4048820508900814),
    Bitboard(0x400250442840e),
];

const SAVED_BISHOP_MAGICS: [Bitboard; 64] = [
    Bitboard(0x208420808430204),
    Bitboard(0x84a1022410448080),
    Bitboard(0x24080202400015),
    Bitboard(0x2004040092000800),
    Bitboard(0x501a021000000008),
    Bitboard(0x2001042095000300),
    Bitboard(0x804d051006200201),
    Bitboard(0x3280804420844002),
    Bitboard(0x444a98808208408),
    Bitboard(0x402a11a00810102),
    Bitboard(0x40800b1021130),
    Bitboard(0x24020c0400920048),
    Bitboard(0x9000540420000020),
    Bitboard(0x8002609010484022),
    Bitboard(0x80005a02104111),
    Bitboard(0x208220211012800),
    Bitboard(0x22020c4a4641800),
    Bitboard(0xcc4006055220202),
    Bitboard(0x48001001404008),
    Bitboard(0x108002220234008),
    Bitboard(0x244000081a02801),
    Bitboard(0x120001004a0e00),
    Bitboard(0x122000125016000),
    Bitboard(0x40061000c2060100),
    Bitboard(0x4b20a06208081120),
    Bitboard(0x8002090150100a80),
    Bitboard(0x4100881004080),
    Bitboard(0x804200200a008200),
    Bitboard(0x109001001004020),
    Bitboard(0x445345000200aa00),
    Bitboard(0x2801014801280820),
    Bitboard(0x6028002004100),
    Bitboard(0x8084024200600c01),
    Bitboard(0x202402122138b000),
    Bitboard(0x100100c800090800),
    Bitboard(0x1208202021880080),
    Bitboard(0x240010900020042),
    Bitboard(0x810020080003000),
    Bitboard(0x410808200831309),
    Bitboard(0x1061012101820042),
    Bitboard(0x2201011040005210),
    Bitboard(0x200148080848c400),
    Bitboard(0x1402a20030084200),
    Bitboard(0x800044012005046),
    Bitboard(0x4000032012000900),
    Bitboard(0x4040080801400060),
    Bitboard(0x18404040412a040),
    Bitboard(0x4080281188a24),
    Bitboard(0xa020220840000),
    Bitboard(0x8180841402028000),
    Bitboard(0x2196100980c0040),
    Bitboard(0x4540081084808c1),
    Bitboard(0x88200088a1010000),
    Bitboard(0x1000092008022100),
    Bitboard(0x1050842808314a00),
    Bitboard(0x50c086810448208),
    Bitboard(0x4ac40200822000),
    Bitboard(0x2c02104101442),
    Bitboard(0x804000a64222800),
    Bitboard(0x1300110000840420),
    Bitboard(0x1034009102408),
    Bitboard(0x10402811a0490500),
    Bitboard(0x48500590308200),
    Bitboard(0x2200102009100),
];
//target shift size for rook move enumeration. smaller is better
const ROOK_SHIFTS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11, 11, 10, 10, 10, 10, 10, 10, 11,
    11, 10, 10, 10, 10, 10, 10, 11, 12, 11, 11, 11, 11, 11, 11, 12,
];

//target shift size for bishop move enumeration. smaller is better
const BISHOP_SHIFTS: [u8; 64] = [
    6, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 7, 9, 9, 7, 5, 5,
    5, 5, 7, 9, 9, 7, 5, 5, 5, 5, 7, 7, 7, 7, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 5, 5, 5, 5, 5, 5, 6,
];

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
        } else {
            table[i].mask = get_bishop_mask(sq);
        }
        table[i].shift = table[i].mask.0.count_ones() as u8;
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
        for trial in 0..NUM_MAGIC_TRIES {
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
                println!(
                    "Found magic for square {}: {} in {} tries",
                    sq, magic, trial
                );

                //use this print to generate a list of magics
                //println!("\t{},", magic);
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
    #[allow(dead_code)]
    use super::*;

    #[allow(dead_code)]
    use crate::square::*;

    use std::mem::{transmute, MaybeUninit};

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
        let mut mtable = MagicTable {
            rook_magic: rtable,
            bishop_magic: btable,
        };
        make_magic(&mut mtable);
    }

    #[test]
    fn test_rook_attacks() {
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
        let mut mtable = MagicTable {
            rook_magic: rtable,
            bishop_magic: btable,
        };
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
