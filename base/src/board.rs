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

//! State representations of boards, which contain board state (such as piece
//! positions), but neither history nor meta-information about a game.

use crate::movegen::{between, square_attackers, MAGIC};

use super::{zobrist, Bitboard, CastleRights, Color, Move, Piece, Square};

use std::{
    convert::TryFrom,
    default::Default,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
    ops::Index,
};

#[derive(Copy, Clone, Debug, Eq)]
/// A representation of a position. Does not handle the repetition or turn
/// timer.
pub struct Board {
    /// The squares ocupied by White and Black, respectively.
    sides: [Bitboard; 2],
    /// The squares occupied by (in order) knights, bishops, rooks,
    /// queens, pawns, and kings.
    pieces: [Bitboard; Piece::NUM_TYPES],
    /// The color of the player to move.
    pub player: Color,
    /// The square which can be moved to by a pawn in en passant. Will be
    /// `None` when a pawn has not moved two squares in the previous move.
    pub en_passant_square: Option<Square>,
    /// The rights of this piece for castling.
    pub castle_rights: CastleRights,

    /*
        Below: metadata which is not critical for board representation, but
        which is useful for performance.
    */
    /// A saved internal hash.
    /// If the board is valid, the this value must ALWAYS be equal to the output
    /// of `Board.get_fresh_hash()`.
    pub hash: u64,
    /// The set of squares which is occupied by pieces which are checking the
    /// king.
    pub checkers: Bitboard,
    /// The squares that the kings are living on.
    /// `king_sqs[0]` is the location of the white king, and
    /// `king_sqs[1]` is the location of the black king.
    pub king_sqs: [Square; 2],
    /// The set of squares containing pieces which are pinned, i.e. which are
    /// blocking some sort of attack on `player`'s king.
    pub pinned: Bitboard,
}

impl Board {
    /// Construct a `Board` from the standard chess starting position.
    pub fn new() -> Board {
        let mut board = Board {
            sides: [
                Bitboard::new(0x000000000000FFFF), //white
                Bitboard::new(0xFFFF000000000000), //black
            ],
            pieces: [
                Bitboard::new(0x4200000000000042), //knight
                Bitboard::new(0x2400000000000024), //bishop
                Bitboard::new(0x8100000000000081), //rook
                Bitboard::new(0x0800000000000008), //queen
                Bitboard::new(0x00FF00000000FF00), //pawn
                Bitboard::new(0x1000000000000010), //king
            ],
            en_passant_square: None,
            player: Color::White,
            castle_rights: CastleRights::ALL_RIGHTS,
            hash: 0,
            king_sqs: [Square::E1, Square::E8],
            checkers: Bitboard::EMPTY,
            pinned: Bitboard::EMPTY,
        };
        board.recompute_hash();
        board
    }

    /// Create a Board populated from some FEN and load it.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the FEN is invalid with a string describing why it
    /// failed.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler_base::Board;
    ///
    /// let default_board = Board::new();
    /// let fen_board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
    /// assert_eq!(default_board, fen_board);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_fen(fen: &str) -> Result<Board, &str> {
        let mut board = Board {
            sides: [Bitboard::EMPTY; 2],
            pieces: [Bitboard::EMPTY; 6],
            en_passant_square: None,
            player: Color::White,
            castle_rights: CastleRights::NO_RIGHTS,
            hash: 0,
            checkers: Bitboard::EMPTY,
            king_sqs: [Square::A1; 2],
            pinned: Bitboard::EMPTY,
        };
        let mut fen_chrs = fen.chars();
        let mut r = 7; //current row parsed
        let mut c = 0; //current col parsed

        loop {
            if (r, c) == (0, 8) {
                break;
            }
            let chr = fen_chrs
                .next()
                .ok_or("reached end of FEN before board was fully parsed")?;
            let is_white = chr.is_uppercase();
            let pt = chr.to_uppercase().next().and_then(Piece::from_code);
            let color = match is_white {
                true => Color::White,
                false => Color::Black,
            };
            if let Some(p) = pt {
                //character is a piece type
                board.add_piece(Square::new(r, c).unwrap(), p, color);
                c += 1;
            } else if chr == '/' {
                //row divider
                r -= 1;
                c = 0;
            } else {
                //number stating number of blank spaces in this row
                let num_blanks = chr.to_digit(10).ok_or("expected number of blanks")?;
                //advance the square under review by the number of blanks
                c += num_blanks as usize;
            }
        }

