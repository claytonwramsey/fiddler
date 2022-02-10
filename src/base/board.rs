use crate::base::piece::Piece;
use crate::base::zobrist;
use crate::base::Bitboard;
use crate::base::CastleRights;
use crate::base::Color;
use crate::base::Move;
use crate::base::Square;

use std::convert::TryFrom;
use std::default::Default;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::result::Result;

use super::moves::MoveResult;

#[derive(Copy, Clone, Debug)]
///
/// A representation of a position. Does not handle the repetition or turn
/// timer.
///
pub struct Board {
    ///
    /// The squares ocupied by White and Black
    ///
    pub sides: [Bitboard; 2],
    ///
    /// The squares occupied by (in order) pawns, knights, bishops, rooks,
    /// queens, and kings.
    ///
    pub pieces: [Bitboard; Piece::NUM_TYPES],
    ///
    /// The color of the player to move.
    ///
    pub player_to_move: Color,
    ///
    /// The square which can be moved to by a pawn in en passant. Will be
    /// `None` when a pawn has not moved two squares in the previous move.
    ///
    pub en_passant_square: Option<Square>,
    ///
    /// The rights of this piece for castling.
    ///
    pub castle_rights: CastleRights,
    ///
    /// A saved internal hash. If the board is valid, the this value must ALWAYS
    /// be equal to the output of `Board.get_fresh_hash()`.
    ///
    hash: u64,
}

impl Board {
    ///
    /// A "bad" board value which can be used as a debug value.
    ///
    pub const BAD_BOARD: Board = Board {
        sides: [Bitboard::EMPTY; 2],
        pieces: [Bitboard::EMPTY; 6],
        en_passant_square: None,
        player_to_move: Color::White,
        castle_rights: CastleRights::NO_RIGHTS,
        hash: 0,
    };
    ///
    /// Create an empty board with no pieces or castle rights.
    ///
    pub fn empty() -> Board {
        let mut board = Board {
            sides: [Bitboard::EMPTY; 2],
            pieces: [Bitboard::EMPTY; 6],
            en_passant_square: None,
            player_to_move: Color::White,
            castle_rights: CastleRights::NO_RIGHTS,
            hash: 0,
        };
        board.recompute_hash();
        board
    }

    ///
    /// Create a Board populated from some FEN and load it.
    /// Will return `Err` if the FEN is invalid with a string describing why it
    /// failed.
    ///
    pub fn from_fen(fen: &str) -> Result<Board, &'static str> {
        let mut board = Board::empty();
        let mut fen_chrs = fen.chars();
        let mut r = 7; //current row parsed
        let mut c = 0; //current col parsed

