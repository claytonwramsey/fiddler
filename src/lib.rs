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

const SAVED_ROOK_MAGICS: [u64; 64] = [0; 64];

const ROOK_BITS: [u8; 64] = [1; 64];

const ROOK_MASKS: [u64; 64] = {
    let mut masks = [0; 64];
    let mut i = 0u8;
    while i < 64 {
        masks[i as usize] = get_rook_mask(i);
        i += 1;
    }
    masks
};

#[allow(unused)]
static ROOK_ATTACKS_TABLE: [u64; 999_999] =
    construct_magic_table(&ROOK_BITS, &SAVED_ROOK_MAGICS, &ROOK_MASKS);

const fn construct_magic_table<const N: usize>(
    bits: &[u8; 64],
    magics: &[u64; 64],
    masks: &[u64; 64],
) -> [u64; N] {
    let mut table = [0; N];

    let mut i = 0;
    let mut table_offset = 0;

    while i < 64 {
        let mask = masks[i];
        let magic = magics[i];
        let n_attacks_to_generate = 1 << mask.count_ones();

        let mut j = 0;
        while j < n_attacks_to_generate {
            let occupancy = index_to_occupancy(j, mask);
            let attack = 0;
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

const fn get_rook_mask(sq: u8) -> u64 {
    (0x7E << (sq & !0x7) ^ 0x0001_0101_0101_0100 << (sq & 0x7)) & !(1 << sq as u64)
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