        //now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after board array section of FEN");
        };

        //now compute player to move
        let player_chr = fen_chrs
            .next()
            .ok_or("reached end of string while parsing for player to move")?;
        board.player = match player_chr {
            'w' => Color::White,
            'b' => Color::Black,
            _ => return Err("unrecognized player to move"),
        };

        //now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after player to move section of FEN");
        }

        //determine castle rights
        let mut castle_chr = fen_chrs
            .next()
            .ok_or("reached end of string while parsing castle rights")?;
        while castle_chr != ' ' {
            //this may accept some technically illegal FENS, but that's ok
            //note: hash was not updated, so will need to be rewritten by the
            //end of the function.
            board.castle_rights |= match castle_chr {
                'K' => CastleRights::king_castle(Color::White),
                'Q' => CastleRights::queen_castle(Color::White),
                'k' => CastleRights::king_castle(Color::Black),
                'q' => CastleRights::queen_castle(Color::Black),
                '-' => CastleRights::NO_RIGHTS,
                _ => return Err("unrecognized castle rights character"),
            };
            castle_chr = fen_chrs
                .next()
                .ok_or("reached end of string while parsing castle rights")?;
        }

        //castle rights searching ate the space, so no need to check for it

        //en passant square
        let ep_file_chr = fen_chrs
            .next()
            .ok_or("reached EOF while parsing en passant characters")?;
        if ep_file_chr != '-' {
            let ep_rank_chr = fen_chrs
                .next()
                .ok_or("reached end of string while parsing en passant rank")?;
            let mut s = String::from(ep_file_chr);
            s.push(ep_rank_chr);
            board.en_passant_square = Some(Square::from_algebraic(&s)?);
        }
        board.recompute_hash();
        board.king_sqs = [
            Square::try_from(board[Piece::King] & board[Color::White])?,
            Square::try_from(board[Piece::King] & board[Color::Black])?,
        ];
        board.checkers =
            square_attackers(&board, board.king_sqs[board.player as usize], !board.player);
        board.recompute_pinned();
        if !(board.is_valid()) {
            return Err("board state after loading was illegal");
        }
        //Ignore move clocks
        Ok(board)
    }

    #[inline(always)]
    /// Get the squares occupied by the pieces of each type (i.e. Black or
    /// White).
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Board, Bitboard};
    ///
    /// let board = Board::new();
    /// assert_eq!(board.occupancy(), Bitboard::new(0xFFFF00000000FFFF));
    /// ```
    pub fn occupancy(&self) -> Bitboard {
        self[Color::White] | self[Color::Black]
    }

    #[inline(always)]
    /// Get the type of the piece occupying a given square.
    /// Returns `None` if there are no pieces occupying the square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Board, Piece, Square};
    ///
    /// let board = Board::new();
    /// assert_eq!(board.type_at_square(Square::E1), Some(Piece::King));
    /// assert_eq!(board.type_at_square(Square::E4), None)
    /// ```
    pub fn type_at_square(&self, sq: Square) -> Option<Piece> {
        for pt in Piece::ALL_TYPES {
            if self[pt].contains(sq) {
                return Some(pt);
            }
        }
        None
    }

    #[inline(always)]
    /// Get the color of a piece occupying a current square.
    /// Returns `None` if there are no pieces occupying the square.
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler_base::{Board, Color, Square};
    ///
    /// let board = Board::new();
    /// assert_eq!(board.color_at_square(Square::E1), Some(Color::White));
    /// assert_eq!(board.color_at_square(Square::E4), None)
    /// ```
    pub fn color_at_square(&self, sq: Square) -> Option<Color> {
        let bb = Bitboard::from(sq);
        if !(self[Color::Black] & bb).is_empty() {
            return Some(Color::Black);
        }
        if !(self[Color::White] & bb).is_empty() {
            return Some(Color::White);
        }
        None
    }

    #[inline(always)]
    /// Is the given move a capture in the current state of the board? Requires
    /// that `m` is a legal move. En passant qualifies as a capture.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler_base::{Board, Move, Square};
    ///
    /// // Scandinavian defense. White can play exd5 to capture Black's pawn or
    /// // play e5 (among other moves).
    /// let board = Board::from_fen("rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 2")?;
    /// // exd5
    /// assert!(board.is_move_capture(Move::normal(Square::E4, Square::D5)));
    /// // e5
    /// assert!(!board.is_move_capture(Move::normal(Square::E4, Square::E5)));
    /// # Ok(())
    /// # }
    /// ```
    pub fn is_move_capture(&self, m: Move) -> bool {
        self.occupancy().contains(m.to_square()) || m.is_en_passant()
    }

    /// Check if the state of this board is valid.
    /// Returns false if the board is invalid.
    fn is_valid(&self) -> bool {
        let mut sides_checksum = Bitboard::EMPTY;
        let mut sides_checkor = Bitboard::EMPTY;
        let mut pieces_checksum = Bitboard::EMPTY;
        let mut pieces_checkor = Bitboard::EMPTY;
        for bb in self.sides {
            sides_checksum += bb;
            sides_checkor |= bb;
        }
        for bb in self.pieces {
            pieces_checksum += bb;
            pieces_checkor |= bb;
        }
        if sides_checksum != sides_checkor {
            return false;
        }
        if pieces_checksum != pieces_checkor {
            return false;
        }
        if sides_checksum != pieces_checksum {
            return false;
        }
        if self.hash != self.get_fresh_hash() {
            return false;
        }

        let w_king_bb = self[Piece::King] & self[Color::White];
        let b_king_bb = self[Piece::King] & self[Color::Black];

        if w_king_bb != Bitboard::from(self.king_sqs[Color::White as usize]) {
            return false;
        }

        if b_king_bb != Bitboard::from(self.king_sqs[Color::Black as usize]) {
            return false;
        }

        if self.checkers
            != square_attackers(self, self.king_sqs[self.player as usize], !self.player)
        {
            return false;
        }

        // TODO validate pinners
        true
    }

    /// Apply the given move to the board. Will assume the move is legal (unlike
    /// `try_move()`). Also requires that this board is currently valid.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler_base::{Board, Move, Square};
    ///
    /// let mut board = Board::new();
    /// // board after 1. e4 is played
    /// let board_after_e4 = Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1")?;
    ///
    /// board.make_move(Move::normal(Square::E2, Square::E4));
    /// assert_eq!(board, board_after_e4);
    /// # Ok(())
    /// # }
    /// ```
    pub fn make_move(&mut self, m: Move) {
        let from_sq = m.from_square();
        let to_sq = m.to_square();

        let player = self.player;
        let opponent = !player;
        //this length is used to determine whether it's not a move that a king
        //or pawn could normally make
        let is_long_move = from_sq.chebyshev_to(to_sq) > 1;
        // TODO figure out a way to remove the (slow) call to `type_at_square`?
        let mover_type = self.type_at_square(from_sq).unwrap();
        let is_pawn_move = mover_type == Piece::Pawn;
        let is_king_move = mover_type == Piece::King;

        /* Core move functionality */
        let capturee = self.type_at_square(to_sq);
        if let Some(c) = capturee {
            self.remove_known_piece(to_sq, c, opponent);
        }
        /* Promotion and normal piece movement */
        if let Some(p) = m.promote_type() {
            self.add_piece(to_sq, p, self.player);
        } else {
            self.add_piece(to_sq, mover_type, self.player);
        }
        self.remove_known_piece(from_sq, mover_type, player);

        /* En passant handling */
        //perform an en passant capture
        if m.is_en_passant() {
            let capturee_sq =
                Square::new(from_sq.rank(), self.en_passant_square.unwrap().file()).unwrap();
            self.remove_known_piece(capturee_sq, Piece::Pawn, opponent);
        }
        //remove previous EP square from hash
        self.hash ^= zobrist::ep_key(self.en_passant_square);
        //update EP square
        self.en_passant_square = match is_pawn_move && is_long_move {
            true => Square::new((from_sq.rank() + to_sq.rank()) / 2, from_sq.file()),
            false => None,
        };
        //insert new EP key into hash
        self.hash ^= zobrist::ep_key(self.en_passant_square);

        /* Handling castling and castle rights */
        //in normal castling, we describe it with a `Move` as a king move which
        //jumps two or three squares.

        let mut rights_to_remove;
        if is_king_move {
            rights_to_remove = CastleRights::color_rights(self.player);
            if is_long_move {
                //a long move from a king means this must be a castle
                //G file is file 6 (TODO move this to be a constant?)
                let is_kingside_castle = to_sq.file() == 6;
                let rook_from_file = match is_kingside_castle {
                    true => 7,  //rook moves from H file for kingside castling
                    false => 0, //rook moves from A file for queenside
                };
                let rook_to_file = match is_kingside_castle {
                    true => 5,  //rook moves to F file for kingside
                    false => 3, //rook moves to D file for queenside
                };
                let rook_from_sq = Square::new(from_sq.rank(), rook_from_file).unwrap();
                let rook_to_sq = Square::new(from_sq.rank(), rook_to_file).unwrap();
                self.remove_known_piece(rook_from_sq, Piece::Rook, player);
                self.add_piece(rook_to_sq, Piece::Rook, self.player);
            }
        } else {
            //don't need to check if it's a rook because moving from this square
            //would mean you didn't have the right anyway
            rights_to_remove = match from_sq {
                Square::A1 => CastleRights::queen_castle(Color::White),
                Square::H1 => CastleRights::king_castle(Color::White),
                Square::A8 => CastleRights::queen_castle(Color::Black),
                Square::H8 => CastleRights::king_castle(Color::Black),
                _ => CastleRights::NO_RIGHTS,
            };

            // capturing a rook also removes rights
            rights_to_remove |= match to_sq {
                Square::A1 => CastleRights::queen_castle(Color::White),
                Square::H1 => CastleRights::king_castle(Color::White),
                Square::A8 => CastleRights::queen_castle(Color::Black),
                Square::H8 => CastleRights::king_castle(Color::Black),
                _ => CastleRights::NO_RIGHTS,
            }
        }
        self.remove_castle_rights(rights_to_remove);

        /* Updating player to move */
        self.player = !self.player;
        self.hash ^= zobrist::BLACK_TO_MOVE_KEY;

        /* Non-meta fields of the board are now in their final state. */

        /* Update metadata */
        // king squares
        if is_king_move {
            // update king locations
            self.king_sqs[!self.player as usize] = m.to_square();
        }

        // checkers
        self.checkers = square_attackers(self, self.king_sqs[self.player as usize], !self.player);

        // pinned pieces
        self.recompute_pinned()
    }

    #[inline(always)]
    /// Remove a piece of a known type at a square.
    /// Will break the validity of the board if there is no piece of type `pt`
    /// and color `color` at `sq`.
    fn remove_known_piece(&mut self, sq: Square, pt: Piece, color: Color) {
        let mask = Bitboard::from(sq);
        self.hash ^= zobrist::square_key(sq, Some(pt), color);
        let removal_mask = !mask;
        self.pieces[pt as usize] &= removal_mask;
        self.sides[color as usize] &= removal_mask;
    }

    #[inline(always)]
    /// Add a piece to the square at a given place on the board.
    /// This should only be called if you believe that the board as-is is empty
    /// at the square below. Otherwise it will break the internal board
    /// representation.
    fn add_piece(&mut self, sq: Square, pt: Piece, color: Color) {
        //Remove the hash from the piece that was there before (no-op if it was
        //empty)
        let mask = Bitboard::from(sq);
        self.pieces[pt as usize] |= mask;
        self.sides[color as usize] |= mask;
        //Update the hash with the result of our addition
        self.hash ^= zobrist::square_key(sq, Some(pt), color);
    }

    /// Remove the given `CastleRights` from this board's castling rights, and
    /// update the internal hash of the board to match.
    fn remove_castle_rights(&mut self, rights_to_remove: CastleRights) {
        let rights_actually_removed = rights_to_remove & self.castle_rights;

        //TODO optimize this?
        for i in 0..4 {
            if 1 << i & rights_actually_removed.0 != 0 {
                self.hash ^= zobrist::get_castle_key(i);
            }
        }

        self.castle_rights &= !rights_actually_removed;
    }

    #[inline(always)]
    /// Recompute the Zobrist hash of this board and set it to the saved hash
    /// value.
    fn recompute_hash(&mut self) {
        self.hash = self.get_fresh_hash();
    }

    /// Recompute the `pinned` metadata of this board.
    fn recompute_pinned(&mut self) {
        self.pinned = Bitboard::EMPTY;
        let king_sq = self.king_sqs[self.player as usize];
        let rook_mask = MAGIC.rook_attacks(Bitboard::EMPTY, king_sq);
        let bishop_mask = MAGIC.bishop_attacks(Bitboard::EMPTY, king_sq);
        let occupancy = self.occupancy();

        let snipers = self[!self.player]
            & ((rook_mask & (self[Piece::Queen] | self[Piece::Rook]))
                | (bishop_mask & (self[Piece::Queen] | self[Piece::Bishop])));

        for sniper_sq in snipers {
            let between_bb = between(king_sq, sniper_sq);
            if (between_bb & occupancy).has_single_bit() {
                self.pinned |= between_bb;
            }
        }
    }

    /// Determine whether this board represents a game which is over due to
    /// insufficient material.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler_base::Board;
    ///
    /// // Same-color bishops on a KBKB endgame is a draw by insufficient
    /// // material in FIDE rules.
    /// let board = Board::from_fen("8/8/3k4/8/4b3/2KB4/8/8 w - - 0 1")?;
    /// assert!(board.insufficient_material());
    /// # Ok(())
    /// # }
    /// ```
    pub fn insufficient_material(&self) -> bool {
        const DARK_SQUARES: Bitboard = Bitboard::new(0xAA55AA55AA55AA55);
        match self.occupancy().len() {
            0 | 1 => unreachable!(), // a king is missing
            2 => true,               // only two kings
            3 => !(self[Piece::Knight] | self[Piece::Bishop]).is_empty(), //KNK or KBK
            // same colored bishops
            4 => {
                self[Piece::Bishop].more_than_one()
                    && !(self[Piece::Bishop] & DARK_SQUARES).has_single_bit()
            }
            _ => false,
        }
    }

    /// Compute the hash value of this board from scratch. This should
    /// generally only be used for debug purposes, as in most cases iteratively
    /// updating the hashes as moves are made is enough.
    fn get_fresh_hash(&self) -> u64 {
        let mut hash = 0;
        for i in 0..64 {
            let sq = Square::try_from(i).unwrap();
            hash ^= match self.color_at_square(sq) {
                Some(c) => zobrist::square_key(sq, self.type_at_square(sq), c),
                None => 0,
            };
        }
        for i in 0..4 {
            if 1 << i & self.castle_rights.0 != 0 {
                hash ^= zobrist::get_castle_key(i);
            }
        }
        hash ^= zobrist::ep_key(self.en_passant_square);
        hash ^= zobrist::player_key(self.player);
        hash
    }
}

