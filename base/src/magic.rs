use super::{Bitboard, Direction, Square};

use rand::{thread_rng, Rng};

use std::{
    convert::TryFrom,
    mem::{transmute, MaybeUninit},
};

/// The number of times to try generating magics.
const NUM_MAGIC_TRIES: u64 = 10_000_000;

/// The diagonal going from A1 to H8.
const MAIN_DIAG: Bitboard = Bitboard::new(0x8040201008040201);

/// The diagonal going from A8 to H1.
const ANTI_DIAG: Bitboard = Bitboard::new(0x0102040810204080);

/// A Bitboard made of 1's around the ring of the board, and 0's in the middle
const RING_MASK: Bitboard = Bitboard::new(0xFF818181818181FF);

/// A saved list of magics for rooks created using the generator. Some magics
/// for sizes below the required bitshift amount were taken from the
/// Chessprogramming Wiki.
const SAVED_ROOK_MAGICS: [Bitboard; 64] = [
    Bitboard::new(0x4080002040001480), //a1
    Bitboard::new(0x40001001402000),   //b1
    Bitboard::new(0x300200018104100),  //c1
    Bitboard::new(0x2100040901100120), //d1
    Bitboard::new(0x8a00060004082070), //e1
    Bitboard::new(0x80014400020080),   //f1
    Bitboard::new(0x11002500208a0004), //g1
    Bitboard::new(0x900004222018100),  //h1
    Bitboard::new(0x208800228c00081),  //a2
    Bitboard::new(0x2280401003402000), //b2
    Bitboard::new(0x8801000200184),    //c2
    Bitboard::new(0x1002010000900),    //d2
    Bitboard::new(0x182000600106008),  //e2
    Bitboard::new(0x2058800400800200), //f2
    Bitboard::new(0x4800200800900),    //g2
    Bitboard::new(0x52d00120040a100),  //h2
    Bitboard::new(0x5400880008024c1),  //a3
    Bitboard::new(0x2000848040022000), //b3
    Bitboard::new(0x400410011006000),  //c3
    Bitboard::new(0x40a10030010108),   //d3
    Bitboard::new(0x1204808008000402), //e3
    Bitboard::new(0x802808004002201),  //f3
    Bitboard::new(0x1002808052000500), //g3
    Bitboard::new(0x40a0021124184),    //h3
    Bitboard::new(0x640012880088040),  //a4
    Bitboard::new(0x841040008020008a), //b4
    Bitboard::new(0x400200880100080),  //c4
    Bitboard::new(0x2001012100091004), //d4
    Bitboard::new(0x12000d0100080010), //e4
    Bitboard::new(0x6004000401201008), //f4
    Bitboard::new(0x7500aa0400084110), //g4
    Bitboard::new(0x100005200040981),  //h4
    Bitboard::new(0x40804002800020),   //a5
    Bitboard::new(0x470002006400240),  //b5
    Bitboard::new(0x1200080801000),    //c5
    Bitboard::new(0x81202002040),      //d5
    Bitboard::new(0xc0804400800800),   //e5
    Bitboard::new(0x9000800a00800400), //f5
    Bitboard::new(0x1000401000600),    //g5
    Bitboard::new(0x421088ca002401),   //h5
    Bitboard::new(0xc000228d8000),     //a6
    Bitboard::new(0x6410042014404001), //b6
    Bitboard::new(0x1002004082260014), //c6
    Bitboard::new(0x206a008811c20021), //d6
    Bitboard::new(0x2001810220024),    //e6
    Bitboard::new(0x2001020004008080), //f6
    Bitboard::new(0x10000801100c001a), //g6
    Bitboard::new(0x48008254020011),   //h6
    Bitboard::new(0x48FFFE99FECFAA00), //a7
    Bitboard::new(0x48FFFE99FECFAA00), //b7
    Bitboard::new(0x497FFFADFF9C2E00), //c7
    Bitboard::new(0x613FFFDDFFCE9200), //d7
    Bitboard::new(0xffffffe9ffe7ce00), //e7
    Bitboard::new(0xfffffff5fff3e600), //f7
    Bitboard::new(0x0003ff95e5e6a4c0), //g7
    Bitboard::new(0x510FFFF5F63C96A0), //h7
    Bitboard::new(0xEBFFFFB9FF9FC526), //a8
    Bitboard::new(0x61FFFEDDFEEDAEAE), //b8
    Bitboard::new(0x53BFFFEDFFDEB1A2), //c8
    Bitboard::new(0x127FFFB9FFDFB5F6), //d8
    Bitboard::new(0x411FFFDDFFDBF4D6), //e8
    Bitboard::new(0x822002408104502),  //f8
    Bitboard::new(0x0003ffef27eebe74), //g8
    Bitboard::new(0x7645FFFECBFEA79E), //h8
];

