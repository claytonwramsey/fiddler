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

//! Full chess games, including history and metadata.

use super::{
    castling::CastleRights,
    movegen::{is_legal, square_attackers, MAGIC, PAWN_ATTACKS},
    zobrist, Bitboard, Color, Move, Piece, Square,
};

use std::{
    default::Default,
    fmt::{Display, Formatter},
    ops::Index,
};

#[allow(clippy::module_name_repetitions)]
#[derive(Clone, Debug, Eq, PartialEq)]
/// A struct containing game information, which unlike a [`Board`], knows about its history and can
/// do things like repetition detection.
pub struct Game {
    /// The current state of the board.
    board: Board,
    /// The list, in order, of all board metadata made in the game.
    history: Vec<BoardMeta>,
    /// The list, in order, of all moves made in the game and the pieces that they captured.
    /// They should all be valid moves.
    /// If the move played is en passant, the capturee type is still `None` because the piece that
    /// is replaced on undo is not on the move's from-square.
    /// The length of `moves` should always be one less than the length of `history`.
    moves: Vec<(Move, Option<Piece>)>,
}
#[derive(Clone, Copy, Debug, Eq)]
/// A representation of a position. Does not handle repetition of moves.
pub struct Board {
    /// A mailbox representation of the state of the board.
    /// Each index corresponds to a square, starting with square A1 at index 0.
    mailbox: [Option<(Piece, Color)>; 64],
    /// The squares ocupied by White and Black, respectively.
    sides: [Bitboard; 2],
    /// The squares occupied by (in order) knights, bishops, rooks,
    /// queens, pawns, and kings.
    pieces: [Bitboard; Piece::NUM],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// The metadata for a board state, containing extra information beyond the locations of pieces.
pub struct BoardMeta {
    /// The color of the player to move.
    pub player: Color,
    /// The square which can be moved to by a pawn in en passant.
    /// Will be `None` when a pawn has not moved two squares in the previous
    /// move.
    pub en_passant_square: Option<Square>,
    /// The rights of this piece for castling.
    pub castle_rights: CastleRights,
    /// The number of plies that have passed since a capture or pawn push has been made.
    pub rule50: u8,

    /*
        Below: metadata which is not critical for board representation, but
        which is useful for performance.
    */
    /// A saved internal hash.
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

impl Game {
    #[must_use]
    /// Construct a new [`Game`] in the conventional chess starting position.
    pub fn new() -> Game {
        let board = Board {
            sides: [
                Bitboard::new(0x0000_0000_0000_FFFF), // white
                Bitboard::new(0xFFFF_0000_0000_0000), // black
            ],
            pieces: [
                Bitboard::new(0x4200_0000_0000_0042), // knight
                Bitboard::new(0x2400_0000_0000_0024), // bishop
                Bitboard::new(0x8100_0000_0000_0081), // rook
                Bitboard::new(0x0800_0000_0000_0008), // queen
                Bitboard::new(0x00FF_0000_0000_FF00), // pawn
                Bitboard::new(0x1000_0000_0000_0010), // king
            ],
            mailbox: [
                Some((Piece::Rook, Color::White)), // a1
                Some((Piece::Knight, Color::White)),
                Some((Piece::Bishop, Color::White)),
                Some((Piece::Queen, Color::White)),
                Some((Piece::King, Color::White)),
                Some((Piece::Bishop, Color::White)),
                Some((Piece::Knight, Color::White)),
                Some((Piece::Rook, Color::White)),
                Some((Piece::Pawn, Color::White)), // a2
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                Some((Piece::Pawn, Color::White)),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None, // rank 3
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None, // rank 4
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None, // rank 5
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,                              // rank 6
                Some((Piece::Pawn, Color::Black)), // a7
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Pawn, Color::Black)),
                Some((Piece::Rook, Color::Black)), // a8
                Some((Piece::Knight, Color::Black)),
                Some((Piece::Bishop, Color::Black)),
                Some((Piece::Queen, Color::Black)),
                Some((Piece::King, Color::Black)),
                Some((Piece::Bishop, Color::Black)),
                Some((Piece::Knight, Color::Black)),
                Some((Piece::Rook, Color::Black)),
            ],
        };
        Game {
            board,
            history: vec![BoardMeta {
                en_passant_square: None,
                player: Color::White,
                castle_rights: CastleRights::ALL,
                rule50: 0,
                hash: Bitboard::ALL
                    .into_iter()
                    .filter_map(|sq| {
                        board.mailbox[sq as usize]
                            .map(|(pt, color)| zobrist::square_key(sq, pt, color))
                    })
                    .chain((0..4).map(zobrist::castle_key))
                    .fold(0, |a, b| a ^ b),
                king_sqs: [Square::E1, Square::E8],
                checkers: Bitboard::EMPTY,
                pinned: Bitboard::EMPTY,
            }],
            moves: vec![],
        }
    }

