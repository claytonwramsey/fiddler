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

//! King safety evaluation.

use std::mem::transmute;

use crate::base::{
    game::Game,
    movegen::{bishop_moves, rook_moves, KNIGHT_MOVES},
    Bitboard, Color, Piece,
};

use super::Score;

/// The score associated with each unit quantity of attack on the area surrounding the king.
pub const UNIT_SCORES: [Score; 256] = unsafe {
    transmute::<[(i16, i16); 256], [Score; 256]>([
        (-3, 8),
        (0, 0),
        (-6, 6),
        (-11, 0),
        (-8, 1),
        (-8, 2),
        (-13, -4),
        (-6, 5),
        (-8, 2),
        (-1, 8),
        (-7, 1),
        (-6, 3),
        (-3, 6),
        (-3, 2),
        (-4, 2),
        (-4, 3),
        (-4, 1),
        (-3, 2),
        (-2, 0),
        (1, 0),
        (-2, 5),
        (0, 0),
        (0, 1),
        (1, 0),
        (2, 0),
        (1, 1),
        (1, 0),
        (3, -1),
        (5, -3),
        (9, -7),
        (8, -5),
        (8, -6),
        (6, -4),
        (7, -4),
        (10, -7),
        (4, -1),
        (7, -5),
        (2, -1),
        (1, 0),
        (2, -1),
        (1, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (9, -7),
        (0, 0),
        (2, -1),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (0, 0),
    ])
};

#[must_use]
/// Get an evaluation of the game based on king safety from the player-to-move's perspective.
pub fn evaluate(g: &Game) -> Score {
    let w_attack_score = UNIT_SCORES[usize::from(attack_units::<{ Color::White }>(g))];
    let b_attack_score = UNIT_SCORES[usize::from(attack_units::<{ Color::Black }>(g))];

    match g.meta().player {
        Color::White => w_attack_score - b_attack_score,
        Color::Black => b_attack_score - w_attack_score,
    }
}

const ATTACKER_UNITS: [u8; 4] = [
    2, // N
    2, // B
    3, // R
    5, // Q
];

#[must_use]
/// Get the "attack units" of a given attacker, which is a number that describes the amount of
/// material dedicated to an attack.
pub fn attack_units<const ATTACKER: Color>(g: &Game) -> u8 {
    #[allow(clippy::cast_sign_loss)]
    const NEIGHBORHOODS: [[Bitboard; 2]; 64] = {
        const W_DIRS: [i8; 12] = [-9, -8, -7, -1, 0, 1, 7, 8, 9, 15, 16, 17];
        const B_DIRS: [i8; 12] = [-17, -16, -15, -9, -8, -7, -1, 0, 1, 7, 8, 9];

        const fn make_mask(dirs: &[i8], i: i8) -> Bitboard {
            let mut mask = 0;
            let mut j = 0;
            while j < dirs.len() {
                let add_sq = i + dirs[j];
                if 0 < add_sq
                    && add_sq < 64
                    && (add_sq & 7).abs_diff(i & 7) <= 2
                    && (add_sq / 8).abs_diff(i / 8) <= 2
                {
                    mask |= 1 << add_sq;
                }
                j += 1;
            }

            Bitboard::new(mask)
        }

        let mut neighborhoods = [[Bitboard::EMPTY; 2]; 64];
        let mut i = 0i8;
        while i < 64 {
            neighborhoods[i as usize][0] = make_mask(&W_DIRS, i);
            neighborhoods[i as usize][1] = make_mask(&B_DIRS, i);
            i += 1;
        }

        neighborhoods
    };

    let king_sq = g.king_sq(!ATTACKER);
    let neighbor_mask = match ATTACKER {
        Color::White => NEIGHBORHOODS[king_sq as usize][1],
        Color::Black => NEIGHBORHOODS[king_sq as usize][0],
    };

    let mut units = 0;
    let attackers = g.by_color(ATTACKER);
    let occupancy = g.occupancy();

    // TODO consider handling pinned pieces?

    for knight_sq in g.knights() & attackers {
        units += ATTACKER_UNITS[Piece::Knight as usize]
            * (KNIGHT_MOVES[knight_sq as usize] & neighbor_mask).len();
    }

    for bishop_sq in g.bishops() & attackers {
        units += ATTACKER_UNITS[Piece::Bishop as usize]
            * (bishop_moves(occupancy, bishop_sq) & neighbor_mask).len();
    }

    for rook_sq in g.rooks() & attackers {
        units += ATTACKER_UNITS[Piece::Rook as usize]
            * (rook_moves(occupancy, rook_sq) & neighbor_mask).len();
    }

    for queen_sq in g.queens() & attackers {
        units += ATTACKER_UNITS[Piece::Queen as usize]
            * ((bishop_moves(occupancy, queen_sq) | rook_moves(occupancy, queen_sq))
                & neighbor_mask)
                .len();
    }

    units
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_attacks() {
        let g = Game::new();
        assert_eq!(attack_units::<{ Color::White }>(&g), 0);
        assert_eq!(attack_units::<{ Color::Black }>(&g), 0);
    }

    #[test]
    fn queen_attacks() {
        let g = Game::from_fen("8/8/5k2/8/4Q3/8/1K6/8 w - - 0 1").unwrap();

        assert_eq!(
            attack_units::<{ Color::White }>(&g),
            7 * ATTACKER_UNITS[Piece::Queen as usize]
        );
        assert_eq!(attack_units::<{ Color::Black }>(&g), 0);
    }

    #[test]
    fn combined_attack() {
        let g = Game::from_fen("8/2kr4/8/8/3BQ3/4K3/8/3n4 w - - 0 1").unwrap();

        assert_eq!(
            attack_units::<{ Color::White }>(&g),
            2 * ATTACKER_UNITS[Piece::Bishop as usize] + 3 * ATTACKER_UNITS[Piece::Queen as usize]
        );

        assert_eq!(
            attack_units::<{ Color::Black }>(&g),
            2 * ATTACKER_UNITS[Piece::Knight as usize] + 2 * ATTACKER_UNITS[Piece::Rook as usize]
        );
    }

    #[test]
    fn border_overflow() {
        let g = Game::from_fen("8/6k1/7r/8/8/8/8/K7 w - - 0 1").unwrap();

        assert_eq!(attack_units::<{ Color::White }>(&g), 0);
        assert_eq!(attack_units::<{ Color::Black }>(&g), 0);
    }
}
