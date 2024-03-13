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

//! A data extractor for training the Fiddler engine.
//! This program extracts input tensors and game outcomes from an EPD file and converts them to a
//! sparse tensor representation. Then it stores the results in a large CSV file.

#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

use std::{
    env,
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    time::Instant,
};

use fiddler::base::{game::Game, Color, Piece, Square};

#[allow(clippy::similar_names, clippy::result_unit_err)]
/// Run the main training function.
///
/// The first command line argument must the the path of the file containing training data.
///
/// # Errors
///
/// This function will return an error if the EPD training data does not exist or cannot be parsed.
pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    // first argument is the name of the binary
    let path_str = match args.len() {
        0 | 1 => {
            return Err("path to a labeled EPD file must be given".into());
        }
        2 => &args[1],
        _ => {
            eprintln!("Warning: extraneous arguments are being ignored");
            &args[1]
        }
    };
    let mut tic = Instant::now();

    // construct the datasets.
    // Outer vector: each element for one datum
    // Inner vector: each element for one feature-quantity pair
    let Ok(input_sets) = extract_epd(path_str) else {
        return Err("unable to parse EPD".into());
    };

    let mut toc = Instant::now();
    println!("extracted data in {} secs", (toc - tic).as_secs_f32());

    tic = Instant::now();
    let mut outfile = BufWriter::new(File::create("input.csv")?);
    for (BoardFeatures(x1, x2), result) in input_sets {
        write!(&mut outfile, "{}", x1[0])?;
        for x in &x1[1..] {
            write!(&mut outfile, ",{x}")?;
        }
        write!(&mut outfile, ";{}", x2[0])?;
        for x in &x2[1..] {
            write!(&mut outfile, ",{x}")?;
        }
        writeln!(&mut outfile, ";{result}")?;
    }
    toc = Instant::now();
    println!("wrote to file in {} secs", (toc - tic).as_secs_f32());

    Ok(())
}

/// Compute the index of an input feature for a piece of type `piece` of color `side` on square
/// `piece_sq`, provided that the player to move has color `player` and their king is on `king_sq`.
fn input_idx(piece: Piece, player: Color, side: Color, king_sq: Square, piece_sq: Square) -> u16 {
    (king_sq as u16 * 64 * 11)
        + (piece as u16 + if player == side { 0 } else { 5 }) * 64
        + piece_sq as u16
}

/// Extracted features from a board.
struct BoardFeatures(Box<[u16]>, Box<[u16]>);

/// Expand an EPD file into a set of features that can be used for training.
fn extract_epd(location: &str) -> Result<Vec<(BoardFeatures, f32)>, Box<dyn std::error::Error>> {
    let file = File::open(location)?;
    let reader = BufReader::new(file);
    let mut data = Vec::new();

    for line_result in reader.lines() {
        let line = line_result?;
        let mut split_line = line.split("c9 \"");
        // first part of the split is the FEN, second is the score, last is just a semicolon
        // convert FEN to EPD description of the position
        let fen = split_line.next().ok_or("no FEN given")?.to_string() + "0 1 ";
        let g = Game::from_fen(&fen)?;
        let features = extract(&g);
        let score_str = split_line
            .next()
            .unwrap() //.ok_or("no result given")?
            .split('\"')
            .next()
            .unwrap(); // .ok_or("no result given")?;
        let score = match score_str {
            "1/2-1/2" => 0.5,
            "0-1" => 0.,
            "1-0" => 1.,
            _ => Err("unknown score string")?,
        };
        data.push((features, score));
    }

    Ok(data)
}

/// Extract a feature vector from a board.
fn extract(g: &Game) -> BoardFeatures {
    let player = g.meta().player;
    BoardFeatures(extract_side(g, player), extract_side(g, !player))
}

fn extract_side(g: &Game, player: Color) -> Box<[u16]> {
    let player_ksq = g.king_sq(player);
    let enemy = !player;

    Piece::NON_KING
        .into_iter()
        .flat_map(|pt| {
            (g.by_piece(pt) & g.by_color(player))
                .map(move |sq| input_idx(pt, player, player, player_ksq, sq))
        })
        .chain(Piece::ALL.into_iter().flat_map(|pt| {
            (g.by_piece(pt) & g.by_color(enemy))
                .map(move |sq| input_idx(pt, player, enemy, player_ksq, sq))
        }))
        .collect()
}

