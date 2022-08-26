/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 The Fiddler Authors (see AUTHORS.md file)

  Fiddler is free software: you can redistribute it and/or modify
  it under the terms of the GNU General Public License as published by
  the Free Software Foundation, either version 3 of the License, or
  (at your option) any later version.

  Fiddler is distributed in the hope that it will be useful,
  but WITHOUT ANY WARRANTY; without even the implied warranty of
  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
  GNU General Public License for more details.

  You should have received a copy of the GNU General Public License
  along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/

//! Magic bitboards, used for generating bishop, knight, and rook moves.

use super::{Bitboard, Direction, Square};

use once_cell::sync::Lazy;

use std::{
    convert::TryFrom,
    mem::{transmute, MaybeUninit},
};

/// A master copy of the main magic table. Used for generating bishop,
/// rook, and queen moves.
pub static MAGIC: Lazy<AttacksTable> = Lazy::new(AttacksTable::load);

/// The number of times to try generating magics.
const NUM_MAGIC_TRIES: u64 = 10_000_000;

/// A saved list of magics for rooks created using the generator. Some magics
/// for sizes below the required bitshift amount were taken from the
/// Chessprogramming Wiki.
const SAVED_ROOK_MAGICS: [Bitboard; 64] = [
    Bitboard::new(0x4080_0020_4000_1480), // a1
    Bitboard::new(0x0040_0010_0140_2000), // b1
    Bitboard::new(0x0300_2000_1810_4100), // c1
    Bitboard::new(0x2100_0409_0110_0120), // d1
    Bitboard::new(0x8a00_0600_0408_2070), // e1
    Bitboard::new(0x0080_0144_0002_0080), // f1
    Bitboard::new(0x1100_2500_208a_0004), // g1
    Bitboard::new(0x0900_0042_2201_8100), // h1
    Bitboard::new(0x0208_8002_28c0_0081), // a2
    Bitboard::new(0x2280_4010_0340_2000), // b2
    Bitboard::new(0x0008_8010_0020_0184), // c2
    Bitboard::new(0x0001_0020_1000_0900), // d2
    Bitboard::new(0x0182_0006_0010_6008), // e2
    Bitboard::new(0x2058_8004_0080_0200), // f2
    Bitboard::new(0x0004_8002_0080_0900), // g2
    Bitboard::new(0x052d_0012_0040_a100), // h2
    Bitboard::new(0x0540_0880_0080_24c1), // a3
    Bitboard::new(0x2000_8480_4002_2000), // b3
    Bitboard::new(0x0400_4100_1100_6000), // c3
    Bitboard::new(0x0040_a100_3001_0108), // d3
    Bitboard::new(0x1204_8080_0800_0402), // e3
    Bitboard::new(0x0802_8080_0400_2201), // f3
    Bitboard::new(0x1002_8080_5200_0500), // g3
    Bitboard::new(0x0004_0a00_2112_4184), // h3
    Bitboard::new(0x0640_0128_8008_8040), // a4
    Bitboard::new(0x8410_4000_8020_008a), // b4
    Bitboard::new(0x0400_2008_8010_0080), // c4
    Bitboard::new(0x2001_0121_0009_1004), // d4
    Bitboard::new(0x1200_0d01_0008_0010), // e4
    Bitboard::new(0x6004_0004_0120_1008), // f4
    Bitboard::new(0x7500_aa04_0008_4110), // g4
    Bitboard::new(0x0100_0052_0004_0981), // h4
    Bitboard::new(0x0040_8040_0280_0020), // a5
    Bitboard::new(0x0470_0020_0640_0240), // b5
    Bitboard::new(0x0001_2000_8080_1000), // c5
    Bitboard::new(0x0000_0812_0200_2040), // d5
    Bitboard::new(0x00c0_8044_0080_0800), // e5
    Bitboard::new(0x9000_800a_0080_0400), // f5
    Bitboard::new(0x0001_0004_0100_0600), // g5
    Bitboard::new(0x0042_1088_ca00_2401), // h5
    Bitboard::new(0x0000_c000_228d_8000), // a6
    Bitboard::new(0x6410_0420_1440_4001), // b6
    Bitboard::new(0x1002_0040_8226_0014), // c6
    Bitboard::new(0x206a_0088_11c2_0021), // d6
    Bitboard::new(0x0002_0018_1022_0024), // e6
    Bitboard::new(0x2001_0200_0400_8080), // f6
    Bitboard::new(0x1000_0801_100c_001a), // g6
    Bitboard::new(0x0048_0082_5402_0011), // h6
    Bitboard::new(0x48FF_FE99_FECF_AA00), // a7
    Bitboard::new(0x48FF_FE99_FECF_AA00), // b7
    Bitboard::new(0x497F_FFAD_FF9C_2E00), // c7
    Bitboard::new(0x613F_FFDD_FFCE_9200), // d7
    Bitboard::new(0xffff_ffe9_ffe7_ce00), // e7
    Bitboard::new(0xffff_fff5_fff3_e600), // f7
    Bitboard::new(0x0003_ff95_e5e6_a4c0), // g7
    Bitboard::new(0x510F_FFF5_F63C_96A0), // h7
    Bitboard::new(0xEBFF_FFB9_FF9F_C526), // a8
    Bitboard::new(0x61FF_FEDD_FEED_AEAE), // b8
    Bitboard::new(0x53BF_FFED_FFDE_B1A2), // c8
    Bitboard::new(0x127F_FFB9_FFDF_B5F6), // d8
    Bitboard::new(0x411F_FFDD_FFDB_F4D6), // e8
    Bitboard::new(0x0822_0024_0810_4502), // f8
    Bitboard::new(0x0003_ffef_27ee_be74), // g8
    Bitboard::new(0x7645_FFFE_CBFE_A79E), // h8
];