/// A saved list of magics for bishops created using the generator. Some magics
/// for sizes below the required bitshift amount were taken from the
/// Chessprogramming Wiki.
const SAVED_BISHOP_MAGICS: [Bitboard; 64] = [
    Bitboard::new(0xffedf9fd7cfcffff), //a1
    Bitboard::new(0xfc0962854a77f576), //b1
    Bitboard::new(0x122808c102a004),   //c1
    Bitboard::new(0x2851240082400440), //d1
    Bitboard::new(0x11104011000202),   //e1
    Bitboard::new(0x8220820000010),    //f1
    Bitboard::new(0xfc0a66c64a7ef576), //g1
    Bitboard::new(0x7ffdfdfcbd79ffff), //h1
    Bitboard::new(0xfc0846a64a34fff6), //a2
    Bitboard::new(0xfc087a874a3cf7f6), //b2
    Bitboard::new(0x988020420a000),    //c2
    Bitboard::new(0x8000440400808200), //d2
    Bitboard::new(0x208c8450c0013407), //e2
    Bitboard::new(0x1980110520108030), //f2
    Bitboard::new(0xfc0864ae59b4ff76), //g2
    Bitboard::new(0x3c0860af4b35ff76), //h2
    Bitboard::new(0x73C01AF56CF4CFFB), //a3
    Bitboard::new(0x41A01CFAD64AAFFC), //b3
    Bitboard::new(0x604000204a20202),  //c3
    Bitboard::new(0x2820806024000),    //d3
    Bitboard::new(0x8a002422010201),   //e3
    Bitboard::new(0x2082004088010802), //f3
    Bitboard::new(0x7c0c028f5b34ff76), //g3
    Bitboard::new(0xfc0a028e5ab4df76), //h3
    Bitboard::new(0x8100420d1041080),  //a4
    Bitboard::new(0x904510002100100),  //b4
    Bitboard::new(0x202280804064403),  //c4
    Bitboard::new(0x4c00400c030082),   //d4
    Bitboard::new(0x602001002005011),  //e4
    Bitboard::new(0x72090200c1089000), //f4
    Bitboard::new(0x4211410424008805), //g4
    Bitboard::new(0x2848421260804),    //h4
    Bitboard::new(0xc001041211212004), //a5
    Bitboard::new(0x208018800044800),  //b5
    Bitboard::new(0x80206410580800),   //c5
    Bitboard::new(0x201100080084),     //d5
    Bitboard::new(0x208003400094100),  //e5
    Bitboard::new(0x2190410200004058), //f5
    Bitboard::new(0x188821401808080),  //g5
    Bitboard::new(0x20060a020000c4c0), //h5
    Bitboard::new(0xDCEFD9B54BFCC09F), //a6
    Bitboard::new(0xF95FFA765AFD602B), //b6
    Bitboard::new(0x200a104110002040), //c6
    Bitboard::new(0x800000c08310c00),  //d6
    Bitboard::new(0x21804010a010400),  //e6
    Bitboard::new(0x1092200400224100), //f6
    Bitboard::new(0x43ff9a5cf4ca0c01), //g6
    Bitboard::new(0x4BFFCD8E7C587601), //h6
    Bitboard::new(0xfc0ff2865334f576), //a7
    Bitboard::new(0xfc0bf6ce5924f576), //b7
    Bitboard::new(0x805220608c300001), //c7
    Bitboard::new(0x2084105042020400), //d7
    Bitboard::new(0xe018801022060220), //e7
    Bitboard::new(0x1122049010200),    //f7
    Bitboard::new(0xc3ffb7dc36ca8c89), //g7
    Bitboard::new(0xc3ff8a54f4ca2c89), //h7
    Bitboard::new(0xfffffcfcfd79edff), //a8
    Bitboard::new(0xfc0863fccb147576), //b8
    Bitboard::new(0x40a0040062133000), //c8
    Bitboard::new(0x142028000840400),  //d8
    Bitboard::new(0x9090010061200),    //e8
    Bitboard::new(0x800844528100308),  //f8
    Bitboard::new(0xfc087e8e4bb2f736), //g8
    Bitboard::new(0x43ff9e4ef4ca2c89), //h8
];

