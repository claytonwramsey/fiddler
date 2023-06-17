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

use std::mem::transmute;

const SAVED_ROOK_MAGICS: [u64; 64] = [0; 64];

const ROOK_BITS: [u8; 64] = [1; 64];

const ROOK_MASKS: [u64; 64] = {
    let mut masks = [0; 64];
    let mut i = 0u8;
    while i < 64 {
        masks[i as usize] = get_rook_mask(unsafe { transmute(i) });
        i += 1;
    }
    masks
};

#[allow(unused)]
static ROOK_ATTACKS_TABLE: [u64; 999_999] = construct_magic_table(
    &ROOK_BITS,
    &SAVED_ROOK_MAGICS,
    &ROOK_MASKS,
    &[8, -8, 1, -1],
);

const fn construct_magic_table<const N: usize>(
    bits: &[u8; 64],
    magics: &[u64; 64],
    masks: &[u64; 64],
    dirs: &[i8],
) -> [u64; N] {
    let mut table = [0; N];

    let mut i = 0;
    let mut table_offset = 0;

    while i < 64 {
        let sq: Square = unsafe { transmute(i as u8) };
        let mask = masks[i];
        let magic = magics[i];
        let n_attacks_to_generate = 1 << mask.count_ones();

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

const fn compute_magic_key(occupancy: u64, magic: u64, shift: u8) -> usize {
    (occupancy.wrapping_mul(magic) >> shift) as usize
}

const fn get_rook_mask(sq: Square) -> u64 {
    let index = sq as u8;
        (0x7E << (index & !0x7) ^ 0x0001_0101_0101_0100 << (index & 0x7)) & !(1 << sq as u64)
}

const fn index_to_occupancy(index: usize, mask: u64) -> u64 {
    let mut result = 0u64;
    let num_points = mask.count_ones();
    let mut editable_mask = mask;
    let mut i = 0;
    while i < num_points {
        let shift_size = editable_mask.trailing_zeros();
        let occupier = 1 << shift_size;
        editable_mask &= !occupier;
        if (index & (1 << i)) != 0 {
            result |= occupier;
        }
        i += 1;
    }

    result
}

const fn directional_attacks(sq: Square, dirs: &[i8], occupancy: u64) -> u64 {
    let mut result = 0;
    let mut dir_idx = 0;
    while dir_idx < dirs.len() {
        let dir = dirs[dir_idx];
        let mut current_square = sq;
        let mut loop_idx = 0;
        while loop_idx < 7 {
            let next_square_int = current_square as i16 + dir as i16;
            if next_square_int < 0 || 64 <= next_square_int {
                break;
            }
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let next_square: Square = unsafe { transmute(next_square_int as u8) };
            if next_square.chebyshev_to(current_square) > 1 {
                break;
            }
            result |= 1 << next_square as u8;
            if (occupancy & (1 << next_square as u8)) == 0 {
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

enum Square {
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
    const fn rank(self) -> u8 {
        self as u8 >> 3u8
    }

    const fn file(self) -> u8 {
        self as u8 & 7u8
    }

    const fn chebyshev_to(self, rhs: Square) -> u8 {
        let rankdiff = ((rhs.rank() as i8) - (self.rank() as i8)).unsigned_abs();
        let filediff = self.file_distance(rhs);
        if rankdiff > filediff {
            rankdiff
        } else {
            filediff
        }
    }

    const fn file_distance(self, rhs: Square) -> u8 {
        ((rhs.file() as i8) - (self.file() as i8)).unsigned_abs()
    }
}