impl Display for Board {
    /// Display this board in a console-ready format. Expresses as a series of 8
    /// lines, where the topmost line is the 8th rank and the bottommost is the
    /// 1st. White pieces are represented with capital letters, while black
    /// pieces have lowercase.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square::try_from(i).unwrap();
                match self.type_at_square(current_square) {
                    Some(p) => match self.color_at_square(current_square).unwrap() {
                        Color::White => write!(f, "{p}")?,
                        Color::Black => write!(f, "{}", p.code().to_lowercase())?,
                    },
                    None => write!(f, ".")?,
                }
                write!(f, " ")?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

impl PartialEq for Board {
    fn eq(&self, other: &Board) -> bool {
        self.sides == other.sides
            && self.pieces == other.pieces
            && self.en_passant_square == other.en_passant_square
            && self.player == other.player
            && self.castle_rights == other.castle_rights
    }
}

impl Index<Piece> for Board {
    type Output = Bitboard;

    #[inline(always)]
    /// Get the squares occupied by the given piece.
    fn index(&self, index: Piece) -> &Self::Output {
        // SAFETY: This will not fail because there are the same number of
        // pieces as legal indices on `pieces`.
        unsafe { self.pieces.get_unchecked(index as usize) }
    }
}

impl Index<Color> for Board {
    type Output = Bitboard;