/// The number of bits used to express the magic lookups for rooks at each
/// square.
const ROOK_BITS: [u8; 64] = [
    12, 11, 11, 11, 11, 11, 11, 12, // rank 1
    11, 10, 10, 10, 10, 10, 10, 11, // 2
    11, 10, 10, 10, 10, 10, 10, 11, // 3
    11, 10, 10, 10, 10, 10, 10, 11, // 4
    11, 10, 10, 10, 10, 10, 10, 11, // 5
    11, 10, 10, 10, 10, 10, 10, 11, // 6
    10, 9, 9, 9, 9, 9, 9, 10, // 7
    11, 10, 10, 10, 10, 11, 10, 11, // 8
];

/// The number of bits used to express the magic lookups for bishops at each
/// square.
const BISHOP_BITS: [u8; 64] = [
    5, 4, 5, 5, 5, 5, 4, 5, // rank 1
    4, 4, 5, 5, 5, 5, 4, 4, // 2
    4, 4, 7, 7, 7, 7, 4, 4, // 3
    5, 5, 7, 9, 9, 7, 5, 5, // 4
    5, 5, 7, 9, 9, 7, 5, 5, // 5
    4, 4, 7, 7, 7, 7, 4, 4, // 6
    4, 4, 5, 5, 5, 5, 4, 4, // 7
    5, 4, 5, 5, 5, 5, 4, 5, // 8
];

#[derive(Clone, Debug)]
/// A complete magic table which can generate moves for rooks and bishops.
pub struct MagicTable {
    rook_magic: [Magic; 64],
    bishop_magic: [Magic; 64],
}

impl MagicTable {
    /// Create an empty `MagicTable`.
    fn new() -> MagicTable {
        let rtable = {
            let mut data: [MaybeUninit<Magic>; 64] = unsafe { MaybeUninit::uninit().assume_init() };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(Magic::new());
            }
            unsafe { transmute(data) }
        };
        let btable = {
            let mut data: [MaybeUninit<Magic>; 64] = unsafe { MaybeUninit::uninit().assume_init() };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(Magic::new());
            }
            unsafe { transmute(data) }
        };
        MagicTable {
            rook_magic: rtable,
            bishop_magic: btable,
        }
    }

    /// Create a pre-loaded `MagicTable`.
    pub fn load() -> MagicTable {
        let mut mtable = MagicTable::new();
        load_magic_helper(&mut mtable.rook_magic, true);
        load_magic_helper(&mut mtable.bishop_magic, false);

        mtable
    }

    #[allow(unused)]
    /// Create a `MagicTable` from scratch, generating new magics.
    pub fn make() -> MagicTable {
        let mut mtable = MagicTable::new();
        make_magic_helper(&mut mtable.rook_magic, true);
        make_magic_helper(&mut mtable.bishop_magic, false);

        mtable
    }

    #[inline(always)]
    /// Get the attacks that a rook on `sq` could make with the reference table
    /// `mtable`.
    pub fn rook_attacks(&self, occupancy: Bitboard, sq: Square) -> Bitboard {
        get_attacks(occupancy, sq, &self.rook_magic)
    }

    #[inline(always)]
    /// Get the attacks that a bishop on `sq` could make with the reference
    /// table `mtable`.
    pub fn bishop_attacks(&self, occupancy: Bitboard, sq: Square) -> Bitboard {
        get_attacks(occupancy, sq, &self.bishop_magic)
    }
}