    #[allow(clippy::missing_panics_doc)]
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
    /// use fiddler::base::Board;
    ///
    /// let default_board = Board::new();
    /// let fen_board = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1")?;
    /// assert_eq!(default_board, fen_board);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_fen(fen: &str) -> Result<Game, &str> {
        let mut game = Game {
            board: Board {
                sides: [Bitboard::EMPTY; 2],
                pieces: [Bitboard::EMPTY; 6],
                mailbox: [None; 64],
            },
            history: vec![BoardMeta {
                en_passant_square: None,
                player: Color::White,
                castle_rights: CastleRights::NONE,
                rule50: 0,
                hash: 0,
                checkers: Bitboard::EMPTY,
                king_sqs: [Square::A1; 2],
                pinned: Bitboard::EMPTY,
            }],
            moves: Vec::new(),
        };

        let mut fen_chrs = fen.chars();
        let mut r = 7; // current row parsed
        let mut c = 0; // current col parsed

        while r != 0 || c < 8 {
            let chr = fen_chrs
                .next()
                .ok_or("reached end of FEN before board was fully parsed")?;
            let color = if chr.is_uppercase() {
                Color::White
            } else {
                Color::Black
            };
            let pt = chr.to_uppercase().next().and_then(Piece::from_code);
            if let Some(p) = pt {
                //character is a piece type
                game.add_piece(
                    Square::new(r, c).ok_or("invalid structure of FEN")?,
                    p,
                    color,
                );
                c += 1;
            } else if chr == '/' {
                //row divider
                r -= 1;
                c = 0;
            } else {
                // number stating number of blank spaces in this row
                let num_blanks = chr.to_digit(10).ok_or("expected number of blanks")?;
                // advance the square under review by the number of blanks
                #[allow(clippy::cast_possible_truncation)]
                {
                    c += num_blanks as u8;
                }
            }
        }

