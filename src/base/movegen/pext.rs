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

//! Implementation of sliding piece move generation using `x86_64`'s pext instruction to extract
//! indices.

use std::{
    arch::x86_64::_pext_u64,
    mem::{transmute, MaybeUninit},
};

use crate::base::{Bitboard, Direction, Square};

use super::{directional_attacks, get_bishop_mask, get_rook_mask, index_to_occupancy};

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

/// The data required to generate as set of sliding moves from one square.
struct PextLookup {
    /// A reference to this lookup's section of the attacks table.
    table: &'static [Bitboard],
    /// The mask for extracting the relevant bits of an occupancy for PEXT move generation.
    mask: u64,
}

#[allow(clippy::cast_possible_truncation)]
/// Get the attacks a square has, given a PEXT lookup table and the current occupancy.
fn get_attacks(occupancy: Bitboard, sq: Square, lookups: &[PextLookup; 64]) -> Bitboard {
    unsafe {
        let pext_data = lookups.get_unchecked(sq as usize);
        let idx = _pext_u64(occupancy.as_u64(), pext_data.mask) as usize;
        *pext_data.table.get_unchecked(idx)
    }
}

/// The size of the table of rook attacks.
const ROOK_TABLE_SIZE: usize = {
    let mut total = 0;
    let mut i = 0;
    while i < 64 {
        total += 1 << get_rook_mask(unsafe { transmute::<u8, Square>(i) }).len();
        i += 1;
    }
    total
};

/// The size of the table of bishop attacks.
const BISHOP_TABLE_SIZE: usize = {
    let mut total = 0;
    let mut i = 0;
    while i < 64 {
        total += 1 << get_bishop_mask(unsafe { transmute::<u8, Square>(i) }).len();
        i += 1;
    }
    total
};

#[allow(long_running_const_eval)]
/// The master table containing every attack that the rook can perform from every square under
/// every occupancy.
const ROOK_ATTACKS_TABLE: [Bitboard; ROOK_TABLE_SIZE] = construct_table(true);

/// A lookup from each square to the relevant section of the attacks table.
const ROOK_LOOKUPS: [PextLookup; 64] = construct_lookups(&ROOK_ATTACKS_TABLE, true);

/// The master table containing every attack that the bishop can perform from every square under
/// every occupancy.
const BISHOP_ATTACKS_TABLE: [Bitboard; BISHOP_TABLE_SIZE] = construct_table(false);

/// A lookup from each square to the relevant section of the attacks table.
const BISHOP_LOOKUPS: [PextLookup; 64] = construct_lookups(&BISHOP_ATTACKS_TABLE, false);

/// Construct the lookup tables for magic move generation by referencing an already-generated
/// attacks table.
const fn construct_lookups(attacks_table: &'static [Bitboard], is_rook: bool) -> [PextLookup; 64] {
    unsafe {
        let mut table: [MaybeUninit<PextLookup>; 64] = MaybeUninit::uninit().assume_init();

        let mut remaining_attacks = attacks_table;
        let mut i = 0;
        while i < 64 {
            let sq = transmute::<u8, Square>(i);
            let mask = if is_rook { get_rook_mask(sq) } else {get_bishop_mask(sq)};
            let these_attacks;
            (these_attacks, remaining_attacks) = remaining_attacks.split_at(1 << mask.len());
            table[i as usize] = MaybeUninit::new(PextLookup {
                table: these_attacks,
                mask: mask.as_u64(),
            });

            i += 1;
        }

        transmute(table)
    }
}

/// Construct the table of all attacks for a sliding piece.
const fn construct_table<const N: usize>(is_rook: bool) -> [Bitboard; N] {
    let mut table = [Bitboard::EMPTY; N];
    let mut offset = 0;
    let mut i = 0u8;
    while i < 64 {
        let sq = unsafe { transmute::<u8, Square>(i) };
        let mask = if is_rook {
            get_rook_mask(sq)
        } else {
            get_bishop_mask(sq)
        };
        let n_steps = 1 << mask.len();
        let mut j = 0;
        while j < n_steps {
            let occupancy = index_to_occupancy(j, mask);
            table[offset + j] = directional_attacks(
                sq,
                if is_rook {
                    &Direction::ROOK_DIRECTIONS
                } else {
                    &Direction::BISHOP_DIRECTIONS
                },
                occupancy,
            );
            j += 1;
        }

        offset += n_steps;
        i += 1;
    }

    table
}