/// A structure containing all the information needed to generate moves for a
/// rook or bishop.
#[derive(Clone, Debug)]
struct Magic {
    /// A mask which, when &ed with the occupancy bitboard, will give only the
    /// bits that matter when computing moves.
    pub mask: Bitboard,
    /// The magic number to multiply to hash the current board effectively.
    pub magic: Bitboard,
    /// A lookup vector of squares attacked.
    pub attacks: Vec<Bitboard>,
    /// The shift related to this square.
    pub shift: u8,
}

impl Magic {
    /// Create an empty `Magic`.
    fn new() -> Magic {
        Magic {
            mask: Bitboard::EMPTY,
            magic: Bitboard::EMPTY,
            attacks: Vec::new(),
            shift: 0,
        }
    }
}

/// A helper function to load data into a `MagicTable`. `is_rook` is `true` if
/// you are loading data for a rook, and `false` for a bishop.
fn load_magic_helper(table: &mut [Magic; 64], is_rook: bool) {
    for i in 0..64 {
        // square of the piece making attacks
        let sq = Square::try_from(i as u8).unwrap();
        if is_rook {
            table[i].mask = get_rook_mask(sq);
            table[i].magic = SAVED_ROOK_MAGICS[i];
            table[i].shift = 64 - ROOK_BITS[i];
        } else {
            table[i].mask = get_bishop_mask(sq);
            table[i].magic = SAVED_BISHOP_MAGICS[i];
            table[i].shift = 64 - BISHOP_BITS[i];
        }
        table[i]
            .attacks
            .resize(1 << (64 - table[i].shift), Bitboard::EMPTY);
        let num_points = table[i].mask.count_ones();
        for j in 0..(1 << num_points) {
            let occupancy = index_to_occupancy(j, table[i].mask);
            let directions = if is_rook {
                &Direction::ROOK_DIRECTIONS
            } else {
                &Direction::BISHOP_DIRECTIONS
            };
            let attack = directional_attacks(sq, directions, occupancy);
            let key = compute_magic_key(occupancy, table[i].magic, table[i].shift);
            if table[i].attacks[key].is_empty() {
                table[i].attacks[key] = attack;
            } else if table[i].attacks[key] != attack {
                // This should never happen, since we should expect our loads to
                // always succeed. Panic immediately since this is a critical
                // error.
                println!(
                    "failed to load {} magics for square {sq}",
                    if is_rook { "rook" } else { "bishop" }
                );
                panic!("Hash collision occurred on magic generation");
            }
        }
    }
}

/// Get the attacks a square has, given a magic lookup table and the current
/// occupancy.
fn get_attacks(occupancy: Bitboard, sq: Square, table: &[Magic; 64]) -> Bitboard {
    // In defense of the unsafe blocks below: `sq` is a valid square, so
    // accessing it by array lookup is OK. Additionally, we can trust that the
    // key was masked correctly in `compute_magic_key` as it was shifted out
    // properly. The speed benefit is extremely important here, as getting
    // magic attacks will be called many tens of millions of times per second.
    let magic_data = unsafe { table.get_unchecked(sq as usize) };
    let masked_occupancy = occupancy & magic_data.mask;
    let key = compute_magic_key(masked_occupancy, magic_data.magic, magic_data.shift);

    unsafe { *magic_data.attacks.get_unchecked(key) }
}

