use crate::base::constants::{Color, BLACK, NO_COLOR, WHITE};
use crate::base::piece::PieceType;
use crate::base::square::{Square, A1, A8, BAD_SQUARE, H1, H8};
use crate::base::util::{opposite_color, pawn_promote_rank};
use crate::base::zobrist;
use crate::base::Bitboard;
use crate::base::CastleRights;
use crate::base::Move;

use std::default::Default;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::result::Result;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
///
/// A representation of a position. Does not handle the repetition or turn timer.
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
    pub pieces: [Bitboard; PieceType::NUM_TYPES],
    ///
    /// The color of the player to move. Should always be `BLACK` or `WHITE`.
    ///
    pub player_to_move: Color,
    ///
    /// The square which can be moved to by a pawn in en passant. If en passant
    /// is not legal, this will be a `BAD_SQUARE`.
    ///
    pub en_passant_square: Square,
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
    pub const BAD_BOARD: Board = Board {
        sides: [Bitboard::EMPTY, Bitboard::EMPTY],
        pieces: [
            Bitboard::EMPTY,
            Bitboard::EMPTY,
            Bitboard::EMPTY,
            Bitboard::EMPTY,
            Bitboard::EMPTY,
            Bitboard::EMPTY,
        ],
        en_passant_square: BAD_SQUARE,
        player_to_move: WHITE,
        castle_rights: CastleRights::NO_RIGHTS,
        hash: 0,
    };
    ///
    /// Create an empty board with no pieces or castle rights.
    ///
    pub fn empty() -> Board {
        let mut board = Board {
            sides: [Bitboard::EMPTY, Bitboard::EMPTY],
            pieces: [
                Bitboard::EMPTY,
                Bitboard::EMPTY,
                Bitboard::EMPTY,
                Bitboard::EMPTY,
                Bitboard::EMPTY,
                Bitboard::EMPTY,
            ],
            en_passant_square: BAD_SQUARE,
            player_to_move: WHITE,
            castle_rights: CastleRights::NO_RIGHTS,
            hash: 0,
        };
        board.recompute_hash();
        return board;
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
            let chr = fen_chrs.next().unwrap_or('!');
            //illegal character or reached end of fen string
            if chr == '!' {
                return Err("reached end of FEN string before completing the board");
            }
            let is_white = chr.is_uppercase();
            let pt = PieceType::from_code(chr.to_uppercase().next().unwrap_or('_'));
            let color = match is_white {
                true => WHITE,
                false => BLACK,
            };
            if pt != PieceType::NO_TYPE {
                //character is a piece type
                board.add_piece(Square::new(r, c), pt, color);
                c += 1;
            } else if chr == '/' {
                //row divider
                r -= 1;
                c = 0;
            } else {
                //number stating number of blank spaces in this row
                let num_blanks = chr.to_digit(10).unwrap_or(0);
                if num_blanks == 0 {
                    //we were unable to get the number of blanks
                    return Err("could not parse FEN character");
                } else {
                    //advance the square under review by the number of blanks
                    c += num_blanks as usize;
                }
            }
        }

        //now a space
        if fen_chrs.next().unwrap_or('!') != ' ' {
            return Err("expected space after board array section of FEN");
        }

        //now compute player to move
        let player_to_move_chr = fen_chrs.next().unwrap_or('!');
        board.player_to_move = match player_to_move_chr {
            'w' => WHITE,
            'b' => BLACK,
            _ => return Err("unrecognized player to move"),
        };

        //now a space
        if fen_chrs.next().unwrap_or('!') != ' ' {
            return Err("expected space after player to move section of FEN");
        }

        //determine castle rights
        let mut castle_chr = fen_chrs.next().unwrap_or('!');
        while castle_chr != ' ' {
            //this may accept some technically illegal FENS, but that's ok
            //note: hash was not updated, so will need to be rewritten by the
            //end of the function.
            board.castle_rights |= match castle_chr {
                'K' => CastleRights::king_castle(WHITE),
                'Q' => CastleRights::queen_castle(WHITE),
                'k' => CastleRights::king_castle(BLACK),
                'q' => CastleRights::queen_castle(BLACK),
                '-' => CastleRights::NO_RIGHTS,
                _ => return Err("unrecognized castle rights character"),
            };
            castle_chr = fen_chrs.next().unwrap_or('!');
        }

        //castle rights searching ate the space, so no need to check for it

        //en passant square
        let ep_file_chr = fen_chrs.next().unwrap_or('!');
        if ep_file_chr == '!' {
            return Err("illegal character in en passant square");
        }
        if ep_file_chr != '-' {
            if !"abcdefgh".contains(ep_file_chr) {
                return Err("illegal file for en passant square");
            }
            //99 is just a dummy err value
            let (ep_file, _) = "abcdefgh"
                .match_indices(ep_file_chr)
                .next()
                .unwrap_or((99, "!"));
            if ep_file != 99 {
                let ep_rank = fen_chrs.next().unwrap_or('!').to_digit(10).unwrap_or(99) as usize;
                if ep_rank == 99 {
                    return Err("illegal rank for en passant square");
                }
                board.en_passant_square = Square::new(ep_rank - 1, ep_file);
            }
        }
        board.recompute_hash();
        if !(board.is_valid()) {
            return Err("board state after loading was illegal");
        }
        //Ignore move clocks
        return Ok(board);
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces.
    ///
    pub fn get_occupancy(&self) -> Bitboard {
        self.sides[WHITE] | self.sides[BLACK]
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given color.
    ///
    pub fn get_color_occupancy(&self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given type.
    ///
    pub fn get_type(&self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    #[inline]
    ///
    /// Get the squares occupied by pieces of a given type and color.
    ///
    pub fn get_type_and_color(&self, pt: PieceType, color: Color) -> Bitboard {
        self.get_type(pt) & self.get_color_occupancy(color)
    }

    ///
    /// Get the type of the piece occupying a given square.
    /// Returns NO_TYPE if there are no pieces occupying the square.
    ///
    pub fn type_at_square(&self, sq: Square) -> PieceType {
        let sq_bb = Bitboard::from(sq);
        for i in 0..PieceType::NUM_TYPES {
            if (self.pieces[i] & sq_bb) != Bitboard::EMPTY {
                return PieceType(i as u8);
            }
        }
        return PieceType::NO_TYPE;
    }

    #[inline]
    ///
    /// Get the color of a piece occupying a current square.
    /// Returns NO_COLOR if there are
    /// no pieces occupying the square.
    ///
    pub fn color_at_square(&self, sq: Square) -> Color {
        let bb = Bitboard::from(sq);
        if self.sides[BLACK] & bb != Bitboard::EMPTY {
            return BLACK;
        }
        if self.sides[WHITE] & bb != Bitboard::EMPTY {
            return WHITE;
        }
        return NO_COLOR;
    }

    #[inline]
    ///
    /// Is a given move en passant? Assumes the move is pseudo-legal.
    ///
    pub fn is_move_en_passant(&self, m: Move) -> bool {
        m.to_square() == self.en_passant_square
            && m.from_square().file() != m.to_square().file()
            && self.type_at_square(m.from_square()) == PieceType::PAWN
    }

    #[inline]
    ///
    /// In this state, is the given move a castle? Assumes the move is
    /// pseudo-legal.
    ///
    pub fn is_move_castle(&self, m: Move) -> bool {
        self.get_type(PieceType::KING).contains(m.from_square())
            && m.from_square().chebyshev_to(m.to_square()) > 1
    }

    pub fn is_move_promotion(&self, m: Move) -> bool {
        self.get_type_and_color(PieceType::PAWN, self.player_to_move)
            .contains(m.from_square())
            && Bitboard::from(m.to_square()) & pawn_promote_rank(self.player_to_move)
                != Bitboard::EMPTY
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
        return true;
    }

    ///
    /// Apply the given move to the board. Will assume the move is legal (unlike
    /// `try_move()`).
    ///
    pub fn make_move(&mut self, m: Move) {
        let from_sq = m.from_square();
        let to_sq = m.to_square();
        let mover_type = self.type_at_square(from_sq);
        let is_en_passant = self.is_move_en_passant(m);
        let is_promotion =
            mover_type == PieceType::PAWN && pawn_promote_rank(self.player_to_move).contains(to_sq);
        //this length is used to determine whether it's not a move that a king
        //or pawn could normally make
        let is_long_move = from_sq.chebyshev_to(to_sq) > 1;

        /* Core move functionality */
        self.remove_piece(from_sq);

        /* Promotion and normal piece movement */
        if is_promotion {
            self.set_piece(to_sq, m.promote_type(), self.player_to_move);
        } else {
            //using set_piece handles capturing internally
            self.set_piece(to_sq, mover_type, self.player_to_move);
        }

        /* En passant handling */
        //perform an en passant capture
        if is_en_passant {
            let capturee_sq = Square::new(from_sq.rank(), self.en_passant_square.file());
            self.remove_piece(capturee_sq);
        }
        //remove previous EP square from hash
        self.hash ^= zobrist::get_ep_key(self.en_passant_square);
        //update EP square
        self.en_passant_square = match mover_type == PieceType::PAWN && is_long_move {
            true => Square::new((from_sq.rank() + to_sq.rank()) / 2, from_sq.file()),
            false => BAD_SQUARE,
        };
        //insert new EP key into hash
        self.hash ^= zobrist::get_ep_key(self.en_passant_square);

        /* Handling castling */
        //in normal castling, we describe it with a `Move` as a king move which
        //jumps two or three squares.
        if mover_type == PieceType::KING && is_long_move {
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
            let rook_from_sq = Square::new(from_sq.rank(), rook_from_file);
            let rook_to_sq = Square::new(from_sq.rank(), rook_to_file);
            self.remove_piece(rook_from_sq);
            self.add_piece(rook_to_sq, PieceType::ROOK, self.player_to_move);
        }

        /* Handling castling rights */
        let rights_to_remove;
        if mover_type == PieceType::KING {
            rights_to_remove = CastleRights::color_rights(self.player_to_move);
        } else {
            //don't need to check if it's a rook because moving from this square
            //would mean you didn't have the right anyway
            rights_to_remove = match from_sq {
                A1 => CastleRights::queen_castle(WHITE),
                H1 => CastleRights::king_castle(WHITE),
                A8 => CastleRights::queen_castle(BLACK),
                H8 => CastleRights::king_castle(BLACK),
                _ => CastleRights::NO_RIGHTS,
            };
        }
        self.remove_castle_rights(rights_to_remove);

        /* Updating player to move */
        self.player_to_move = opposite_color(self.player_to_move);
        self.hash ^= zobrist::BLACK_TO_MOVE_KEY;
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
    ) -> Result<(), &'static str> {
        let legal_moves = mgen.get_moves(self);
        if !legal_moves.contains(&m) {
            return Err("not contained in the set of legal moves");
        }
        self.make_move(m);
        Ok(())
    }

    ///
    /// Remove the piece at `sq` from this board.
    ///
    fn remove_piece(&mut self, sq: Square) {
        //Remove the hash from the piece that was there before
        //(no-op if it was empty)
        self.hash ^= zobrist::get_square_key(sq, self.type_at_square(sq), self.color_at_square(sq));
        let mask = !Bitboard::from(sq);

        for i in 0..PieceType::NUM_TYPES {
            self.pieces[i] &= mask;
        }
        self.sides[BLACK] &= mask;
        self.sides[WHITE] &= mask;
    }

    ///
    /// Add a piece to the square at a given place on the board.
    /// This should only be called if you believe that the board as-is is empty
    /// at the square below. Otherwise it will break the internal board
    /// representation.
    ///
    fn add_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        //Remove the hash from the piece that was there before (no-op if it was
        //empty)
        let mask = Bitboard::from(sq);
        self.pieces[pt.0 as usize] |= mask;
        self.sides[color] |= mask;
        //Update the hash with the result of our addition
        self.hash ^= zobrist::get_square_key(sq, pt, color);
    }

    #[inline]
    ///
    /// Set the piece at a given position to be a certain piece. This is safe,
    /// and will not result in any issues regarding hash legality. If the given
    /// piece type is `NO_TYPE`, the color given will be ignored.
    ///
    pub fn set_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        self.remove_piece(sq);
        if pt != PieceType::NO_TYPE {
            self.add_piece(sq, pt, color);
        }
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
    /// Compute the hash value of this board from scratch.
    ///
    fn get_fresh_hash(&self) -> u64 {
        let mut hash = 0;
        for i in 0..64 {
            let sq = Square(i);
            hash ^= zobrist::get_square_key(sq, self.type_at_square(sq), self.color_at_square(sq));
        }
        for i in 0..4 {
            if 1 << i & self.castle_rights.0 != 0 {
                hash ^= zobrist::get_castle_key(i);
            }
        }
        hash ^= zobrist::get_ep_key(self.en_passant_square);
        hash ^= zobrist::get_player_to_move_key(self.player_to_move);
        return hash;
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
                let current_square = Square(i);
                let pt = self.type_at_square(current_square);

                match self.color_at_square(current_square) {
                    WHITE => write!(f, "{}", pt)?,
                    BLACK => write!(f, "{}", pt.get_code().to_lowercase())?,
                    _ => write!(f, " ")?,
                };
            }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

impl Hash for Board {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.hash.hash(state);
    }
}

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
            en_passant_square: BAD_SQUARE,
            player_to_move: WHITE,
            castle_rights: CastleRights::ALL_RIGHTS,
            hash: 0,
        };
        board.recompute_hash();
        return board;
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::base::fens;
    use crate::base::movegen::MoveGenerator;
    use crate::base::square::*;

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
        en_passant_square: BAD_SQUARE,
        player_to_move: WHITE,
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
                println!("{}", e);
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
        test_move_helper(Board::default(), Move::new(E2, E4, PieceType::NO_TYPE));
    }

    #[test]
    ///
    /// Test that we can capture en passant.
    ///
    fn test_en_passant() {
        test_fen_helper(
            fens::EN_PASSANT_READY_FEN,
            Move::new(E5, F6, PieceType::NO_TYPE),
        );
    }

    ///
    /// Test that White can castle kingside.
    ///
    #[test]
    fn test_white_kingide_castle() {
        test_fen_helper(
            fens::WHITE_KINGSIDE_CASTLE_READY_FEN,
            Move::new(E1, G1, PieceType::NO_TYPE),
        );
    }

    #[test]
    ///
    /// Test that White can promote their pawn to a queen
    ///
    fn test_white_promote_queen() {
        test_fen_helper(
            fens::WHITE_READY_TO_PROMOTE_FEN,
            Move::new(F7, F8, PieceType::QUEEN),
        );
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
        let mgen = MoveGenerator::new();

        //new_board will be mutated to reflect the move
        let mut new_board = board;

        let result = new_board.try_move(&mgen, m);

        assert_eq!(result, Ok(()));

        test_move_result_helper(board, new_board, m);
    }

    ///
    /// Test that `new_board` was created by playing the move `m` on `
    /// old_board`. Fails assertion if this is not the case.
    ///
    pub fn test_move_result_helper(old_board: Board, new_board: Board, m: Move) {
        let mover_color = old_board.color_at_square(m.from_square());
        let mover_type = old_board.type_at_square(m.from_square());
        let is_en_passant = old_board.is_move_en_passant(m);
        let is_castle = old_board.is_move_castle(m);
        let is_promotion = old_board.is_move_promotion(m);

        assert!(new_board.is_valid());

        if is_promotion {
            assert_eq!(new_board.type_at_square(m.to_square()), m.promote_type());
        } else {
            assert_eq!(new_board.type_at_square(m.to_square()), mover_type);
        }
        assert_eq!(new_board.color_at_square(m.to_square()), mover_color);

        assert_eq!(
            new_board.type_at_square(m.from_square()),
            PieceType::NO_TYPE
        );
        assert_eq!(new_board.color_at_square(m.from_square()), NO_COLOR);

        //Check en passant worked correctly
        if is_en_passant {
            assert_eq!(
                new_board.type_at_square(old_board.en_passant_square),
                PieceType::PAWN
            );
            assert_eq!(
                new_board.color_at_square(old_board.en_passant_square),
                old_board.player_to_move
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
            let rook_start_sq = Square::new(m.from_square().rank(), rook_start_file);
            let rook_end_sq = Square::new(m.from_square().rank(), rook_end_file);

            assert_eq!(new_board.type_at_square(rook_start_sq), PieceType::NO_TYPE);
            assert_eq!(new_board.color_at_square(rook_start_sq), NO_COLOR);

            assert_eq!(new_board.type_at_square(rook_end_sq), PieceType::ROOK);
            assert_eq!(
                new_board.color_at_square(rook_end_sq),
                old_board.player_to_move
            );
        }

        // TODO Check castling rights were removed correctly
    }
}