        // now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after board array section of FEN");
        };

        let meta = game.history.last_mut().unwrap();

        // now compute player to move
        meta.player = {
            let player_chr = fen_chrs
                .next()
                .ok_or("reached end of string while parsing for player to move")?;
            match player_chr {
                'w' => Color::White,
                'b' => Color::Black,
                _ => return Err("unrecognized player to move"),
            }
        };

        if meta.player == Color::Black {
            meta.hash ^= zobrist::BLACK_TO_MOVE_KEY;
        }

        // now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after player to move section of FEN");
        }

        // determine castle rights
        let mut castle_chr = fen_chrs
            .next()
            .ok_or("reached end of string while parsing castle rights")?;
        while castle_chr != ' ' {
            // this may accept some technically illegal FENS, but that's ok
            let (rights_to_add, key_to_add) = match castle_chr {
                'K' => (CastleRights::WHITE_KINGSIDE, zobrist::castle_key(0)),
                'Q' => (CastleRights::WHITE_QUEENSIDE, zobrist::castle_key(1)),
                'k' => (CastleRights::BLACK_KINGSIDE, zobrist::castle_key(2)),
                'q' => (CastleRights::BLACK_QUEENSIDE, zobrist::castle_key(3)),
                '-' => (CastleRights::NONE, 0),
                _ => return Err("unrecognized castle rights character"),
            };
            meta.castle_rights |= rights_to_add;
            meta.hash ^= key_to_add;
            castle_chr = fen_chrs
                .next()
                .ok_or("reached end of string while parsing castle rights")?;
        }

        // castle rights searching ate the space, so no need to check for it

        // en passant square
        meta.en_passant_square = {
            let ep_file_chr = fen_chrs
                .next()
                .ok_or("reached EOF while parsing en passant characters")?;
            if ep_file_chr == '-' {
                None
            } else {
                let ep_rank_chr = fen_chrs
                    .next()
                    .ok_or("reached end of string while parsing en passant rank")?;
                let ep_sq = Square::from_algebraic(&format!("{ep_file_chr}{ep_rank_chr}"))?;
                meta.hash ^= zobrist::ep_key(ep_sq);
                Some(ep_sq)
            }
        };

        // now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after en passant square section of FEN");
        }

        // 50 move timer
        meta.rule50 = {
            let mut rule50_buf = String::new();
            // there may be more digits
            loop {
                match fen_chrs.next() {
                    Some(' ') => break,
                    Some(c) if c.is_ascii_digit() => rule50_buf.push(c),
                    Some(_) => return Err("illegal character for rule50 counter"),
                    None => return Err("reached end of string while parsing rule 50"),
                };
            }

            let rule50_num = rule50_buf
                .parse::<u8>()
                .map_err(|_| "could not parse rule50 counter")?;
            if rule50_num > 100 {
                return Err("rule 50 number is too high");
            }

            rule50_num
        };

        // updating metadata
        meta.king_sqs = [
            Square::try_from(game.board[Piece::King] & game.board[Color::White])?,
            Square::try_from(game.board[Piece::King] & game.board[Color::Black])?,
        ];
        game.history[0].checkers = square_attackers(
            &game.board,
            meta.king_sqs[meta.player as usize],
            !meta.player,
        );
        game.history[0].pinned = game.compute_pinned(
            game.history[0].king_sqs[game.history[0].player as usize],
            !game.history[0].player,
        );
        if !game.board.is_valid() {
            return Err("board state after loading was illegal");
        }

        Ok(game)
    }

    #[must_use]
    /// Get the position representing the current state of the game.
    pub fn board(&self) -> &Board {
        &self.board
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    /// Get the metadata associated with the current board state.
    pub fn meta(&self) -> &BoardMeta {
        self.history.last().unwrap()
    }

    /// Apply the given move to the board.
    /// Will assume the move is legal.
    /// Requires that this board is currently valid.
    ///
    /// # Panics
    ///
    /// This function may or may not panic if `m` is not a legal move.
    /// However, you can trust that it will never panic if `m` is legal.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler::base::{Board, Move, Square};
    ///
    /// let mut board = Board::new();
    /// // board after 1. e4 is played
    /// let board_after_e4 =
    ///     Board::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1")?;
    ///
    /// board.make_move(Move::normal(Square::E2, Square::E4));
    /// assert_eq!(board, board_after_e4);
    /// # Ok(())
    /// # }
    /// ```
    pub fn make_move(&mut self, m: Move) {
        /* -------- Check move legality in debug builds --------- */
        #[cfg(debug_assertions)]
        if !is_legal(m, self) {
            println!("an illegal move {m} is being attempted. History: {self}");
            panic!("Illegal move attempted on `Game::make_move`");
        }
        let from_sq = m.from_square();
        let to_sq = m.to_square();

        let mover_type = self.board[from_sq].unwrap().0;
        let player = self.meta().player;
        let ep_sq = self.meta().en_passant_square;
        let old_castle_rights = self.meta().castle_rights;
        let is_pawn_move = mover_type == Piece::Pawn;
        let is_king_move = mover_type == Piece::King;
        let capturee = self.board[to_sq];
        // hash key of new position

        let mut new_meta = BoardMeta {
            player: !self.meta().player,
            rule50: if is_pawn_move || capturee.is_some() {
                0
            } else {
                self.meta().rule50 + 1
            },
            hash: self.meta().hash
                ^ zobrist::BLACK_TO_MOVE_KEY
                ^ zobrist::square_key(from_sq, mover_type, player),
            ..*self.meta()
        };

        /* -------- Core move functionality -------- */
        /* Promotion and normal piece movement */
        self.add_piece(to_sq, m.promote_type().unwrap_or(mover_type), player);
        self.remove_piece(from_sq);

        if let Some((capturee_type, _)) = capturee {
            self.remove_piece(to_sq);
            new_meta.hash ^= zobrist::square_key(to_sq, capturee_type, !player);
        }

        /* -------- En passant handling -------- */
        // perform an en passant capture

        if m.is_en_passant() {
            let capturee_sq = Square::new(from_sq.rank(), ep_sq.unwrap().file()).unwrap();
            self.remove_piece(capturee_sq);
            new_meta.hash ^= zobrist::square_key(capturee_sq, Piece::Pawn, !player);
        }

        // remove previous EP square from hash
        if let Some(sq) = ep_sq {
            new_meta.hash ^= zobrist::ep_key(sq);
        }

        // update EP square
        if is_pawn_move && from_sq.rank_distance(to_sq) > 1 {
            let ep_candidate =
                Square::new((from_sq.rank() + to_sq.rank()) / 2, from_sq.file()).unwrap();
            if (PAWN_ATTACKS[player as usize][ep_candidate as usize]
                & self.board[Piece::Pawn]
                & self.board[!player])
                .is_empty()
            {
                new_meta.en_passant_square = None;
            } else {
                new_meta.en_passant_square = Some(ep_candidate);
                new_meta.hash ^= zobrist::ep_key(ep_candidate);
            }
        } else {
            new_meta.en_passant_square = None;
        };

        /* -------- Handling castling and castle rights -------- */
        // in normal castling, we describe it with a `Move` as a king move which jumps two or three
        // squares.

        let mut rights_to_remove;
        if is_king_move {
            rights_to_remove = match player {
                Color::White => CastleRights::WHITE,
                Color::Black => CastleRights::BLACK,
            };
            if from_sq.file_distance(to_sq) > 1 {
                // a long move from a king means this must be a castle
                // G file is file 6
                let is_kingside_castle = to_sq.file() == 6;
                let (rook_from_file, rook_to_file) = if is_kingside_castle {
                    (7, 5) // rook moves from H file for kingside castling
                } else {
                    (0, 3) // rook moves from A to D for queenside caslting
                };
                let rook_from_sq = Square::new(from_sq.rank(), rook_from_file).unwrap();
                let rook_to_sq = Square::new(from_sq.rank(), rook_to_file).unwrap();
                self.remove_piece(rook_from_sq);
                new_meta.hash ^= zobrist::square_key(rook_from_sq, Piece::Rook, player);
                self.add_piece(rook_to_sq, Piece::Rook, player);
            }
        } else {
            // don't need to check if it's a rook because moving from this square
            // would mean you didn't have the right anyway
            rights_to_remove = match from_sq {
                Square::A1 => CastleRights::WHITE_QUEENSIDE,
                Square::H1 => CastleRights::WHITE_KINGSIDE,
                Square::A8 => CastleRights::BLACK_QUEENSIDE,
                Square::H8 => CastleRights::BLACK_KINGSIDE,
                _ => CastleRights::NONE,
            };

            // capturing a rook also removes rights
            rights_to_remove |= match to_sq {
                Square::A1 => CastleRights::WHITE_QUEENSIDE,
                Square::H1 => CastleRights::WHITE_KINGSIDE,
                Square::A8 => CastleRights::BLACK_QUEENSIDE,
                Square::H8 => CastleRights::BLACK_KINGSIDE,
                _ => CastleRights::NONE,
            }
        }

        let mut rights_actually_removed = rights_to_remove & old_castle_rights;
        new_meta.castle_rights ^= rights_actually_removed;

        #[allow(clippy::cast_possible_truncation)]
        while rights_actually_removed.0 != 0 {
            new_meta.hash ^= zobrist::castle_key(rights_actually_removed.0.trailing_zeros() as u8);
            rights_actually_removed &= CastleRights(rights_actually_removed.0 - 1);
        }

        /* -------- Non-meta fields of the board are now in their final state. -------- */

        /* -------- Update other metadata -------- */

        // checkers
        new_meta.checkers =
            square_attackers(&self.board, new_meta.king_sqs[!player as usize], player);

        // pinned pieces
        new_meta.pinned = self.compute_pinned(new_meta.king_sqs[!player as usize], player);
        self.history.push(new_meta);
        self.moves.push((m, capturee.map(|c| c.0)));
    }

    /// Remove a piece from a square, assuming that `sq` is occupied.
    ///
    /// # Panics
    ///
    /// This operation will panic if `sq` is empty.
    fn remove_piece(&mut self, sq: Square) {
        let mask = !Bitboard::from(sq);
        let (pt, color) = self.board.mailbox[sq as usize].unwrap();
        self.board.pieces[pt as usize] &= mask;
        self.board.sides[color as usize] &= mask;
        self.board.mailbox[sq as usize] = None;
    }

    /// Add a piece to the square at a given place on the board.
    fn add_piece(&mut self, sq: Square, pt: Piece, color: Color) {
        // Remove the hash from the piece that was there before (no-op if it was
        // empty)
        let mask = Bitboard::from(sq);
        self.board.pieces[pt as usize] |= mask;
        self.board.sides[color as usize] |= mask;
        self.board.mailbox[sq as usize] = Some((pt, color));
        // Update the hash with the result of our addition
        self.history.last_mut().unwrap().hash ^= zobrist::square_key(sq, pt, color);
    }

    /// Compute a bitboard of all pieces pinned to square `pin_sq` by attacks from color `enemy`.
    fn compute_pinned(&self, pin_sq: Square, enemy: Color) -> Bitboard {
        let mut pinned = Bitboard::EMPTY;
        let rook_mask = MAGIC.rook_attacks(Bitboard::EMPTY, pin_sq);
        let bishop_mask = MAGIC.bishop_attacks(Bitboard::EMPTY, pin_sq);
        let occupancy = self.board.occupancy();
        let queens = self.board[Piece::Queen];

        let snipers = self.board[enemy]
            & ((rook_mask & (queens | self.board[Piece::Rook]))
                | (bishop_mask & (queens | self.board[Piece::Bishop])));

        for sniper_sq in snipers {
            let between_bb = Bitboard::between(pin_sq, sniper_sq);
            if (between_bb & occupancy).has_single_bit() {
                pinned |= between_bb;
            }
        }

        pinned
    }

    /// Empty out the history of this game completely, but leave the original start state of the
    /// board.
    /// Will also end the searching period for the game.
    pub fn clear(&mut self) {
        for _ in 0..self.moves.len() {
            let _ = self.undo();
        }
    }

    #[allow(clippy::result_unit_err)]
    /// Attempt to play a move, which may or may not be legal.
    /// Will return `Ok(())` if `m` was a legal move.
    ///
    /// # Errors
    ///
    /// This function will return an `Err(())` if the move is illegal.
    pub fn try_move(&mut self, m: Move) -> Result<(), ()> {
        if is_legal(m, self) {
            self.make_move(m);
            Ok(())
        } else {
            Err(())
        }
    }

    #[allow(clippy::missing_panics_doc)]
    /// Undo the most recent move.
    /// This function will return `Ok()` if there was history to undo.
    ///
    /// # Errors
    ///
    /// This function will return an `Err` if the history of this game has no more positions left
    /// to undo.
    pub fn undo(&mut self) -> Result<(), &'static str> {
        let (m, capturee_type) = self.moves.pop().ok_or("no history to undo")?;
        self.history.pop().unwrap();

        let from_sq = m.from_square();
        let to_sq = m.to_square();

        let (pt, color) = self.board[to_sq].unwrap();

        // note: we don't need to update hashes here because that was saved in the history

        // return the original piece to its from-square
        self.add_piece(from_sq, pt, color);

        if let Some(c_pt) = capturee_type {
            // undo capture by putting the capturee back
            self.add_piece(to_sq, c_pt, !color);
        } else {
            self.remove_piece(to_sq);

            if m.is_castle() {
                // replace rook
                let replacement_rook_sq = match (color, to_sq.file()) {
                    (Color::White, 2) => Square::A1,
                    (Color::White, 6) => Square::H1,
                    (Color::Black, 2) => Square::A8,
                    (Color::Black, 6) => Square::H8,
                    _ => unreachable!("undo castle to bad square"),
                };
                self.add_piece(replacement_rook_sq, Piece::Rook, color);
            } else if m.is_en_passant() {
                // replace captured pawn by en passant
                self.add_piece(
                    self.history.last().unwrap().en_passant_square.unwrap(),
                    Piece::Pawn,
                    !color,
                );
            }
        }

        Ok(())
    }

    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    /// Determine whether there has been a repetition in the last `moves_since_start` moves, or
    /// until the most recent pawn move or capture, whichever comes first.
    pub fn repetition_since(&self, moves_since_start: usize) -> bool {
        let mut hist_iter = self.history.iter().rev().take(moves_since_start + 1);
        let latest_hash = hist_iter.next().unwrap().hash;
        for meta in hist_iter {
            if latest_hash == meta.hash {
                return true;
            } else if meta.rule50 == 0 {
                return false;
            }
        }
        false
    }

    #[allow(clippy::len_without_is_empty)]
    #[must_use]
    /// Get the number of total positions in this history of this game.
    pub fn len(&self) -> usize {
        self.history.len()
    }
}