#[inline(always)]
/// Use magic hashing to get the index to look up attacks in a bitboad.
fn compute_magic_key(occupancy: Bitboard, magic: Bitboard, shift: u8) -> usize {
    usize::from((occupancy * magic) >> shift)
}

/// Populate a magic table. If `is_rook` is true, it will make magics for rook
/// moves; otherwise it will make magics for bishops.
///
/// # Panics
///
/// Will panic if this helper function is unable to compute each magic value in
/// the specified number of tries.
fn make_magic_helper(table: &mut [Magic; 64], is_rook: bool) {
    for i in 0..64 {
        // square of the piece making attacks
        let sq = Square::try_from(i as u8).unwrap();
        if is_rook {
            table[i].mask = get_rook_mask(sq);
            table[i].shift = 64 - ROOK_BITS[i];
        } else {
            table[i].mask = get_bishop_mask(sq);
            table[i].shift = 64 - BISHOP_BITS[i];
        }
        // number of squares where occupancy matters
        let num_points = table[i].mask.count_ones();

        // we know that there are at most 12 pieces that will matter when it
        // comes to attack lookups
        let mut occupancies = [Bitboard::EMPTY; 1 << 12];
        let mut attacks = [Bitboard::EMPTY; 1 << 12];

        // compute every possible occupancy arrangement for attacking
        for j in 0..(1 << num_points) {
            occupancies[j] = index_to_occupancy(j, table[i].mask);
            //compute attacks
            attacks[j] = if is_rook {
                directional_attacks(sq, &Direction::ROOK_DIRECTIONS, occupancies[j])
            } else {
                directional_attacks(sq, &Direction::BISHOP_DIRECTIONS, occupancies[j])
            };
        }
        // try random magics until one works
        let mut found_magic = false;
        let mut used;
        for _ in 0..NUM_MAGIC_TRIES {
            let magic = random_sparse_bitboard();

            // repopulate the usage table with zeros
            used = [Bitboard::EMPTY; 1 << 12];
            found_magic = true;
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], magic, table[i].shift);
                if used[key].is_empty() {
                    used[key] = attacks[j];
                } else if used[key] != attacks[j] {
                    found_magic = false;
                    break;
                }
            }

            // found a working magic, we're done here
            if found_magic {
                //use this print to generate a list of magics
                println!("\t{magic}, //{sq}");
                table[i].magic = magic;
                break;
            }
        }
        if !found_magic {
            println!(
                "failed to find {} magic for square {sq}",
                if is_rook { "rook" } else { "bishop" }
            );
            panic!();
        } else {
            // found a magic, populate the attack vector
            table[i]
                .attacks
                .resize(1 << (64 - table[i].shift), Bitboard::EMPTY);
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], table[i].magic, table[i].shift);
                table[i].attacks[key] = attacks[j];
            }
        }
    }
}

/// Create the mask for the relevant bits in magic of a rook. `sq` is the
/// square that a rook would occupy to receive this mask.
fn get_rook_mask(sq: Square) -> Bitboard {
    let index = sq as i8;
    // sequence of 1s down the same row as the piece to move, except on the
    // ends
    let row_mask = Bitboard::new(0x7E << (8 * (index / 8)));
    // sequence of 1s down the same col as the piece to move, except on the
    // ends
    let col_mask = Bitboard::new(0x0001010101010100 << (index % 8));
    // note: pieces at the end of the travel don't matter, which is why the
    // masks arent' uniform

    // in the col mask or row mask, but not the piece to move
    // xor operation will remove the square the piece is on
    (row_mask ^ col_mask) & !Bitboard::from(sq)
}