        loop {
            if (r, c) == (0, 8) {
                break;
            }
            let chr = match fen_chrs.next() {
                Some(chr) => chr,
                None => return Err("reached end of FEN before board was fully parsed"),
            };
            let is_white = chr.is_uppercase();
            let pt = match chr.to_uppercase().next() {
                Some(c) => Piece::from_code(c),
                None => None,
            };
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
                let num_blanks = match chr.to_digit(10) {
                    Some(num) => num,
                    None => return Err("expected number of blanks"),
                };
                //advance the square under review by the number of blanks
                c += num_blanks as usize;
            }
        }

        //now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after board array section of FEN");
        };

        //now compute player to move
        let player_to_move_chr = match fen_chrs.next() {
            Some(c) => c,
            None => return Err("reached end of string while parsing for player to move"),
        };
        board.player_to_move = match player_to_move_chr {
            'w' => Color::White,
            'b' => Color::Black,
            _ => return Err("unrecognized player to move"),
        };

        //now a space
        if fen_chrs.next() != Some(' ') {
            return Err("expected space after player to move section of FEN");
        }

        //determine castle rights
        let mut castle_chr = match fen_chrs.next() {
            Some(c) => c,
            None => return Err("reached end of string while parsing castle rights"),
        };
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
            castle_chr = match fen_chrs.next() {
                Some(c) => c,
                None => return Err("reached end of string while parsing castle rights"),
            };
        }

        //castle rights searching ate the space, so no need to check for it

        //en passant square
        let ep_file_chr = match fen_chrs.next() {
            Some(c) => c,
            None => return Err("illegal character in en passant square"),
        };
        if ep_file_chr != '-' {
            let ep_rank_chr = match fen_chrs.next() {
                Some(c) => c,
                None => return Err("reached end of string while parsing en passant rank"),
            };
            let mut s = String::from(ep_file_chr);
            s.push(ep_rank_chr);
            board.en_passant_square = match Square::from_algebraic(&s) {
                Ok(sq) => Some(sq),
                Err(e) => return Err(e),
            };
        }
        board.recompute_hash();
        if !(board.is_valid()) {
            return Err("board state after loading was illegal");
        }
        //Ignore move clocks
        Ok(board)
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces.
    ///
    pub fn get_occupancy(&self) -> Bitboard {
        // This gets called so often that unchecked getting is necessary.
        self.get_color_occupancy(Color::White) | self.get_color_occupancy(Color::Black)
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given color. `color` must be
    /// either `Color::White` or `Color::Black`.
    ///
    pub fn get_color_occupancy(&self, color: Color) -> Bitboard {
        // This gets called so often that unchecked lookup is necessary for
        // performance. Because `Color` can only be 0 or 1, this will never
        // result in a fault.
        unsafe { *self.sides.get_unchecked(color as usize) }
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given type. `pt` must be the
    /// type of a valid piece (i.e. pawn, knight, rook, bishop, queen, or king).
    ///
    pub fn get_type(&self, pt: Piece) -> Bitboard {
        // This gets called so often that unchecked lookup is necessary for
        // performance. The enum nature means that this can never be out of
        // bounds.
        unsafe { *self.pieces.get_unchecked(pt as usize) }
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given type and color. The type
    /// must be a valid piece type, and the color must be either `Color::White` or
    /// `Color::Black`.
    ///
    pub fn get_type_and_color(&self, pt: Piece, color: Color) -> Bitboard {
        self.get_type(pt) & self.get_color_occupancy(color)
    }

    ///
    /// Get the type of the piece occupying a given square.
    /// Returns NO_TYPE if there are no pieces occupying the square.
    ///
    pub fn type_at_square(&self, sq: Square) -> Option<Piece> {
        for pt in Piece::ALL_TYPES {
            if self.get_type(pt).contains(sq) {
                return Some(pt);
            }
        }
        None
    }

    #[inline]
    ///
    /// Get the color of a piece occupying a current square.
    /// Returns NO_COLOR if there are
    /// no pieces occupying the square.
    ///
    pub fn color_at_square(&self, sq: Square) -> Option<Color> {
        let bb = Bitboard::from(sq);
        if self.sides[Color::Black as usize] & bb != Bitboard::EMPTY {
            return Some(Color::Black);
        }
        if self.sides[Color::White as usize] & bb != Bitboard::EMPTY {
            return Some(Color::White);
        }
        None
    }

    #[inline]
    ///
    /// Is a given move en passant? Assumes the move is pseudo-legal.
    ///
    pub fn is_move_en_passant(&self, m: Move) -> bool {
        Some(m.to_square()) == self.en_passant_square
            && m.from_square().file() != m.to_square().file()
            && self.type_at_square(m.from_square()) == Some(Piece::Pawn)
    }

    #[inline]
    ///
    /// In this state, is the given move a castle? Assumes the move is
    /// pseudo-legal.
    ///
    pub fn is_move_castle(&self, m: Move) -> bool {
        self.get_type(Piece::King).contains(m.from_square())
            && m.from_square().chebyshev_to(m.to_square()) > 1
    }

    pub fn is_move_promotion(&self, m: Move) -> bool {
        self.get_type_and_color(Piece::Pawn, self.player_to_move)
            .contains(m.from_square())
            && self
                .player_to_move
                .pawn_promote_rank()
                .contains(m.to_square())
    }

    ///
    /// Is the given move a capture in the current state of the board?
    ///
    pub fn is_move_capture(&self, m: Move) -> bool {
        let opponents_bb =
            self.get_color_occupancy(!self.color_at_square(m.from_square()).unwrap());

        opponents_bb.contains(m.to_square())
            || (self.get_type(Piece::Pawn).contains(m.from_square())
                && Some(m.to_square()) == self.en_passant_square)
    }

    ///
    /// Check if the state of this board is valid,
    /// Returns false if the board is invalid.
    ///
    pub fn is_valid(&self) -> bool {
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

        //TODO check if castle rights are legal
        true
    }

    ///
    /// Apply the given move to the board. Will assume the move is legal (unlike
    /// `try_move()`).
    ///
    pub fn make_move(&mut self, m: Move) -> MoveResult {
        let from_sq = m.from_square();
        let to_sq = m.to_square();
        let mover_type = self.type_at_square(from_sq).unwrap();
        let is_en_passant = self.is_move_en_passant(m);
        //this length is used to determine whether it's not a move that a king
        //or pawn could normally make
        let is_long_move = from_sq.chebyshev_to(to_sq) > 1;

        /* Core move functionality */
        self.remove_piece(from_sq);

        let capturee = self.type_at_square(m.to_square());

        /* Promotion and normal piece movement */
        if let Some(p) = m.promote_type() {
            self.set_piece(to_sq, Some(p), self.player_to_move);
        } else {
            //using set_piece handles capturing internally
            self.set_piece(to_sq, Some(mover_type), self.player_to_move);
        }

        /* En passant handling */
        //perform an en passant capture
        if is_en_passant {
            let capturee_sq =
                Square::new(from_sq.rank(), self.en_passant_square.unwrap().file()).unwrap();
            self.remove_piece(capturee_sq);
        }
        //remove previous EP square from hash
        self.hash ^= zobrist::get_ep_key(self.en_passant_square);
        let old_ep_square = self.en_passant_square;
        //update EP square
        self.en_passant_square = match mover_type == Piece::Pawn && is_long_move {
            true => Square::new((from_sq.rank() + to_sq.rank()) / 2, from_sq.file()),
            false => None,
        };
        //insert new EP key into hash
        self.hash ^= zobrist::get_ep_key(self.en_passant_square);

        /* Handling castling */
        //in normal castling, we describe it with a `Move` as a king move which
        //jumps two or three squares.
        if mover_type == Piece::King && is_long_move {
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
            self.remove_piece(rook_from_sq);
            self.add_piece(rook_to_sq, Piece::Rook, self.player_to_move);
        }

        /* Handling castling rights */
        let old_rights = self.castle_rights;
        let mut rights_to_remove;
        if mover_type == Piece::King {
            rights_to_remove = CastleRights::color_rights(self.player_to_move);
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
        self.player_to_move = !self.player_to_move;
        self.hash ^= zobrist::BLACK_TO_MOVE_KEY;

        MoveResult {
            m,
            capturee,
            rights: old_rights,
            ep_square: old_ep_square,
        }
    }

    ///
    /// Apply the given move to the board. Will *not* assume the move is legal
    /// (unlike `make_move()`). On illegal moves, will return an `Err` with a
    /// string describing the issue.
    ///
    pub fn try_move(
        &mut self,
        mgen: &crate::base::movegen::MoveGenerator,
        m: Move,
    ) -> Result<MoveResult, &'static str> {
        let legal_moves = mgen.get_moves(self);
        if !legal_moves.contains(&m) {
            return Err("not contained in the set of legal moves");
        }

        Ok(self.make_move(m))
    }

    ///
    /// Undo a move. `result` must have been created by the most recent output
    /// of `Board::make_move`. Currently it's not faster than just making a 
    /// copy of the original board, though.
    ///
    pub fn undo(&mut self, result: &MoveResult) {
        let from_sq = result.m.from_square();
        let to_sq = result.m.to_square();
        let mover_type = match result.m.promote_type() {
            Some(_) => Piece::Pawn,
            None => self.type_at_square(to_sq).unwrap(),
        };

        let former_player = !self.player_to_move;

        if mover_type == Piece::Pawn && result.ep_square == Some(to_sq) {
            // the previous move was en passant
            self.add_piece(
                to_sq + self.player_to_move.pawn_direction(),
                Piece::Pawn,
                self.player_to_move,
            );
        } else if mover_type == Piece::King && from_sq.chebyshev_to(to_sq) > 1 {
            // this was a castle and requires special effort to undo
            let (rook_from_file, rook_to_file) = match to_sq.file() {
                2 => (0, 3), // queenside castle
                _ => (7, 5), // kingside castle
            };
            let rook_rank = match former_player {
                Color::White => 0,
                Color::Black => 7,
            };
            let rook_from_sq = Square::try_from(rook_rank << 3 | rook_from_file).unwrap();
            let rook_to_sq = Square::try_from(rook_rank << 3 | rook_to_file).unwrap();

            self.remove_piece(rook_to_sq);
            self.add_piece(rook_from_sq, Piece::Rook, former_player);
        }
        self.add_piece(from_sq, mover_type, former_player);
        self.set_piece(to_sq, result.capturee, self.player_to_move);

        self.en_passant_square = result.ep_square;
        self.hash ^= zobrist::get_ep_key(result.ep_square);

        self.return_castle_rights(result.rights);
        self.player_to_move = former_player;
        self.hash ^= zobrist::BLACK_TO_MOVE_KEY;
    }

    #[inline]
    ///
    /// Remove the piece at `sq` from this board.
    ///
    fn remove_piece(&mut self, sq: Square) {
        //Remove the hash from the piece that was there before
        //(no-op if it was empty)

        self.hash ^= match self.color_at_square(sq) {
            Some(c) => zobrist::get_square_key(sq, self.type_at_square(sq), c),
            None => 0,
        };
        let mask = !Bitboard::from(sq);

        for i in 0..Piece::NUM_TYPES {
            self.pieces[i] &= mask;
        }
        self.sides[Color::Black as usize] &= mask;
        self.sides[Color::White as usize] &= mask;
    }

    #[inline]
    ///
    /// Add a piece to the square at a given place on the board.
    /// This should only be called if you believe that the board as-is is empty
    /// at the square below. Otherwise it will break the internal board
    /// representation.
    ///
    fn add_piece(&mut self, sq: Square, pt: Piece, color: Color) {
        //Remove the hash from the piece that was there before (no-op if it was
        //empty)
        let mask = Bitboard::from(sq);
        self.pieces[pt as usize] |= mask;
        self.sides[color as usize] |= mask;
        //Update the hash with the result of our addition
        self.hash ^= zobrist::get_square_key(sq, Some(pt), color);
    }

    #[inline]
    ///
    /// Set the piece at a given position to be a certain piece. This is safe,
    /// and will not result in any issues regarding hash legality. If the given
    /// piece type is None, the color given will be ignored.
    ///
    pub fn set_piece(&mut self, sq: Square, pt: Option<Piece>, color: Color) {
        self.remove_piece(sq);
        if let Some(p) = pt {
            self.add_piece(sq, p, color);
        }
    }

    ///
    /// Give the castle rights indicated back to the board.
    ///
    fn return_castle_rights(&mut self, rights_to_return: CastleRights) {
        let rights_really_returned = rights_to_return & !self.castle_rights;

        for i in 0..4 {
            if 1 << i & rights_really_returned.0 != 0 {
                self.hash ^= zobrist::get_castle_key(i);
            }
        }

        self.castle_rights |= rights_really_returned;
    }

    ///
    /// Remove the given `CastleRights` from this board's castling rights, and
    /// update the internal hash of the board to match.
    ///
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

    #[inline]
    ///
    /// Recompute the Zobrist hash of this board and set it to the saved hash
    /// value.
    ///
    pub fn recompute_hash(&mut self) {
        self.hash = self.get_fresh_hash();
    }

    ///
    /// Compute the hash value of this board from scratch. This should
    /// generally only be used for debug purposes, as in most cases iteratively
    /// updating the hashes as moves are made is enough.
    ///
    fn get_fresh_hash(&self) -> u64 {
        let mut hash = 0;
        for i in 0..64 {
            let sq = Square::try_from(i).unwrap();
            hash ^= match self.color_at_square(sq) {
                Some(c) => zobrist::get_square_key(sq, self.type_at_square(sq), c),
                None => 0,
            };
        }
        for i in 0..4 {
            if 1 << i & self.castle_rights.0 != 0 {
                hash ^= zobrist::get_castle_key(i);
            }
        }
        hash ^= zobrist::get_ep_key(self.en_passant_square);
        hash ^= zobrist::get_player_to_move_key(self.player_to_move);
        hash
    }
}

impl Display for Board {
    ///
    /// Display this board in a console-ready format. Expresses as a series of 8
    /// lines, where the topmost line is the 8th rank and the bottommost is the
    /// 1st. White pieces are represented with capital letters, while black
    /// pieces have lowercase.
    ///
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square::try_from(i).unwrap();
                match self.type_at_square(current_square) {
                    Some(p) => match self.color_at_square(current_square) {
                        Some(Color::White) => write!(f, "{p}")?,
                        Some(Color::Black) => write!(f, "{}", p.get_code().to_lowercase())?,
                        None => write!(f, " ")?,
                    },
                    None => write!(f, " ")?,
                }
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
            && self.player_to_move == other.player_to_move
            && self.castle_rights == other.castle_rights
    }
}

impl Eq for Board {}

impl Default for Board {
    fn default() -> Board {
        let mut board = Board {
            sides: [
                Bitboard(0x000000000000FFFF), //white
                Bitboard(0xFFFF000000000000), //black
            ],
            pieces: [
                Bitboard(0x00FF00000000FF00), //pawn
                Bitboard(0x4200000000000042), //knight
                Bitboard(0x2400000000000024), //bishop
                Bitboard(0x8100000000000081), //rook
                Bitboard(0x0800000000000008), //queen
                Bitboard(0x1000000000000010), //king
            ],
            en_passant_square: None,
            player_to_move: Color::White,
            castle_rights: CastleRights::ALL_RIGHTS,
            hash: 0,
        };
        board.recompute_hash();
        board
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::base::movegen::MoveGenerator;
    use crate::base::square::*;
    use crate::fens;

    ///
    /// A board with the white king on A1 and the black king on H8.
    ///
    const TWO_KINGS_BOARD: Board = Board {
        sides: [
            Bitboard(0x0000000000000001), //white
            Bitboard(0x8000000000000000), //black
        ],
        pieces: [
            Bitboard(0x0000000000000000), //pawn
            Bitboard(0x0000000000000000), //knight
            Bitboard(0x0000000000000000), //bishop
            Bitboard(0x0000000000000000), //rook
            Bitboard(0x0000000000000000), //queen
            Bitboard(0x8000000000000001), //king
        ],
        en_passant_square: None,
        player_to_move: Color::White,
        castle_rights: CastleRights::NO_RIGHTS,
        hash: 3483926298739092744,
    };

    #[test]
    ///
    /// Test that a chessboard with kings on A1 and H8 can be loaded from a FEN.
    ///
    fn test_load_two_kings_fen() {
        let result = Board::from_fen(fens::TWO_KINGS_BOARD_FEN);
        match result {
            Ok(b) => {
                assert_eq!(b, TWO_KINGS_BOARD);
            }
            Err(e) => {
                println!("{e}");
                assert!(false);
            }
        };
    }

    #[test]
    ///
    /// Test that the start position of a normal chess game can be loaded from
    /// its FEN.
    ///
    fn test_start_fen() {
        let result = Board::from_fen(fens::BOARD_START_FEN);
        match result {
            Ok(b) => {
                assert_eq!(b, Board::default());
            }
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        };
    }

    #[test]
    ///
    /// Test that we can play e4 on the first move of the game.
    ///
    fn test_play_e4() {
        test_move_helper(Board::default(), Move::new(Square::E2, Square::E4, None));
    }

    #[test]
    ///
    /// Test that a board with an en passant square can be loaded from a FEN 
    /// correctly.
    ///
    fn test_load_en_passant() {
        let b = Board::from_fen(fens::EN_PASSANT_READY_FEN).unwrap();
        assert_eq!(b.en_passant_square, Some(Square::F6));
    }

    #[test]
    ///
    /// Test that we can capture en passant.
    ///
    fn test_en_passant() {
        test_fen_helper(
            fens::EN_PASSANT_READY_FEN,
            Move::normal(Square::E5, Square::F6),
        );
    }

    ///
    /// Test that White can castle kingside.
    ///
    #[test]
    fn test_white_kingide_castle() {
        test_fen_helper(
            fens::WHITE_KINGSIDE_CASTLE_READY_FEN,
            Move::normal(Square::E1, Square::G1),
        );
    }

    #[test]
    ///
    /// Test that White can promote their pawn to a queen
    ///
    fn test_white_promote_queen() {
        test_fen_helper(
            fens::WHITE_READY_TO_PROMOTE_FEN,
            Move::promoting(Square::F7, Square::F8, Piece::Queen),
        );
    }

    #[test]
    ///
    /// Test that capturing a rook removes the right to castle with that rook.
    ///
    fn test_no_castle_after_capture() {
        let m = Move::new(Square::B2, Square::H8, None);
        let mgen = MoveGenerator::default();
        test_fen_helper(fens::ROOK_HANGING_FEN, m);
        let mut b = Board::from_fen(fens::ROOK_HANGING_FEN).unwrap();
        b.make_move(m);
        let castle_move = Move::new(Square::E8, Square::G8, None);
        assert!(b.try_move(&mgen, castle_move).is_err());
    }

    ///
    /// A helper function which will load a board from a FEN and then try
    /// running the given move on that board.
    ///
    pub fn test_fen_helper(fen: &str, m: Move) {
        let result = Board::from_fen(fen);
        match result {
            Ok(board) => test_move_helper(board, m),
            Err(_) => assert!(false),
        };
    }

    ///
    /// A helper function which will attempt to make a legal move on a board,
    /// and will fail assertions if the board's state was not changed correctly.
    ///
    pub fn test_move_helper(board: Board, m: Move) {
        let mgen = MoveGenerator::default();

        //new_board will be mutated to reflect the move
        let mut new_board = board;

        let result = new_board.try_move(&mgen, m).unwrap();
        test_move_result_helper(board, new_board, m);
        new_board.undo(&result);
        assert_eq!(new_board, board);
    }

    ///
    /// Test that `new_board` was created by playing the move `m` on `
    /// old_board`. Fails assertion if this is not the case.
    ///
    pub fn test_move_result_helper(old_board: Board, new_board: Board, m: Move) {
        let mover_color = old_board.color_at_square(m.from_square()).unwrap();
        let mover_type = old_board.type_at_square(m.from_square()).unwrap();
        let is_en_passant = old_board.is_move_en_passant(m);
        let is_castle = old_board.is_move_castle(m);
        let is_promotion = old_board.is_move_promotion(m);

        assert!(new_board.is_valid());

        if is_promotion {
            assert_eq!(new_board.type_at_square(m.to_square()), m.promote_type());
        } else {
            assert_eq!(new_board.type_at_square(m.to_square()), Some(mover_type));
        }
        assert_eq!(new_board.color_at_square(m.to_square()), Some(mover_color));

        assert_eq!(new_board.type_at_square(m.from_square()), None);
        assert_eq!(new_board.color_at_square(m.from_square()), None);

        //Check en passant worked correctly
        if is_en_passant {
            assert_eq!(
                new_board.type_at_square(old_board.en_passant_square.unwrap()),
                Some(Piece::Pawn)
            );
            assert_eq!(
                new_board.color_at_square(old_board.en_passant_square.unwrap()),
                Some(old_board.player_to_move)
            );
        }

        //Check castling worked correctly
        if is_castle {
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
                Some(old_board.player_to_move)
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
