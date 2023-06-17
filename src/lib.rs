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

//! Shared data types and useful basic definitions found across the entire Fiddler engine.

// Many module elements are re-exported to make names more ergonomic to access.

use std::mem::transmute;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(transparent)]
/// A bitboard, which uses an integer to express a set of `Square`s.
/// This expression allows the efficient computation of set intersection, union, disjunction,
/// element selection, and more, all in constant time.
///
/// Nearly all board-related representations use `Bitboard`s as a key part of their construction.
pub struct Bitboard(u64);

impl Bitboard {
    /// A bitboard representing the empty set.
    /// Accordingly, `Bitboard::EMPTY` contains no squares, and functions exactly like the empty set
    /// in all observable behavior.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let sq = Square::A1; // this could be any square
    /// assert!(!Bitboard::EMPTY.contains(sq));
    /// ```
    pub const EMPTY: Bitboard = Bitboard::new(0);

    #[must_use]
    /// Construct a new Bitboard from a numeric literal.
    ///
    /// Internally, `Bitboard`s are 64-bit integers, where the LSB represents whether the square A1
    /// is an element, the second-least bit represents the square A2, and so on.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let bb = Bitboard::EMPTY.with_square(Square::A1);
    ///
    /// assert_eq!(bb, Bitboard::new(1));
    /// ```
    pub const fn new(x: u64) -> Bitboard {
        Bitboard(x)
    }

    #[must_use]
    /// Determine whether this bitboard contains a given square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// assert!(Bitboard::new(1).contains(Square::A1));
    /// assert!(!(Bitboard::new(2).contains(Square::A1)));
    /// ```
    pub const fn contains(self, square: Square) -> bool {
        self.0 & (1 << square as u8) != 0
    }

    #[allow(clippy::cast_possible_truncation)]
    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    /// Compute the number of squares contained in this `Bitboard`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Square};
    ///
    /// let mut bb = Bitboard::EMPTY;
    /// assert_eq!(bb.len(), 0);
    /// bb.insert(Square::A1);
    /// assert_eq!(bb.len(), 1);
    /// ```
    pub const fn len(self) -> u8 {
        self.0.count_ones() as u8
    }

    #[must_use]
    /// Convert this bitboard to a u64.
    /// This operation requires no computation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::Bitboard;
    ///
    /// assert_eq!(Bitboard::EMPTY.as_u64(), 0);
    /// assert_eq!(Bitboard::ALL.as_u64(), !0);
    /// ```
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/// A difference between two squares. `Direction`s form a vector field, which allows us to define
/// subtraction between squares.
/// Internally, they use the same representation as a `Square` but with a signed integer.
pub struct Direction(pub(crate) i8);

impl Direction {
    /* Cardinal directions */

    /// A `Direction` corresponding to a move "north" from White's point of view, in the direction
    /// a white pawn would travel.
    pub const NORTH: Direction = Direction(8);

    /// A `Direction` corresponding to a move "east" from White's point of view.
    pub const EAST: Direction = Direction(1);

    // sadly, the nature of rust consts means the following doesn't work:
    // pub const SOUTH: Direction = -NORTH;

    /// A `Direction` corresponding to a move "south" from White's point of view.
    pub const SOUTH: Direction = Direction(-8);

    /// A `Direction` corresponding to a move "west" from White's point of view.
    pub const WEST: Direction = Direction(-1);

    /* Composite directions */

    /// The directions that a rook can move, along only one step.
    pub const ROOK_DIRECTIONS: [Direction; 4] = [
        Direction::NORTH,
        Direction::SOUTH,
        Direction::EAST,
        Direction::WEST,
    ];
}


#[allow(unused)]
/// A saved list of magics for rooks created using the generator.
///
/// Some magics for sizes below the required bitshift amount were taken from the Chess Programming
/// Wiki.
pub const SAVED_ROOK_MAGICS: [u64; 64] = [
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

#[allow(unused)]
// #[allow(long_running_const_eval)]
/// The master table containing every attack that the rook can perform from every square under
/// every occupancy.
/// Borrowed by the individual [`AttacksLookup`]s in [`ROOK_LOOKUPS`].
const ROOK_ATTACKS_TABLE: [Bitboard; table_size(&ROOK_BITS)] = construct_magic_table(
    &ROOK_BITS,
    &SAVED_ROOK_MAGICS,
    &ROOK_MASKS,
    &Direction::ROOK_DIRECTIONS,
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


#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[allow(unused)]
/// A square: one of 64 spots on a `Board` that a `Piece` can occupy.
//  Internally, `Square`s are represented as a single integer to maintain a small size.
//  From MSB to LSB, each square is composed of:
//  * 2 unused bits
//  * 3 bits for the rank
//  * 3 bits for the file
pub enum Square {
    A1 = 0,
    B1 = 1,
    C1 = 2,
    D1 = 3,
    E1 = 4,
    F1 = 5,
    G1 = 6,
    H1 = 7,
    A2 = 8,
    B2 = 9,
    C2 = 10,
    D2 = 11,
    E2 = 12,
    F2 = 13,
    G2 = 14,
    H2 = 15,
    A3 = 16,
    B3 = 17,
    C3 = 18,
    D3 = 19,
    E3 = 20,
    F3 = 21,
    G3 = 22,
    H3 = 23,
    A4 = 24,
    B4 = 25,
    C4 = 26,
    D4 = 27,
    E4 = 28,
    F4 = 29,
    G4 = 30,
    H4 = 31,
    A5 = 32,
    B5 = 33,
    C5 = 34,
    D5 = 35,
    E5 = 36,
    F5 = 37,
    G5 = 38,
    H5 = 39,
    A6 = 40,
    B6 = 41,
    C6 = 42,
    D6 = 43,
    E6 = 44,
    F6 = 45,
    G6 = 46,
    H6 = 47,
    A7 = 48,
    B7 = 49,
    C7 = 50,
    D7 = 51,
    E7 = 52,
    F7 = 53,
    G7 = 54,
    H7 = 55,
    A8 = 56,
    B8 = 57,
    C8 = 58,
    D8 = 59,
    E8 = 60,
    F8 = 61,
    G8 = 62,
    H8 = 63,
}

impl Square {
    #[must_use]
    /// Get the integer representing the rank (0 -> 1, ...) of this square.
    pub const fn rank(self) -> u8 {
        self as u8 >> 3u8
    }

    #[must_use]
    /// Get the integer representing the file (0 -> A, ...) of this square.
    pub const fn file(self) -> u8 {
        self as u8 & 7u8
    }

    #[must_use]
    /// Get the Chebyshev distance to another square.
    pub const fn chebyshev_to(self, rhs: Square) -> u8 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
        {
            let rankdiff = ((rhs.rank() as i8) - (self.rank() as i8)).unsigned_abs();
            let filediff = self.file_distance(rhs);
            // we would use `max()` here if it were a const function
            if rankdiff > filediff {
                rankdiff
            } else {
                filediff
            }
        }
    }

    #[must_use]
    #[allow(clippy::cast_possible_wrap, clippy::cast_possible_truncation)]
    /// Get the distance between two `Square`s by traveling along files.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::Square;
    ///
    /// let sq1 = Square::A8;
    /// let sq2 = Square::C1;
    ///
    /// assert_eq!(sq1.file_distance(sq2), 2);
    /// ```
    pub const fn file_distance(self, rhs: Square) -> u8 {
        ((rhs.file() as i8) - (self.file() as i8)).unsigned_abs()
    }
}