#[cfg(test)]
mod tests {
    use fiddler::base::game::Game;

    use crate::{extract, input_idx, BoardFeatures, Color::*, Piece::*, Square::*};

    #[test]
    fn extract_opening() {
        let g = Game::new();
        let BoardFeatures(ex0, ex1) = extract(&g);
        assert_eq!(
            ex0.as_ref(),
            &[
                input_idx(Knight, White, White, E1, B1),
                input_idx(Knight, White, White, E1, G1),
                input_idx(Bishop, White, White, E1, C1),
                input_idx(Bishop, White, White, E1, F1),
                input_idx(Rook, White, White, E1, A1),
                input_idx(Rook, White, White, E1, H1),
                input_idx(Queen, White, White, E1, D1),
                input_idx(Pawn, White, White, E1, A2),
                input_idx(Pawn, White, White, E1, B2),
                input_idx(Pawn, White, White, E1, C2),
                input_idx(Pawn, White, White, E1, D2),
                input_idx(Pawn, White, White, E1, E2),
                input_idx(Pawn, White, White, E1, F2),
                input_idx(Pawn, White, White, E1, G2),
                input_idx(Pawn, White, White, E1, H2),
                input_idx(Knight, White, Black, E1, B8),
                input_idx(Knight, White, Black, E1, G8),
                input_idx(Bishop, White, Black, E1, C8),
                input_idx(Bishop, White, Black, E1, F8),
                input_idx(Rook, White, Black, E1, A8),
                input_idx(Rook, White, Black, E1, H8),
                input_idx(Queen, White, Black, E1, D8),
                input_idx(Pawn, White, Black, E1, A7),
                input_idx(Pawn, White, Black, E1, B7),
                input_idx(Pawn, White, Black, E1, C7),
                input_idx(Pawn, White, Black, E1, D7),
                input_idx(Pawn, White, Black, E1, E7),
                input_idx(Pawn, White, Black, E1, F7),
                input_idx(Pawn, White, Black, E1, G7),
                input_idx(Pawn, White, Black, E1, H7),
                input_idx(King, White, Black, E1, E8),
            ]
        );

        assert_eq!(
            ex1.as_ref(),
            &[
                input_idx(Knight, Black, Black, E8, B8),
                input_idx(Knight, Black, Black, E8, G8),
                input_idx(Bishop, Black, Black, E8, C8),
                input_idx(Bishop, Black, Black, E8, F8),
                input_idx(Rook, Black, Black, E8, A8),
                input_idx(Rook, Black, Black, E8, H8),
                input_idx(Queen, Black, Black, E8, D8),
                input_idx(Pawn, Black, Black, E8, A7),
                input_idx(Pawn, Black, Black, E8, B7),
                input_idx(Pawn, Black, Black, E8, C7),
                input_idx(Pawn, Black, Black, E8, D7),
                input_idx(Pawn, Black, Black, E8, E7),
                input_idx(Pawn, Black, Black, E8, F7),
                input_idx(Pawn, Black, Black, E8, G7),
                input_idx(Pawn, Black, Black, E8, H7),
                input_idx(Knight, Black, White, E8, B1),
                input_idx(Knight, Black, White, E8, G1),
                input_idx(Bishop, Black, White, E8, C1),
                input_idx(Bishop, Black, White, E8, F1),
                input_idx(Rook, Black, White, E8, A1),
                input_idx(Rook, Black, White, E8, H1),
                input_idx(Queen, Black, White, E8, D1),
                input_idx(Pawn, Black, White, E8, A2),
                input_idx(Pawn, Black, White, E8, B2),
                input_idx(Pawn, Black, White, E8, C2),
                input_idx(Pawn, Black, White, E8, D2),
                input_idx(Pawn, Black, White, E8, E2),
                input_idx(Pawn, Black, White, E8, F2),
                input_idx(Pawn, Black, White, E8, G2),
                input_idx(Pawn, Black, White, E8, H2),
                input_idx(King, Black, White, E8, E1),
            ]
        );
    }
}