/// Create the mask for the relevant bits in magic of a bishop. `sq` is the
/// square that a bishop would be on to receiver this mask.
fn get_bishop_mask(sq: Square) -> Bitboard {
    // thank u chessprogramming wiki for this code
    let i = sq as i32;
    let main_diag = 8 * (i & 7) - (i as i32 & 56);
    let main_lshift = -main_diag & (main_diag >> 31);
    let main_rshift = main_diag & (-main_diag >> 31);
    let main_diag_mask = (MAIN_DIAG >> main_rshift) << main_lshift;

    let anti_diag = 56 - 8 * (i & 7) - (i & 56);
    let anti_lshift = -anti_diag & (anti_diag >> 31);
    let anti_rshift = anti_diag & (-anti_diag >> 31);
    let anti_diag_mask = (ANTI_DIAG >> anti_rshift) << anti_lshift;

    (main_diag_mask ^ anti_diag_mask) & !RING_MASK
}

/// Given some mask, create the occupancy bitboard according to this index.
/// `index` must be less than or equal to 2 ^ (number of ones in `mask`).
///
/// For instance: if `mask` repreresented a board like the following:
/// ```ignore
/// 8 | . . . . . . . .
/// 7 | . . . . . . . .
/// 6 | . . . . . . . .
/// 5 | . . . . . . . .
/// 4 | . . . . . . . .
/// 3 | . . . . . . . .
/// 2 | . 1 . . . . . .
/// 1 | 1 . . . . . . .
/// - + - - - - - - - -
/// . | A B C D E F G H
/// ```
///
/// and the given index were `0b10`, then the output mask would be
///
/// ```ignore
/// 8 | . . . . . . . .
/// 7 | . . . . . . . .
/// 6 | . . . . . . . .
/// 5 | . . . . . . . .
/// 4 | . . . . . . . .
/// 3 | . . . . . . . .
/// 2 | . 1 . . . . . .
/// 1 | . . . . . . . .
/// - + - - - - - - - -
/// . | A B C D E F G H
/// ```
fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut result = Bitboard::EMPTY;
    let num_points = mask.count_ones();
    let mut editable_mask = mask;
    // go from right to left in the bits of num_points,
    // and add an occupancy if something is there
    for i in 0..num_points {
        let shift_size = editable_mask.trailing_zeros();
        //make a bitboard which only occupies the rightmost square
        let occupier = Bitboard::new(1 << shift_size);
        //remove the occupier from the mask
        editable_mask &= !occupier;
        if (index & (1 << i)) != 0 {
            //the bit corresponding to the occupier is nonzero
            result |= occupier;
        }
    }

    result
}

/// Construct the squares attacked by the pieces at `sq` if it could move along
/// the directions in `dirs`, when the board is occupied by the pieces in
/// `occupancy`. This is slow and should only be used for generatic magic
/// bitboards (instead of for move generation.)
fn directional_attacks(sq: Square, dirs: &[Direction], occupancy: Bitboard) -> Bitboard {
    let mut result = Bitboard::EMPTY;
    for dir in dirs.iter() {
        let mut current_square = sq;
        for _ in 0..7 {
            if !is_valid_step(current_square, *dir) {
                break;
            }
            current_square += *dir;
            result |= Bitboard::from(current_square);
            if occupancy.contains(current_square) {
                break;
            }
        }
    }

    result
}

/// Return whether the following move is a single-step.
fn is_valid_step(sq: Square, dir: Direction) -> bool {
    sq.chebyshev_to(sq + dir) <= 1
}