    #[inline(always)]
    /// Get the squares occupied by the given piece.
    fn index(&self, index: Color) -> &Self::Output {
        // SAFETY: This will not fail because there are the same number of
        // colors as indices on `sides`.
        unsafe { self.sides.get_unchecked(index as usize) }
    }
}

impl Default for Board {
    fn default() -> Board {
        Board::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::square::*;

    /// A board with the white king on A1 and the black king on H8.
    const TWO_KINGS_BOARD: Board = Board {
        sides: [
            Bitboard::new(0x0000000000000001), //white
            Bitboard::new(0x8000000000000000), //black
        ],
        pieces: [
            Bitboard::new(0x0000000000000000), //pawn
            Bitboard::new(0x0000000000000000), //knight
            Bitboard::new(0x0000000000000000), //bishop
            Bitboard::new(0x0000000000000000), //rook
            Bitboard::new(0x0000000000000000), //queen
            Bitboard::new(0x8000000000000001), //king
        ],
        en_passant_square: None,
        player: Color::White,
        castle_rights: CastleRights::NO_RIGHTS,
        hash: 3483926298739092744,
        checkers: Bitboard::EMPTY,
        king_sqs: [Square::A1, Square::H8],
        pinned: Bitboard::EMPTY,
    };

    #[test]
    /// Test that a chessboard with kings on A1 and H8 can be loaded from a FEN.
    fn load_two_kings_fen() {
        let result = Board::from_fen("7k/8/8/8/8/8/8/K7 w - - 0 1");
        assert_eq!(result, Ok(TWO_KINGS_BOARD));
    }

    #[test]
    /// Test that the start position of a normal chess game can be loaded from
    /// its FEN.
    fn start_fen() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        assert_eq!(result, Ok(Board::default()));
    }

