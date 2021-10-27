use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, NO_TYPE, PIECE_TYPES};
use crate::r#move::Move;
use crate::square::{Square, BAD_SQUARE};

use std::fmt::{Display, Formatter};
use std::ops::{BitOr, BitOrAssign};
use std::result::Result;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
/**
 * A representation of a position. Does not handle the repetition timer, but 
 * will handle turn counts.
 */
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    pub sides: [Bitboard; 2],
    pub pieces: [Bitboard; NUM_PIECE_TYPES],
    pub player_to_move: Color,
    pub en_passant_square: Square,
    pub castle_rights: CastleRights,
}

impl Board {
    /**
     * Make a newly populated board in the board start position.
     */
    pub fn new() -> Board {
        Board {
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
        }
    }

    /**
     * Create an empty board with no pieces or castle rights.
     */
    pub fn empty() -> Board {
        Board {
            sides: [BB_EMPTY, BB_EMPTY],
            pieces: [BB_EMPTY, BB_EMPTY, BB_EMPTY, BB_EMPTY, BB_EMPTY, BB_EMPTY],
            en_passant_square: BAD_SQUARE,
            player_to_move: WHITE,
            castle_rights: CastleRights::NO_RIGHTS,
        }
    }

    /**
     * Create a Board populated from some FEN and load it.
     * Will return Err if the FEN is invalid with a string describing why it 
     * failed.
     */
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
            if pt != NO_TYPE {
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
                board.en_passant_square = Square::new(ep_rank, ep_file);
            }
        }
        if !(board.is_valid()) {
            return Err("board state after loading was illegal");
        }
        //for now let's just ignore move clocks

        return Ok(board);
    }

    #[inline]
    /**
     * Get the squares occupied by pieces.
     */
    pub fn get_occupancy(&self) -> Bitboard {
        self.sides[WHITE] | self.sides[BLACK]
    }

    #[inline]
    //Get the squares occupied by pieces of a given color.
    pub fn get_color_occupancy(&self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    #[inline]
    /**
     * Get the squares occupied by pieces of a given type.
     */
    pub fn get_pieces_of_type(&self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    #[inline]
    /**
     * Get the squares occupied by pieces of a given type and color.
     */
    pub fn get_pieces_of_type_and_color(&self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }

    /**
     * Get the type of the piece occupying a given square.
     * Returns NO_TYPE if there are no pieces occupying the square.
     */
    pub fn type_at_square(&self, sq: Square) -> PieceType {
        let sq_bb = Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            if (self.pieces[i] & sq_bb) != BB_EMPTY {
                return PieceType(i as u8);
            }
        }
        return NO_TYPE;
    }

    /**
     * Get the color of a piece occupying a current square.
     * Returns BLACK (TODO replace this with a bad color value?) if there are 
     * no pieces occupying the square.
     */
    pub fn color_at_square(&self, sq: Square) -> Color {
        match Bitboard::from(sq) & self.sides[WHITE] {
            BB_EMPTY => BLACK,
            _ => WHITE,
        }
    }

    /**
     * Check if the state of this board is valid,
     * Returns false if the board is invalid.
     */
    pub fn is_valid(&self) -> bool {
        let mut sides_checksum = BB_EMPTY;
        let mut sides_checkor = BB_EMPTY;
        let mut pieces_checksum = BB_EMPTY;
        let mut pieces_checkor = BB_EMPTY;
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

        //TODO check if castle rights are legal
        return true;
    }

    /**
     * Apply the given move to the board. Will assume the move is legal (unlike
     * `try_move()`).
     */
    pub fn make_move(&mut self, m: Move) {
        //TODO handle castle rights
        //TODO handle en passant
        //TODO handle castling
        //TODO handle promotion
        //TODO handle turn timer
        let from_sq = m.from_square();
        let to_sq = m.to_square();
        let mover_type = self.type_at_square(from_sq);
        self.remove_piece(from_sq);
        self.set_piece(to_sq, mover_type, self.player_to_move);
    }

    /**
     * Apply the given move to the board. Will *not* assume the move is legal
     * (unlike `make_move()`). On illegal moves, will return an Err with a
     * string describing the issue.
     */
    pub fn try_move(
        &mut self,
        mgen: &crate::movegen::MoveGenerator,
        m: Move,
    ) -> Result<(), &'static str> {
        let legal_moves = mgen.get_moves(self);
        if !legal_moves.contains(&m) {
            return Err("not contained in the set of legal moves");
        }
        self.make_move(m);
        Ok(())
    }

    /**
     * Remove the piece at sq from this board.
     */
    fn remove_piece(&mut self, sq: Square) {
        let mask = !Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            self.pieces[i] &= mask;
        }
        self.sides[BLACK] &= mask;
        self.sides[WHITE] &= mask;
    }

    /**
     * Add a piece to the square at a given place on the board.
     * This should only be called if you believe that the board as-is is empty * at the square below. Otherwise it will break the internal board
     * representation.
     */
    fn add_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        let mask = Bitboard::from(sq);
        self.pieces[pt.0 as usize] |= mask;
        self.sides[color] |= mask;
    }

    #[inline]
    /**
     * Set the piece at a given position to be a certain piece. This is safe,  * and will not result in any issues regarding legality. If the given piece * type is NO_TYPE, the color given will be ignored.
     */
    pub fn set_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        self.remove_piece(sq);
        self.add_piece(sq, pt, color);
    }
}

