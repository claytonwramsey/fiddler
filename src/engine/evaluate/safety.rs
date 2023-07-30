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
    Bitboard, Color, Piece, Square,
};

use super::Score;

/// The score associated with each unit quantity of attack on the area surrounding the king.
pub const UNIT_SCORES: [Score; 256] = unsafe {
    transmute::<[(i16, i16); 256], [Score; 256]>([
        (-1, 9),
        (0, 0),
        (-5, 5),
        (-9, 1),
        (-7, 1),
        (-6, 2),
        (-11, -3),
        (-5, 4),
        (-6, 2),
        (-1, 7),
        (-5, 1),
        (-5, 2),
        (-3, 5),
        (-2, 1),
        (-3, 1),
        (-3, 2),
        (-3, 1),
        (-2, 1),
        (-2, 0),
        (1, 0),
        (-2, 3),
        (0, 0),
        (0, 0),
        (1, 0),
        (2, 0),
        (1, 0),
        (1, 0),
        (3, -1),
        (4, -2),
        (8, -6),
        (7, -4),
        (6, -5),
        (5, -3),
        (5, -3),
        (8, -6),
        (3, -1),
        (5, -4),
        (1, -1),
        (1, 0),
        (1, -1),
        (1, 0),
        (0, 0),
        (0, 0),
        (0, 0),
        (7, -5),
        (0, 0),
        (1, -1),
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
    let b3_neighborhood: Bitboard = match ATTACKER {
        Color::White => Bitboard::new(0x00_0707_0707),
        Color::Black => Bitboard::new(0x07_0707_0700),
    };

    let king_sq = g.king_sq(!ATTACKER);
    let neighbor_mask = if (king_sq as u8) < (Square::B3 as u8) {
        b3_neighborhood >> ((Square::B3 as u8) - (king_sq as u8))
    } else {
        b3_neighborhood << ((king_sq as u8) - (Square::B3 as u8))
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
}