    #[test]
    /// Test that we can play e4 on the first move of the game.
    fn play_e4() {
        move_helper(Board::default(), Move::normal(Square::E2, Square::E4));
    }

    #[test]
    /// Test that a board with an en passant square can be loaded from a FEN
    /// correctly.
    fn load_en_passant() {
        // exf6 is en passant here
        let b = Board::from_fen("rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3")
            .unwrap();
        assert_eq!(b.en_passant_square, Some(Square::F6));
    }

    #[test]
    /// Test that we can capture en passant.
    fn en_passant() {
        // exf6 is en passant here
        fen_helper(
            "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
            Move::normal(Square::E5, Square::F6),
        );
    }

    #[test]
    /// Test that White can castle kingside.
    fn white_kingide_castle() {
        fen_helper(
            "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            Move::normal(Square::E1, Square::G1),
        );
    }

    #[test]
    /// Test that White can promote their pawn to a queen
    fn white_promote_queen() {
        // f7 pawn can promote
        fen_helper(
            "8/5P2/2k5/4K3/8/8/8/8 w - - 0 1",
            Move::promoting(Square::F7, Square::F8, Piece::Queen),
        );
    }

    #[test]
    /// Test that capturing a rook removes the right to castle with that rook.
    fn no_castle_after_capture() {
        let m = Move::normal(Square::B2, Square::H8);
        // capturing the rook on h8 prevents castle rights
        fen_helper(
            "rnbqk2r/ppppnp1p/4p1pb/8/4P3/1P1P4/PBP2PPP/RN1QKBNR w KQkq - 1 5",
            m,
        );
    }