/// A saved list of magics for bishops created using the generator. Some magics
/// for sizes below the required bitshift amount were taken from the
/// Chessprogramming Wiki.
const SAVED_BISHOP_MAGICS: [Bitboard; 64] = [
    Bitboard::new(0xffed_f9fd_7cfc_ffff), // a1
    Bitboard::new(0xfc09_6285_4a77_f576), // b1
    Bitboard::new(0x0012_2808_c102_a004), // c1
    Bitboard::new(0x2851_2400_8240_0440), // d1
    Bitboard::new(0x0011_1040_1100_0202), // e1
    Bitboard::new(0x0008_2208_2000_0010), // f1
    Bitboard::new(0xfc0a_66c6_4a7e_f576), // g1
    Bitboard::new(0x7ffd_fdfc_bd79_ffff), // h1
    Bitboard::new(0xfc08_46a6_4a34_fff6), // a2
    Bitboard::new(0xfc08_7a87_4a3c_f7f6), // b2
    Bitboard::new(0x0009_8802_0420_a000), // c2
    Bitboard::new(0x8000_4404_0080_8200), // d2
    Bitboard::new(0x208c_8450_c001_3407), // e2
    Bitboard::new(0x1980_1105_2010_8030), // f2
    Bitboard::new(0xfc08_64ae_59b4_ff76), // g2
    Bitboard::new(0x3c08_60af_4b35_ff76), // h2
    Bitboard::new(0x73C0_1AF5_6CF4_CFFB), // a3
    Bitboard::new(0x41A0_1CFA_D64A_AFFC), // b3
    Bitboard::new(0x0604_0002_04a2_0202), // c3
    Bitboard::new(0x0002_8208_0602_4000), // d3
    Bitboard::new(0x008a_0024_2201_0201), // e3
    Bitboard::new(0x2082_0040_8801_0802), // f3
    Bitboard::new(0x7c0c_028f_5b34_ff76), // g3
    Bitboard::new(0xfc0a_028e_5ab4_df76), // h3
    Bitboard::new(0x0810_0420_d104_1080), // a4
    Bitboard::new(0x0904_5100_0210_0100), // b4
    Bitboard::new(0x0202_2808_0406_4403), // c4
    Bitboard::new(0x004c_0040_0c03_0082), // d4
    Bitboard::new(0x0602_0010_0200_5011), // e4
    Bitboard::new(0x7209_0200_c108_9000), // f4
    Bitboard::new(0x4211_4104_2400_8805), // g4
    Bitboard::new(0x0002_8484_2126_0804), // h4
    Bitboard::new(0xc001_0412_1121_2004), // a5
    Bitboard::new(0x0208_0188_0004_4800), // b5
    Bitboard::new(0x0080_2064_1058_0800), // c5
    Bitboard::new(0x0000_2011_0008_0084), // d5
    Bitboard::new(0x0208_0034_0009_4100), // e5
    Bitboard::new(0x2190_4102_0000_4058), // f5
    Bitboard::new(0x0188_8214_0180_8080), // g5
    Bitboard::new(0x2006_0a02_0000_c4c0), // h5
    Bitboard::new(0xDCEF_D9B5_4BFC_C09F), // a6
    Bitboard::new(0xF95F_FA76_5AFD_602B), // b6
    Bitboard::new(0x200a_1041_1000_2040), // c6
    Bitboard::new(0x0800_000c_0831_0c00), // d6
    Bitboard::new(0x0218_0401_0a01_0400), // e6
    Bitboard::new(0x1092_2004_0022_4100), // f6
    Bitboard::new(0x43ff_9a5c_f4ca_0c01), // g6
    Bitboard::new(0x4BFF_CD8E_7C58_7601), // h6
    Bitboard::new(0xfc0f_f286_5334_f576), // a7
    Bitboard::new(0xfc0b_f6ce_5924_f576), // b7
    Bitboard::new(0x8052_2060_8c30_0001), // c7
    Bitboard::new(0x2084_1050_4202_0400), // d7
    Bitboard::new(0xe018_8010_2206_0220), // e7
    Bitboard::new(0x0001_1220_4901_0200), // f7
    Bitboard::new(0xc3ff_b7dc_36ca_8c89), // g7
    Bitboard::new(0xc3ff_8a54_f4ca_2c89), // h7
    Bitboard::new(0xffff_fcfc_fd79_edff), // a8
    Bitboard::new(0xfc08_63fc_cb14_7576), // b8
    Bitboard::new(0x40a0_0400_6213_3000), // c8
    Bitboard::new(0x0142_0280_0084_0400), // d8
    Bitboard::new(0x0009_0900_1006_1200), // e8
    Bitboard::new(0x0800_8445_2810_0308), // f8
    Bitboard::new(0xfc08_7e8e_4bb2_f736), // g8
    Bitboard::new(0x43ff_9e4e_f4ca_2c89), // h8
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
pub struct AttacksTable {
    rook_table: [SquareAttacks; 64],
    bishop_table: [SquareAttacks; 64],
}

impl AttacksTable {
    /// Create an empty `AttacksTable`.
    fn new() -> AttacksTable {
        let rook_table = {
            // SAFETY: We will immediately overwrite this.
            let mut data: [MaybeUninit<SquareAttacks>; 64] =
                unsafe { MaybeUninit::uninit().assume_init() };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(SquareAttacks::new());
            }
            // SAFETY: The entire block was overwritten with correct data.
            unsafe { transmute(data) }
        };
        let bishop_table = {
            // SAFETY: We will immediately overwrite this.
            let mut data: [MaybeUninit<SquareAttacks>; 64] =
                unsafe { MaybeUninit::uninit().assume_init() };
            for elem in &mut data[..] {
                *elem = MaybeUninit::new(SquareAttacks::new());
            }
            // SAFETY: The entire block was overwritten with correct data.
            unsafe { transmute(data) }
        };
        AttacksTable {
            rook_table,
            bishop_table,
        }
    }

    /// Create a pre-loaded `AttacksTable`.
    fn load() -> AttacksTable {
        let mut table = AttacksTable::new();
        load_magic_helper(&mut table.rook_table, true);
        load_magic_helper(&mut table.bishop_table, false);

        table
    }

    #[allow(unused)]
    /// Create a `AttacksTable` from scratch, generating new magics.
    pub fn make() -> AttacksTable {
        let mut table = AttacksTable::new();
        make_magic_helper(&mut table.rook_table, true);
        make_magic_helper(&mut table.bishop_table, false);

        table
    }

    #[inline(always)]
    /// Get the attacks that a rook on `sq` could make with the reference table
    /// `table`.
    pub fn rook_attacks(&self, occupancy: Bitboard, sq: Square) -> Bitboard {
        get_attacks(occupancy, sq, &self.rook_table)
    }

    #[inline(always)]
    /// Get the attacks that a bishop on `sq` could make with the reference
    /// table `table`.
    pub fn bishop_attacks(&self, occupancy: Bitboard, sq: Square) -> Bitboard {
        get_attacks(occupancy, sq, &self.bishop_table)
    }
}