impl Display for Board {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for r in 0..8 {
            for c in 0..8 {
                let i = 64 - (r + 1) * 8 + c;
                let current_square = Square(i);
                let sq_bb = Bitboard::from(current_square);

                if (sq_bb & self.get_occupancy()) != BB_EMPTY {
                    //TODO capitalize if white pieces
                    let is_white = (sq_bb & self.get_color_occupancy(WHITE)) != BB_EMPTY;
                    //find the type of this piece
                    for pt in PIECE_TYPES {
                        let pt_bb = self.get_pieces_of_type(pt);
                        if (pt_bb & sq_bb) != BB_EMPTY {
                            //there's probably a better way to do this
                            if is_white {
                                write!(f, "{}", pt)?;
                            } else {
                                write!(f, "{}", pt.get_code().to_lowercase())?;
                            }
                            break;
                        }
                    }
                }
                write!(f, " ")?;

                if c == 7 {
                    write!(f, "\n")?;
                }
            }
        }
        write!(f, "")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/**
 * A simple struct to store a piece's castling rights.
 * The internal bits are used to represent castling rights.
 *
 * From MSB to LSB:
 * 4 unused bits
 * Black queenside castling
 * Black kingside castling
 * White queenside castling
 * White kingside castling
 */
pub struct CastleRights(u8);

impl CastleRights {
    pub const ALL_RIGHTS: CastleRights = CastleRights(15);
    pub const NO_RIGHTS: CastleRights = CastleRights(0);

    /**
     * Create a `CastleRights` for kingside castling on one side
     */
    #[inline]
    pub fn king_castle(color: Color) -> CastleRights {
        match color {
            WHITE => CastleRights(1),
            BLACK => CastleRights(4),
            _ => CastleRights(0),
        }
    }

    /**
     * Create a `CastleRights` for queenside castling on one side
     */
    #[inline]
    pub fn queen_castle(color: Color) -> CastleRights {
        match color {
            WHITE => CastleRights(2),
            BLACK => CastleRights(8),
            _ => CastleRights(0),
        }
    }
}

impl BitOr<CastleRights> for CastleRights {
    type Output = CastleRights;
    #[inline]
    fn bitor(self, other: CastleRights) -> CastleRights {
        CastleRights(self.0 | other.0)
    }
}

impl BitOrAssign<CastleRights> for CastleRights {
    #[inline]
    fn bitor_assign(&mut self, other: CastleRights) {
        self.0 |= other.0;
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    /**
     * A board with the white king on A1 and the black king on H8.
     */
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
    };

    #[test]
    /**
     * Test that a chessboard with kinds on A1 and H8 can be loaded from a FEN.
     */
    fn test_load_two_kings_fen() {
        let result = Board::from_fen("7k/8/8/8/8/8/8/K7 w - - 0 1");
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
    /**
     * Test that the start position of a normal chess game can be loaded from
     * its FEN.
     */
    fn test_start_fen() {
        let result = Board::from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        match result {
            Ok(b) => {
                assert_eq!(b, Board::new());
            }
            Err(e) => {
                println!("{}", e);
                assert!(false);
            }
        };
    }
}