#[inline(always)]
/// Generate a random, mostly-empty bitboard.
fn random_sparse_bitboard() -> Bitboard {
    let mut result = Bitboard::new(thread_rng().gen::<u64>());
    for _ in 0..2 {
        result &= Bitboard::new(thread_rng().gen::<u64>());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Square;

    #[test]
    fn test_rook_mask() {
        //println!("{:064b}", get_rook_mask(A1).0);
        assert_eq!(get_rook_mask(Square::A1), Bitboard::new(0x000101010101017E));

        //println!("{:064b}", get_rook_mask(E1).0);
        assert_eq!(get_rook_mask(Square::E1), Bitboard::new(0x001010101010106E));

        //println!("{:064b}", get_rook_mask(E5).0);
        assert_eq!(get_rook_mask(Square::E5), Bitboard::new(0x0010106E10101000));
    }

    #[test]
    fn test_bishop_mask() {
        //println!("{:064b}", get_bishop_mask(A1).0);
        assert_eq!(
            get_bishop_mask(Square::A1),
            Bitboard::new(0x0040201008040200)
        );

        //println!("{:064b}", get_bishop_mask(E1).0);
        assert_eq!(
            get_bishop_mask(Square::E1),
            Bitboard::new(0x0000000002442800)
        );

        //println!("{:064b}", get_bishop_mask(E5).0);
        assert_eq!(
            get_bishop_mask(Square::E5),
            Bitboard::new(0x0044280028440200)
        );
    }

    #[test]
    fn test_index_to_occupancy() {
        let mask = Bitboard::new(0b1111);
        for i in 0..16 {
            let occu = index_to_occupancy(i, mask);
            assert_eq!(occu, Bitboard::new(i as u64));
        }
    }

    // This test is commented out because the shifts
    // currently used are smaller than are practical to
    // search for.
    /*
    #[test]
    fn test_magic_creation() {
        MagicTable::make();
    }
    */

    #[test]
    fn test_magic_rook_attacks() {
        let mtable = MagicTable::load();
        //cases in order:
        //rook on A1 blocked by other pieces, so it only attacks its neighbors
        //likewise, but there are other pieces on the board to be masked out
        let occupancies = [Bitboard::new(0x103), Bitboard::new(0x1FC3)];
        let squares = [Square::A1, Square::A1];
        let attacks = [Bitboard::new(0x102), Bitboard::new(0x102)];
        for i in 0..1 {
            let resulting_attack = mtable.rook_attacks(occupancies[i], squares[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }

    #[test]
    fn test_magic_bishop_attacks() {
        //cases in order:
        //bishop on A1 is blocked by piece on B2, so it only has 1 attack
        //bishop on A8 is blocked by piece on B7, so it only has 1 attack
        //bishop is in board start position on C1
        //bishop in board start position on F1
        let occupancies = [
            Bitboard::new(0x0000000000000201), //
            Bitboard::new(0x0102000000000000), //
            Bitboard::new(0xFFFF00000000FFFF), //
            Bitboard::new(0xFFFF00000000FFFF), //
        ];
        let squares = [
            Square::A1, //
            Square::A8, //
            Square::C1, //
            Square::F1, //
        ];
        let attacks = [
            Bitboard::new(0x0000000000000200), //
            Bitboard::new(0x0002000000000000), //
            Bitboard::new(0x0000000000000A00), //
            Bitboard::new(0x0000000000005000), //
        ];
        for i in 0..3 {
            let resulting_attack =
                directional_attacks(squares[i], &Direction::BISHOP_DIRECTIONS, occupancies[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }

    #[test]
    fn test_bishop_attacks() {
        let mtable = MagicTable::load();
        //cases in order:
        //bishop on A1 is blocked by piece on B2, so it only has 1 attack
        //bishop on A8 is blocked by piece on B7, so it only has 1 attack
        //bishop is in board start position on C1
        //bishop in board start position on F1
        let occupancies = [
            Bitboard::new(0x0000000000000201), //
            Bitboard::new(0x0102000000000000), //
            Bitboard::new(0xFFFF00000000FFFF), //
            Bitboard::new(0xFFFF00000000FFFF), //
        ];
        let squares = [
            Square::A1, //
            Square::A8, //
            Square::C1, //
            Square::F1, //
        ];
        let attacks = [
            Bitboard::new(0x0000000000000200), //
            Bitboard::new(0x0002000000000000), //
            Bitboard::new(0x0000000000000A00), //
            Bitboard::new(0x0000000000005000), //
        ];
        for i in 0..3 {
            let resulting_attack = mtable.bishop_attacks(occupancies[i], squares[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }
}