/// A structure containing all the information needed to generate moves for a
/// rook or bishop from one square.
#[derive(Clone, Debug)]
struct SquareAttacks {
    /// A mask which, when &ed with the occupancy bitboard, will give only the
    /// bits that matter when computing moves.
    mask: Bitboard,
    /// The magic number to multiply to hash the current board effectively.
    magic: Bitboard,
    /// A lookup vector of squares attacked.
    attacks: Vec<Bitboard>,
    /// The shift related to this square.
    shift: u8,
}

impl SquareAttacks {
    /// Create an empty `SquareAttacks`.
    fn new() -> SquareAttacks {
        SquareAttacks {
            mask: Bitboard::EMPTY,
            magic: Bitboard::EMPTY,
            attacks: Vec::new(),
            shift: 0,
        }
    }
}

/// A helper function to load data into a `AttacksTable`. `is_rook` is `true` if
/// you are loading data for a rook, and `false` for a bishop.
fn load_magic_helper(table: &mut [SquareAttacks; 64], is_rook: bool) {
    #[allow(clippy::cast_possible_truncation)]
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
        let num_points = table[i].mask.len();
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
fn get_attacks(occupancy: Bitboard, sq: Square, table: &[SquareAttacks; 64]) -> Bitboard {
    // SAFETY: `sq` is a valid square, so accessing it by array lookup is OK.
    // Additionally, we can trust that the key was masked correctly in
    // `compute_magic_key` as it was shifted out properly.
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
fn make_magic_helper(table: &mut [SquareAttacks; 64], is_rook: bool) {
    #[allow(clippy::cast_possible_truncation)]
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
        let num_points = table[i].mask.len();

        // we know that there are at most 12 pieces that will matter when it
        // comes to attack lookups
        let mut occupancies = [Bitboard::EMPTY; 1 << 12];
        let mut attacks = [Bitboard::EMPTY; 1 << 12];

        // compute every possible occupancy arrangement for attacking
        for j in 0..(1 << num_points) {
            occupancies[j] = index_to_occupancy(j, table[i].mask);
            // compute attacks
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
        if found_magic {
            // found a magic, populate the attack vector
            table[i]
                .attacks
                .resize(1 << (64 - table[i].shift), Bitboard::EMPTY);
            for j in 0..(1 << num_points) {
                let key = compute_magic_key(occupancies[j], table[i].magic, table[i].shift);
                table[i].attacks[key] = attacks[j];
            }
        } else {
            println!(
                "failed to find {} magic for square {sq}",
                if is_rook { "rook" } else { "bishop" }
            );
            panic!();
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
    let col_mask = Bitboard::new(0x0001_0101_0101_0100 << (index % 8));
    // note: pieces at the end of the travel don't matter, which is why the
    // masks aren't uniform

    // in the col mask or row mask, but not the piece to move
    // xor operation will remove the square the piece is on
    (row_mask ^ col_mask) & !Bitboard::from(sq)
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
/// Create the mask for the relevant bits in magic of a bishop. `sq` is the
/// square that a bishop would be on to receiver this mask.
fn get_bishop_mask(sq: Square) -> Bitboard {
    /// A Bitboard made of 1's around the ring of the board, and 0's in the middle
    const RING_MASK: Bitboard = Bitboard::new(0xFF81_8181_8181_81FF);

    // thank u chessprogramming wiki for this code
    (Bitboard::diagonal(sq) ^ Bitboard::anti_diagonal(sq)) & !RING_MASK
}

/// Given some mask, create the occupancy bitboard according to this index.
/// `index` must be less than or equal to 2 ^ (number of ones in `mask`).
///
/// For instance: if `mask` repreresented a board like the following:
/// ```text
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
/// ```text
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
    let num_points = mask.len();
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
            result.insert(current_square);
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
    let mut result = Bitboard::new(fastrand::u64(..));
    for _ in 0..2 {
        result &= Bitboard::new(fastrand::u64(..));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rook_mask() {
        //println!("{:064b}", get_rook_mask(A1).0);
        assert_eq!(
            get_rook_mask(Square::A1),
            Bitboard::new(0x0001_0101_0101_017E)
        );

        //println!("{:064b}", get_rook_mask(E1).0);
        assert_eq!(
            get_rook_mask(Square::E1),
            Bitboard::new(0x0010_1010_1010_106E)
        );

        //println!("{:064b}", get_rook_mask(E5).0);
        assert_eq!(
            get_rook_mask(Square::E5),
            Bitboard::new(0x0010_106E_1010_1000)
        );
    }

    #[test]
    fn bishop_mask() {
        //println!("{:064b}", get_bishop_mask(A1).0);
        assert_eq!(
            get_bishop_mask(Square::A1),
            Bitboard::new(0x0040_2010_0804_0200)
        );

        //println!("{:064b}", get_bishop_mask(E1).0);
        assert_eq!(
            get_bishop_mask(Square::E1),
            Bitboard::new(0x0000_0000_0244_2800)
        );

        //println!("{:064b}", get_bishop_mask(E5).0);
        assert_eq!(
            get_bishop_mask(Square::E5),
            Bitboard::new(0x0044_2800_2844_0200)
        );
    }

    #[test]
    fn valid_index_to_occupancy() {
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
    fn magic_creation() {
        AttacksTable::make();
    }
    */

    #[test]
    fn magic_rook_attacks() {
        let table = AttacksTable::load();
        // cases in order:
        //rook on A1 blocked by other pieces, so it only attacks its neighbors
        //likewise, but there are other pieces on the board to be masked out
        let occupancies = [Bitboard::new(0x103), Bitboard::new(0x1FC3)];
        let squares = [Square::A1, Square::A1];
        let attacks = [Bitboard::new(0x102), Bitboard::new(0x102)];
        for i in 0..1 {
            let resulting_attack = table.rook_attacks(occupancies[i], squares[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }

    #[test]
    fn magic_bishop_attacks() {
        // cases in order:
        // bishop on A1 is blocked by piece on B2, so it only has 1 attack
        // bishop on A8 is blocked by piece on B7, so it only has 1 attack
        // bishop is in board start position on C1
        // bishop in board start position on F1
        let occupancies = [
            Bitboard::new(0x0000_0000_0000_0201), //
            Bitboard::new(0x0102_0000_0000_0000), //
            Bitboard::new(0xFFFF_0000_0000_FFFF), //
            Bitboard::new(0xFFFF_0000_0000_FFFF), //
        ];
        let squares = [
            Square::A1, //
            Square::A8, //
            Square::C1, //
            Square::F1, //
        ];
        let attacks = [
            Bitboard::new(0x0000_0000_0000_0200), //
            Bitboard::new(0x0002_0000_0000_0000), //
            Bitboard::new(0x0000_0000_0000_0A00), //
            Bitboard::new(0x0000_0000_0000_5000), //
        ];
        for i in 0..3 {
            let resulting_attack =
                directional_attacks(squares[i], &Direction::BISHOP_DIRECTIONS, occupancies[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }

    #[test]
    fn bishop_attacks() {
        let table = AttacksTable::load();
        // cases in order:
        // bishop on A1 is blocked by piece on B2, so it only has 1 attack
        // bishop on A8 is blocked by piece on B7, so it only has 1 attack
        // bishop is in board start position on C1
        // bishop in board start position on F1
        let occupancies = [
            Bitboard::new(0x0000_0000_0000_0201), //
            Bitboard::new(0x0102_0000_0000_0000), //
            Bitboard::new(0xFFFF_0000_0000_FFFF), //
            Bitboard::new(0xFFFF_0000_0000_FFFF), //
        ];
        let squares = [
            Square::A1, //
            Square::A8, //
            Square::C1, //
            Square::F1, //
        ];
        let attacks = [
            Bitboard::new(0x0000_0000_0000_0200), //
            Bitboard::new(0x0002_0000_0000_0000), //
            Bitboard::new(0x0000_0000_0000_0A00), //
            Bitboard::new(0x0000_0000_0000_5000), //
        ];
        for i in 0..3 {
            let resulting_attack = table.bishop_attacks(occupancies[i], squares[i]);
            assert_eq!(attacks[i], resulting_attack);
        }
    }
}