impl Board {
    #[must_use]
    /// Get the squares occupied by the pieces of each type (i.e. Black or
    /// White).
    ///
    /// # Examples
    ///
    /// ```
    /// use fiddler::base::{Bitboard, Board};
    ///
    /// let board = Board::new();
    /// assert_eq!(board.occupancy(), Bitboard::new(0xFFFF00000000FFFF));
    /// ```
    pub fn occupancy(&self) -> Bitboard {
        self[Color::White] | self[Color::Black]
    }

    #[must_use]
    /// Is the given move a capture in the current state of the board? Requires
    /// that `m` is a legal move. En passant qualifies as a capture.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler::base::{Board, Move, Square};
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
        m.is_en_passant() || self[m.to_square()].is_some()
    }

    #[must_use]
    /// Check if the state of this board is valid.
    ///
    /// Returns false if the board is invalid.
    pub fn is_valid(&self) -> bool {
        Bitboard::ALL
            .into_iter()
            .all(|sq| match self.mailbox[sq as usize] {
                Some((pt, color)) => {
                    self.sides[color as usize].contains(sq)
                        && !self.sides[!color as usize].contains(sq)
                        && Piece::ALL
                            .into_iter()
                            .all(|pt2| (pt2 == pt) == self.pieces[pt2 as usize].contains(sq))
                }
                None => self
                    .sides
                    .iter()
                    .chain(self.pieces.iter())
                    .all(|bb| !bb.contains(sq)),
            })
    }

    #[must_use]
    /// Determine whether this `Board`'s position is drawn by insufficient material.
    ///
    /// # Examples
    ///
    /// ```
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// use fiddler::base::Board;
    ///
    /// // Start position of the game is not a draw.
    /// let (board0, _) = Board::new();
    /// assert!(!board0.insufficient_material());
    ///
    /// // Same-color bishops on a KBKB endgame is a draw by insufficient material in FIDE rules.
    /// let (board1, _) = Board::from_fen("8/8/3k4/8/4b3/2KB4/8/8 w - - 0 1")?;
    /// assert!(board1.insufficient_material());
    /// # Ok(())
    /// # }
    /// ```
    pub fn insufficient_material(&self) -> bool {
        /// The set of dark squares, i.e. A1 and those on its diagonal.
        const DARK_SQUARES: Bitboard = Bitboard::new(0xAA55_AA55_AA55_AA55);
        match self.occupancy().len() {
            0 | 1 => unreachable!(), // a king is missing
            2 => true,               // only two kings
            3 => !(self[Piece::Knight] | self[Piece::Bishop]).is_empty(), // KNK or KBK
            // same colored bishops
            4 => {
                self[Piece::Bishop].more_than_one()
                    && !(self[Piece::Bishop] & DARK_SQUARES).has_single_bit()
            }
            _ => false,
        }
    }
}