    /// A helper function which will load a board from a FEN and then try
    /// running the given move on that board.
    pub fn fen_helper(fen: &str, m: Move) {
        let result = Board::from_fen(fen);
        match result {
            Ok(board) => move_helper(board, m),
            Err(_) => panic!("could not load FEN"),
        };
    }

    /// A helper function which will attempt to make a legal move on a board,

    /// and will fail assertions if the board's state was not changed correctly.
    pub fn move_helper(board: Board, m: Move) {
        //new_board will be mutated to reflect the move
        let mut new_board = board;
        new_board.make_move(m);
        move_result_helper(board, new_board, m);
    }

    /// Test that `new_board` was created by playing the move `m` on
    /// `old_board`. Fails assertion if this is not the case.
    pub fn move_result_helper(old_board: Board, new_board: Board, m: Move) {
        let mover_color = old_board.color_at_square(m.from_square()).unwrap();
        let mover_type = old_board.type_at_square(m.from_square()).unwrap();

        assert!(new_board.is_valid());

        if m.is_promotion() {
            assert_eq!(new_board.type_at_square(m.to_square()), m.promote_type());
        } else {
            assert_eq!(new_board.type_at_square(m.to_square()), Some(mover_type));
        }
        assert_eq!(new_board.color_at_square(m.to_square()), Some(mover_color));

        assert_eq!(new_board.type_at_square(m.from_square()), None);
        assert_eq!(new_board.color_at_square(m.from_square()), None);

        //Check en passant worked correctly
        if m.is_en_passant() {
            assert_eq!(
                new_board.type_at_square(old_board.en_passant_square.unwrap()),
                Some(Piece::Pawn)
            );
            assert_eq!(
                new_board.color_at_square(old_board.en_passant_square.unwrap()),
                Some(old_board.player)
            );
        }

        //Check castling worked correctly
        if m.is_castle() {
            let rook_start_file = match m.to_square().file() {
                2 => 0,
                6 => 7,
                _ => 9,
            };
            let rook_end_file = match m.to_square().file() {
                2 => 3,
                6 => 5,
                _ => 9,
            };
            let rook_start_sq = Square::new(m.from_square().rank(), rook_start_file).unwrap();
            let rook_end_sq = Square::new(m.from_square().rank(), rook_end_file).unwrap();

            assert_eq!(new_board.type_at_square(rook_start_sq), None);
            assert_eq!(new_board.color_at_square(rook_start_sq), None);

            assert_eq!(new_board.type_at_square(rook_end_sq), Some(Piece::Rook));
            assert_eq!(
                new_board.color_at_square(rook_end_sq),
                Some(old_board.player)
            );

            assert!(!new_board
                .castle_rights
                .is_kingside_castle_legal(mover_color));
            assert!(!new_board
                .castle_rights
                .is_queenside_castle_legal(mover_color));
        }

        // Check castling rights were removed correctly
        if mover_type == Piece::Rook {
            match m.from_square() {
                Square::A1 => assert!(!new_board
                    .castle_rights
                    .is_queenside_castle_legal(Color::White)),
                Square::A8 => assert!(!new_board
                    .castle_rights
                    .is_kingside_castle_legal(Color::White)),
                Square::H1 => assert!(!new_board
                    .castle_rights
                    .is_queenside_castle_legal(Color::Black)),
                Square::H8 => assert!(!new_board
                    .castle_rights
                    .is_kingside_castle_legal(Color::Black)),
                _ => {}
            };
        }

        match m.to_square() {
            Square::A1 => assert!(!new_board
                .castle_rights
                .is_queenside_castle_legal(Color::White)),
            Square::A8 => assert!(!new_board
                .castle_rights
                .is_kingside_castle_legal(Color::White)),
            Square::H1 => assert!(!new_board
                .castle_rights
                .is_queenside_castle_legal(Color::Black)),
            Square::H8 => assert!(!new_board
                .castle_rights
                .is_kingside_castle_legal(Color::Black)),
            _ => {}
        };
    }
}
