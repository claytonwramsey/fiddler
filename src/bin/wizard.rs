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

//! A binary program used to search for magic numbers.
//!
//! # Arguments
//!
//! Each argument is given to the binary in order, as follows:
//! 1. "rook" or "bishop", determining whether the searcher should find magic numbers for a rook or
//!    bishop.
//! 1. The square for which we should find magic numbers
//! 1. Log-base-2 of the size of the square's attacks table.
//! 1. The number of tries to take when searching.
//! 1. (optional) The random seed to use.

use std::thread::scope;

use fiddler::base::{
    movegen::{bishop_moves, rook_moves},
    Bitboard, Square,
};

struct Args {
    is_rook: bool,
    sq: Square,
    bits: u8,
    tries: u64,
    seed: Option<u64>,
}

fn main() -> Result<(), ()> {
    let Ok(args) = parse_args() else {
        println!("usage: wizard rook|bishop <square> <bits> [seed]");
        return Err(());
    };

    let occ_mask = if args.is_rook {
        get_rook_mask(args.sq)
    } else {
        get_bishop_mask(args.sq)
    };
    scope(|s| {
        // TODO make number of search threads an argument
        let handles: Vec<_> = (0..16)
            .map(|thread_id| {
                s.spawn(move || {
                    let mut rand_state = args.seed.unwrap_or(12345 + thread_id);
                    'a: for _ in 0..args.tries {
                        let mut all_attacks = vec![Bitboard::EMPTY; 1 << args.bits];
                        let magic = random_magic(&mut rand_state);
                        for i in 0..(1 << u64::from(occ_mask.len())) {
                            let occupancy = pdep(occ_mask, i);
                            let attacks = if args.is_rook {
                                rook_moves(occupancy, args.sq)
                            } else {
                                bishop_moves(occupancy, args.sq)
                            };
                            let idx = (occupancy.as_u64().wrapping_mul(magic) >> (64 - args.bits))
                                as usize;
                            if !all_attacks[idx].is_empty() && all_attacks[idx] != attacks {
                                continue 'a;
                            }
                            all_attacks[idx] = attacks;
                        }

                        println!("found magic {magic:#0x}");
                    }
                })
            })
            .collect();

        handles.into_iter().for_each(|h| h.join().unwrap());
    });

    Ok(())
}

/// Compute a random magic number, which is likely to be sparse.
fn random_magic(state: &mut u64) -> u64 {
    xorshift(state) & xorshift(state) // & xorshift(state)
}

/// Create the mask for the relevant bits in magic of a rook.
/// `sq` is the identifying the square that we want to generate the mask for.
const fn get_rook_mask(sq: Square) -> Bitboard {
    let row_mask = 0x7E << (sq as u8 & !0x7);
    let col_mask = 0x0001_0101_0101_0100 << (sq as u8 & 0x7);
    Bitboard::new((row_mask | col_mask) & !(1 << sq as u64))
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

/// Compute a random 64-bit number using the xorshift algorithm.
fn xorshift(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

/// Perform a parallel bits deposit from an index into a mask.
fn pdep(mask: Bitboard, index: u64) -> Bitboard {
    #[cfg(target_arch = "x86_64")]
    {
        Bitboard::new(unsafe { core::arch::x86_64::_pdep_u64(index, mask.as_u64()) })
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let mut result = 0u64;
        let mut editable_mask = mask.as_u64();
        for i in 0..mask.len() {
            let occupier = 1 << editable_mask.trailing_zeros();
            editable_mask &= !occupier;
            if (index & (1 << i)) != 0 {
                result |= occupier;
            }
        }

        Bitboard::new(result)
    }
}

/// Attempt to parse the command-line arguments.
fn parse_args() -> Result<Args, ()> {
    let args = std::env::args().collect::<Vec<String>>();
    if !(4..=5).contains(&args.len()) {
        return Err(());
    }

    let is_rook = match args[1].as_str() {
        "rook" => true,
        "bishop" => false,
        _ => return Err(()),
    };
    let sq = Square::from_algebraic(&args[2]).map_err(|_| ())?;
    let bits = args[3].parse().map_err(|_| ())?;
    let tries = args[4].parse().map_err(|_| ())?;
    let seed = match args.get(5) {
        Some(s) => Some(s.parse().map_err(|_| ())?),
        None => None,
    };

    Ok(Args {
        is_rook,
        sq,
        bits,
        tries,
        seed,
    })
}
