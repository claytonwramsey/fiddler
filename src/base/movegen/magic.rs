/*
  Fiddler, a UCI-compatible chess engine.
  Copyright (C) 2022 Clayton Ramsey.

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

use std::mem::{transmute, MaybeUninit};

/// A lookup table for generating attacks via magic bitboard for one piece type and square.
struct AttacksLookup {
    /// A reference to this lookup's section of the magic attacks.
    table: &'static [Bitboard],
    /// The mask for extracting out the relevant occupancy map on a board.
    mask: Bitboard,
    /// The magic multiply constant for converting occupancies to indices.
    magic: u64,
    /// The shift to extract an index from a multiplied constant.
    shift: u8,
}

#[must_use]
/// Compute the set of squares that a bishop on square `sq` can see if the board is occupied by
/// `occupancy`.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::bishop_moves, Bitboard, Square};
///
/// // squares A1 and C3 are occupied
/// let occupancy = Bitboard::EMPTY
///     .with_square(Square::A1)
///     .with_square(Square::C3);
///
/// // the bishop on A1 can see B2 and C3
/// assert_eq!(
///     bishop_moves(occupancy, Square::A1),
///     Bitboard::EMPTY
///         .with_square(Square::B2)
///         .with_square(Square::C3)
/// );
/// ```
pub fn bishop_moves(occupancy: Bitboard, sq: Square) -> Bitboard {
    get_attacks(occupancy, sq, &BISHOP_LOOKUPS)
}

#[must_use]
/// Compute the set of squares that a rook on square `sq` can see if the board is occupied by
/// `occupancy`.
///
/// # Examples
///
/// ```
/// use fiddler::base::{movegen::rook_moves, Bitboard, Square};
///
/// // squares A3 and B1 are occupied
/// let occupancy = Bitboard::EMPTY
///     .with_square(Square::A3)
///     .with_square(Square::B1);
///
/// // the rook on A1 can see B1, A2, and A3
/// assert_eq!(
///     rook_moves(occupancy, Square::A1),
///     Bitboard::EMPTY
///         .with_square(Square::B1)
///         .with_square(Square::A2)
///         .with_square(Square::A3)
/// );
/// ```
pub fn rook_moves(occupancy: Bitboard, sq: Square) -> Bitboard {
    get_attacks(occupancy, sq, &ROOK_LOOKUPS)
}

/// A saved list of magics for rooks created using the generator.
///
/// Some magics for sizes below the required bitshift amount were taken from the Chess Programming
/// Wiki.
const SAVED_ROOK_MAGICS: [u64; 64] = [
    0x4080_0020_4000_1480, // a1
    0x0040_0010_0140_2000, // b1
    0x0300_2000_1810_4100, // c1
    0x2100_0409_0110_0120, // d1
    0x8a00_0600_0408_2070, // e1
    0x0080_0144_0002_0080, // f1
    0x1100_2500_208a_0004, // g1
    0x0900_0042_2201_8100, // h1
    0x0208_8002_28c0_0081, // a2
    0x2280_4010_0340_2000, // b2
    0x0008_8010_0020_0184, // c2
    0x0001_0020_1000_0900, // d2
    0x0182_0006_0010_6008, // e2
    0x2058_8004_0080_0200, // f2
    0x0004_8002_0080_0900, // g2
    0x052d_0012_0040_a100, // h2
    0x0540_0880_0080_24c1, // a3
    0x2000_8480_4002_2000, // b3
    0x0400_4100_1100_6000, // c3
    0x0040_a100_3001_0108, // d3
    0x1204_8080_0800_0402, // e3
    0x0802_8080_0400_2201, // f3
    0x1002_8080_5200_0500, // g3
    0x0004_0a00_2112_4184, // h3
    0x0640_0128_8008_8040, // a4
    0x8410_4000_8020_008a, // b4
    0x0400_2008_8010_0080, // c4
    0x2001_0121_0009_1004, // d4
    0x1200_0d01_0008_0010, // e4
    0x6004_0004_0120_1008, // f4
    0x7500_aa04_0008_4110, // g4
    0x0100_0052_0004_0981, // h4
    0x0040_8040_0280_0020, // a5
    0x0470_0020_0640_0240, // b5
    0x0001_2000_8080_1000, // c5
    0x0000_0812_0200_2040, // d5
    0x00c0_8044_0080_0800, // e5
    0x9000_800a_0080_0400, // f5
    0x0001_0004_0100_0600, // g5
    0x0042_1088_ca00_2401, // h5
    0x0000_c000_228d_8000, // a6
    0x6410_0420_1440_4001, // b6
    0x1002_0040_8226_0014, // c6
    0x206a_0088_11c2_0021, // d6
    0x0002_0018_1022_0024, // e6
    0x2001_0200_0400_8080, // f6
    0x1000_0801_100c_001a, // g6
    0x0048_0082_5402_0011, // h6
    0x48FF_FE99_FECF_AA00, // a7, found by Grant Osborne
    0x48FF_FE99_FECF_AA00, // b7, found by Grant Osborne
    0x497F_FFAD_FF9C_2E00, // c7, found by Grant Osborne
    0x613F_FFDD_FFCE_9200, // d7, found by Grant Osborne
    0xffff_ffe9_ffe7_ce00, // e7, found by Volker Annuss
    0xffff_fff5_fff3_e600, // f7, found by Volker Annuss
    0x0003_ff95_e5e6_a4c0, // g7, found by Niklas Fiekas
    0x510F_FFF5_F63C_96A0, // h7, found by Grant Osborne
    0xEBFF_FFB9_FF9F_C526, // a8, found by Grant Osborne
    0x61FF_FEDD_FEED_AEAE, // b8, found by Grant Osborne
    0x53BF_FFED_FFDE_B1A2, // c8, found by Grant Osborne
    0x127F_FFB9_FFDF_B5F6, // d8, found by Grant Osborne
    0x411F_FFDD_FFDB_F4D6, // e8, found by Grant Osborne
    0x0822_0024_0810_4502, // f8
    0x0003_ffef_27ee_be74, // g8, found by Peter Österlund 
    0x7645_FFFE_CBFE_A79E, // h8, found by Grant Osborne
];

/// A saved list of magics for bishops created using the generator.
///
/// Some magics for sizes below the required bitshift amount were taken from the Chess Programming
/// Wiki.
const SAVED_BISHOP_MAGICS: [u64; 64] = [
    0xffed_f9fd_7cfc_ffff, // a1, found by Gerd Isenberg
    0xfc09_6285_4a77_f576, // b1, found by Gerd Isenberg
    0x0012_2808_c102_a004, // c1
    0x2851_2400_8240_0440, // d1
    0x0011_1040_1100_0202, // e1
    0x0008_2208_2000_0010, // f1
    0xfc0a_66c6_4a7e_f576, // g1, found by Gerd Isenberg
    0x7ffd_fdfc_bd79_ffff, // h1, found by Gerd Isenberg
    0xfc08_46a6_4a34_fff6, // a2, found by Gerd Isenberg
    0xfc08_7a87_4a3c_f7f6, // b2, found by Gerd Isenberg
    0x0009_8802_0420_a000, // c2
    0x8000_4404_0080_8200, // d2
    0x208c_8450_c001_3407, // e2
    0x1980_1105_2010_8030, // f2
    0xfc08_64ae_59b4_ff76, // g2, found by Gerd Isenberg
    0x3c08_60af_4b35_ff76, // h2, found by Gerd Isenberg
    0x73C0_1AF5_6CF4_CFFB, // a3, found by Richard Pijl
    0x41A0_1CFA_D64A_AFFC, // b3, found by Richard Pijl
    0x0604_0002_04a2_0202, // c3
    0x0002_8208_0602_4000, // d3
    0x008a_0024_2201_0201, // e3
    0x2082_0040_8801_0802, // f3
    0x7c0c_028f_5b34_ff76, // g3, found by Gerd Isenberg
    0xfc0a_028e_5ab4_df76, // h3, found by Gerd Isenberg
    0x0810_0420_d104_1080, // a4
    0x0904_5100_0210_0100, // b4
    0x0202_2808_0406_4403, // c4
    0x004c_0040_0c03_0082, // d4
    0x0602_0010_0200_5011, // e4
    0x7209_0200_c108_9000, // f4
    0x4211_4104_2400_8805, // g4
    0x0002_8484_2126_0804, // h4
    0xc001_0412_1121_2004, // a5
    0x0208_0188_0004_4800, // b5
    0x0080_2064_1058_0800, // c5
    0x0000_2011_0008_0084, // d5
    0x0208_0034_0009_4100, // e5
    0x2190_4102_0000_4058, // f5
    0x0188_8214_0180_8080, // g5
    0x2006_0a02_0000_c4c0, // h5
    0xDCEF_D9B5_4BFC_C09F, // a6, found by Richard Pijl
    0xF95F_FA76_5AFD_602B, // b6, found by Richard Pijl
    0x200a_1041_1000_2040, // c6
    0x0800_000c_0831_0c00, // d6
    0x0218_0401_0a01_0400, // e6
    0x1092_2004_0022_4100, // f6
    0x43ff_9a5c_f4ca_0c01, // g6, found by Gerd Isenberg
    0x4BFF_CD8E_7C58_7601, // h6, found by Richard Pijl
    0xfc0f_f286_5334_f576, // a7, found by Gerd Isenberg
    0xfc0b_f6ce_5924_f576, // b7, found by Gerd Isenberg
    0x8052_2060_8c30_0001, // c7
    0x2084_1050_4202_0400, // d7
    0xe018_8010_2206_0220, // e7
    0x0001_1220_4901_0200, // f7
    0xc3ff_b7dc_36ca_8c89, // g7, found by Gerd Isenberg
    0xc3ff_8a54_f4ca_2c89, // h7, found by Gerd Isenberg
    0xffff_fcfc_fd79_edff, // a8, found by Gerd Isenberg
    0xfc08_63fc_cb14_7576, // b8, found by Gerd Isenberg
    0x40a0_0400_6213_3000, // c8
    0x0142_0280_0084_0400, // d8
    0x0009_0900_1006_1200, // e8
    0x0800_8445_2810_0308, // f8
    0xfc08_7e8e_4bb2_f736, // g8, found by Gerd Isenberg
    0x43ff_9e4e_f4ca_2c89, // h8, found by Gerd Isenberg
];

/// The number of bits used to express the magic lookups for rooks at each square.
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

/// The number of bits used to express the magic lookups for bishops at each square.
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

/// Compute the number of entries in a magic-movegen table required to store every element, given
/// the number of bits required for each square.
const fn table_size(bits_table: &[u8; 64]) -> usize {
    let mut i = 0;
    let mut total = 0;
    while i < 64 {
        total += 1 << bits_table[i];
        i += 1;
    }
    total
}

/// The bitwise masks for extracting the relevant pieces for a bishop's attacks in a board, indexed 
/// by the square occupied by the bishop.
const BISHOP_MASKS: [Bitboard; 64] = {
    let mut masks = [Bitboard::EMPTY; 64];
    let mut i = 0u8;
    while i < 64 {
        masks[i as usize] = get_bishop_mask(unsafe { transmute(i) });
        i += 1;
    }
    masks
};

/// The bitwise masks for extracting the relevant pieces for a rook's attacks in a board, indexed 
/// by the square occupied by the rook.
const ROOK_MASKS: [Bitboard; 64] = {
    let mut masks = [Bitboard::EMPTY; 64];
    let mut i = 0u8;
    while i < 64 {
        masks[i as usize] = get_rook_mask(unsafe { transmute(i) });
        i += 1;
    }
    masks
};

#[allow(long_running_const_eval)]
/// The master table containing every attack that the bishop can perform from every square under
/// every occupancy.
/// Borrowed by the individual [`AttacksLookup`]s in [`BISHOP_LOOKUPS`].
const BISHOP_ATTACKS_TABLE: [Bitboard; table_size(&BISHOP_BITS)] = construct_magic_table(
    &BISHOP_BITS,
    &SAVED_BISHOP_MAGICS,
    &BISHOP_MASKS,
    &Direction::BISHOP_DIRECTIONS,
);

#[allow(long_running_const_eval)]
/// The necessary information for generatng attacks for bishops, indexed b the square occupied by 
/// said bishop.
const BISHOP_LOOKUPS: [AttacksLookup; 64] = construct_lookups(
    &BISHOP_BITS,
    &SAVED_BISHOP_MAGICS,
    &BISHOP_MASKS,
    &BISHOP_ATTACKS_TABLE,
);

#[allow(long_running_const_eval)]
/// The master table containing every attack that the rook can perform from every square under
/// every occupancy.
/// Borrowed by the individual [`AttacksLookup`]s in [`ROOK_LOOKUPS`].
const ROOK_ATTACKS_TABLE: [Bitboard; table_size(&ROOK_BITS)] = construct_magic_table(
    &ROOK_BITS,
    &SAVED_ROOK_MAGICS,
    &ROOK_MASKS,
    &Direction::ROOK_DIRECTIONS,
);

#[allow(long_running_const_eval)]
/// The necessary information for generatng attacks for rook, indexed b the square occupied by 
/// said rook.
const ROOK_LOOKUPS: [AttacksLookup; 64] = construct_lookups(
    &ROOK_BITS,
    &SAVED_ROOK_MAGICS,
    &ROOK_MASKS,
    &ROOK_ATTACKS_TABLE,
);

#[allow(clippy::cast_possible_truncation)]
/// Construct the master magic table for a rook or bishop based on all the requisite information.
/// 
/// # Inputs
/// 
/// - `bits`: For each square, the number of other squares which are involved in the calculation of 
///   attacks from that square.
/// - `magics`: The magic numbers for each square.
/// - `masks`: The masks used for extracting the relevant squares for an attack on each starting 
///   square.
/// - `dirs`: The directions in which the piece can move
const fn construct_magic_table<const N: usize>(
    bits: &[u8; 64],
    magics: &[u64; 64],
    masks: &[Bitboard; 64],
    dirs: &[Direction],
) -> [Bitboard; N] {
    let mut table = [Bitboard::EMPTY; N];

    let mut i = 0;
    let mut table_offset = 0;

    while i < 64 {
        let sq: Square = unsafe { transmute(i as u8) };
        let mask = masks[i];
        let magic = magics[i];
        let n_attacks_to_generate = 1 << mask.len();

        let mut j = 0;
        while j < n_attacks_to_generate {
            let occupancy = index_to_occupancy(j, mask);
            let attack = directional_attacks(sq, dirs, occupancy);
            let key = compute_magic_key(occupancy, magic, 64 - bits[i]);
            table[key + table_offset] = attack;
            j += 1;
        }

        table_offset += 1 << bits[i];
        i += 1;
    }

    table
}

/// Construct the lookup tables for magic move generation by referencing an already-generated 
/// attacks table.
const fn construct_lookups(
    bits: &[u8; 64],
    magics: &[u64; 64],
    masks: &[Bitboard; 64],
    attacks_table: &'static [Bitboard],
) -> [AttacksLookup; 64] {
    unsafe {
        let mut table: [MaybeUninit<AttacksLookup>; 64] = MaybeUninit::uninit().assume_init();

        let mut remaining_attacks = attacks_table;
        let mut i = 0;
        while i < 64 {
            let these_attacks;
            (these_attacks, remaining_attacks) = remaining_attacks.split_at(1 << bits[i]);
            table[i] = MaybeUninit::new(AttacksLookup {
                table: these_attacks,
                mask: masks[i],
                magic: magics[i],
                shift: 64 - bits[i],
            });

            i += 1;
        }

        transmute(table)
    }
}

/// Get the attacks a square has, given a magic lookup table and the current occupancy.
fn get_attacks(occupancy: Bitboard, sq: Square, lookup: &[AttacksLookup; 64]) -> Bitboard {
    // SAFETY: `sq` is a valid square, so accessing it by array lookup is OK.
    // Additionally, we can trust that the key was masked correctly in `compute_magic_key` as it was
    // shifted out properly.
    let magic_data = unsafe { lookup.get_unchecked(sq as usize) };
    let key = compute_magic_key(occupancy & magic_data.mask, magic_data.magic, magic_data.shift);

    unsafe { *magic_data.table.get_unchecked(key) }
}

#[allow(clippy::cast_possible_truncation)]
/// Use magic hashing to get the index to look up attacks in a bitboad.
const fn compute_magic_key(occupancy: Bitboard, magic: u64, shift: u8) -> usize {
    (occupancy.as_u64().wrapping_mul(magic) >> shift) as usize
}

/// Create the mask for the relevant bits in magic of a rook.
/// `sq` is the square that a rook would occupy to receive this mask.
const fn get_rook_mask(sq: Square) -> Bitboard {
    let index = sq as i8;
    // sequence of 1s down the same row as the piece to move, except on the ends
    let row_mask = 0x7E << (index & !0x7);
    // sequence of 1s down the same col as the piece to move, except on the ends
    let col_mask = 0x0001_0101_0101_0100 << (index % 8);
    // note: pieces at the end of the travel don't matter, which is why the masks aren't uniform

    // in the col mask or row mask, but not the piece to move xor operation will remove the square
    // the piece is on
    Bitboard::new((row_mask ^ col_mask) & !(1 << sq as u64))
}

#[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
/// Create the mask for the relevant bits in magic of a bishop.
/// `sq` is the square that a bishop would be on to receiver this mask.
const fn get_bishop_mask(sq: Square) -> Bitboard {
    Bitboard::new(
        (Bitboard::diagonal(sq).as_u64() ^ Bitboard::anti_diagonal(sq).as_u64())
            & !0xFF81_8181_8181_81FF,
    )
}

/// Given some mask, create the occupancy [`Bitboard`] according to this index.
///
/// `index` must be less than or equal to 2 ^ (number of ones in `mask`).
/// This is equivalent to the parallel-bits-extract (PEXT) instruction on x86 architectures.
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
const fn index_to_occupancy(index: usize, mask: Bitboard) -> Bitboard {
    let mut result = 0u64;
    let num_points = mask.len();
    let mut editable_mask = mask.as_u64();
    // go from right to left in the bits of num_points,
    // and add an occupancy if something is there
    let mut i = 0;
    while i < num_points {
        let shift_size = editable_mask.trailing_zeros();
        // make a bitboard which only occupies the rightmost square
        let occupier = 1 << shift_size;
        // remove the occupier from the mask
        editable_mask &= !occupier;
        if (index & (1 << i)) != 0 {
            // the bit corresponding to the occupier is nonzero
            result |= occupier;
        }
        i += 1;
    }

    Bitboard::new(result)
}

/// Construct the squares attacked by the pieces at `sq` if it could move along the directions in
/// `dirs` when the board is occupied by the pieces in `occupancy`.
///
/// This is slow and should only be used for generatic magic bitboards (instead of for move
/// generation.
pub(crate) const fn directional_attacks(
    sq: Square,
    dirs: &[Direction],
    occupancy: Bitboard,
) -> Bitboard {
    // behold: much hackery for making this work as a const fn
    let mut result = Bitboard::EMPTY;
    let mut dir_idx = 0;
    while dir_idx < dirs.len() {
        let dir = dirs[dir_idx];
        let mut current_square = sq;
        let mut loop_idx = 0;
        while loop_idx < 7 {
            let next_square_int: i16 = current_square as i16
                + unsafe {
                    // SAFETY: All values for an `i8` are valid.
                    transmute::<Direction, i8>(dir) as i16
                };
            if next_square_int < 0 || 64 <= next_square_int {
                break;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let next_square: Square = unsafe {
                // SAFETY: We checked that this next square was in the range 0..63, which is how a
                // square is represented.
                transmute(next_square_int as u8)
            };
            if next_square.chebyshev_to(current_square) > 1 {
                break;
            }
            result = Bitboard::new(
                unsafe {
                    // SAFETY: Any value is OK for an int.
                    transmute::<Bitboard, u64>(result)
                } | 1 << next_square as u8,
            );
            if occupancy.contains(next_square) {
                break;
            }
            current_square = next_square;
            loop_idx += 1;
        }
        dir_idx += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rook_mask() {
        // println!("{:064b}", get_rook_mask(A1).0);
        assert_eq!(
            get_rook_mask(Square::A1),
            Bitboard::new(0x0001_0101_0101_017E)
        );

        // println!("{:064b}", get_rook_mask(E1).0);
        assert_eq!(
            get_rook_mask(Square::E1),
            Bitboard::new(0x0010_1010_1010_106E)
        );

        // println!("{:064b}", get_rook_mask(E5).0);
        assert_eq!(
            get_rook_mask(Square::E5),
            Bitboard::new(0x0010_106E_1010_1000)
        );
    }

    #[test]
    fn bishop_mask() {
        // println!("{:064b}", get_bishop_mask(A1).0);
        assert_eq!(
            get_bishop_mask(Square::A1),
            Bitboard::new(0x0040_2010_0804_0200)
        );

        // println!("{:064b}", get_bishop_mask(E1).0);
        assert_eq!(
            get_bishop_mask(Square::E1),
            Bitboard::new(0x0000_0000_0244_2800)
        );

        // println!("{:064b}", get_bishop_mask(E5).0);
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

    #[test]
    /// Test that bishop magic move generation is correct for every possible occupancy bitboard.
    fn all_bishop_attacks() {
        for sq in Bitboard::ALL {
            let mask = get_bishop_mask(sq);
            for i in 0..(1 << mask.len()) {
                let occupancy = index_to_occupancy(i, mask);
                let attacks = directional_attacks(sq, &Direction::BISHOP_DIRECTIONS, occupancy);
                assert_eq!(attacks, bishop_moves(occupancy, sq));
            }
        }
    }

    #[test]
    /// Test that rook magic move generation is correct for every possible occupancy bitboard.
    fn all_rook_attacks() {
        for sq in Bitboard::ALL {
            let mask = get_rook_mask(sq);
            for i in 0..(1 << mask.len()) {
                let occupancy = index_to_occupancy(i, mask);
                let attacks = directional_attacks(sq, &Direction::ROOK_DIRECTIONS, occupancy);
                assert_eq!(attacks, rook_moves(occupancy, sq));
            }
        }
    }
}