impl BoardMeta {
    #[must_use]
    /// Determine whether this board meta-state is drawn by the 50-move rule.
    pub fn drawn_50(&self) -> bool {
        self.rule50 >= 100
    }
}

impl Display for Board {
    /// Display this board in a console-ready format.
    /// Expresses as a series of 8 lines, where the topmost line is the 8th rank and the bottommost
    /// is the 1st.
    /// White pieces are represented with capital letters, while black pieces are represented in
    /// lowercase.
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square::try_from(i).unwrap();
                match self[current_square] {
                    Some((p, Color::White)) => write!(f, "{p} ")?,
                    Some((p, Color::Black)) => write!(f, "{} ", p.code().to_lowercase())?,
                    None => write!(f, ". ")?,
                };
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

impl PartialEq for Board {
    fn eq(&self, other: &Board) -> bool {
        // We assume the board is valid, so we don't need to check the mailbox.
        self.sides == other.sides && self.pieces == other.pieces
    }
}

impl Index<Piece> for Board {
    type Output = Bitboard;

    /// Get the squares occupied by pieces of the given type.
    fn index(&self, index: Piece) -> &Self::Output {
        // SAFETY: This will not fail because there are the same number of pieces as legal indices
        // on `pieces`.
        unsafe { self.pieces.get_unchecked(index as usize) }
    }
}

impl Index<Color> for Board {
    type Output = Bitboard;

    /// Get the squares occupied by pieces of the given color.
    fn index(&self, index: Color) -> &Self::Output {
        // SAFETY: This will not fail because there are the same number of colors as indices on
        // `sides`.
        unsafe { self.sides.get_unchecked(index as usize) }
    }
}

impl Index<Square> for Board {
    type Output = Option<(Piece, Color)>;

    /// Get the type and color of a piece occupying a given square, if it exists.
    fn index(&self, index: Square) -> &Self::Output {
        // SAFETY: This will not fail because there are the same number of squares as there are
        // indices on `mailbox`.
        unsafe { self.mailbox.get_unchecked(index as usize) }
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

impl Display for Game {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for m in &self.moves {
            write!(f, "{m:?} ")?;
        }

        Ok(())
    }
}
