use crate::bitboard::{Bitboard, BB_EMPTY};
use crate::constants::NUM_PIECE_TYPES;
use crate::constants::{Color, BLACK, WHITE};
use crate::piece::{PieceType, NO_TYPE, PIECE_TYPES};
use crate::r#move::Move;
use crate::square::{Square, BAD_SQUARE};

use std::fmt::{Display, Formatter};
use std::ops::{BitOr, BitOrAssign};
use std::result::Result;

#[derive(Copy, Clone, Debug)]
pub struct Board {
    //a bitboard for both color occupancies and then for each piece type
    pub sides: [Bitboard; 2],
    pub pieces: [Bitboard; NUM_PIECE_TYPES],
    pub player_to_move: Color,
    pub en_passant_square: Square,
    pub castle_rights: CastleRights,
}

impl Board {
    //Make a newly populated board in the board start position.
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

    //Create a Board populated from some FEN and load it.
    //Will return Err if the FEN is invalid.
    pub fn from_fen(fen: &str) -> Result<Board, &'static str> {
        let mut board = Board::new();
        let mut fen_chrs = fen.chars();
        let mut r = 0; //current row parsed
        let mut c = 0; //current col parsed

        while r < 8 {
            if (r, c) == (7, 8) {
                break;
            }
            let chr = fen_chrs.next().unwrap_or('!');
            //illegal character, cannot parse
            if chr == '!' {
                return Err("illegal character found");
            }
            let is_white = chr.is_uppercase();
            let pt = PieceType::from_code(chr.to_uppercase().next().unwrap_or('_'));
            let color = match is_white {
                true => WHITE,
                false => BLACK,
            };
            if pt != NO_TYPE {
                //character is a piece type
                board.add_piece(Square::new(8 - r, c), pt, color);
            } else if chr == '/' {
                //row divider
                r += 1;
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

        //for now let's just ignore move clocks

        return Ok(board);
    }

    #[inline]
    pub fn get_occupancy(&self) -> Bitboard {
        self.sides[WHITE] | self.sides[BLACK]
    }

    #[inline]
    pub fn get_color_occupancy(&self, color: Color) -> Bitboard {
        self.sides[color as usize]
    }

    #[inline]
    pub fn get_pieces_of_type(&self, pt: PieceType) -> Bitboard {
        self.pieces[pt.0 as usize]
    }

    #[inline]
    pub fn get_pieces_of_type_and_color(&self, pt: PieceType, color: Color) -> Bitboard {
        self.get_pieces_of_type(pt) & self.get_color_occupancy(color)
    }

    pub fn type_at_square(&self, sq: Square) -> PieceType {
        let sq_bb = Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            if (self.pieces[i] & sq_bb) != BB_EMPTY {
                return PieceType(i as u8);
            }
        }
        return NO_TYPE;
    }

    pub fn color_at_square(&self, sq: Square) -> Color {
        match Bitboard::from(sq) & self.sides[WHITE] {
            BB_EMPTY => BLACK,
            _ => WHITE,
        }
    }

    //Check through the state of this board and return false if this is an
    //invalid board state
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
        return true;
    }

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
        self.remove_piece(to_sq);
        self.add_piece(to_sq, mover_type, self.player_to_move);
    }

    //Remove the piece at sq from the board.
    fn remove_piece(&mut self, sq: Square) {
        let mask = !Bitboard::from(sq);
        for i in 0..NUM_PIECE_TYPES {
            self.pieces[i] &= mask;
        }
        self.sides[BLACK] &= mask;
        self.sides[WHITE] &= mask;
    }

    fn add_piece(&mut self, sq: Square, pt: PieceType, color: Color) {
        let mask = Bitboard::from(sq);
        self.pieces[pt.0 as usize] |= mask;
        self.sides[color] |= mask;
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
                                if let Err(e) = write!(f, "{}", pt) {
                                    println!("Error {} while trying to write board!", e.to_string())
                                }
                            } else {
                                if let Err(e) = write!(f, "{}", pt.get_code().to_lowercase()) {
                                    println!("Error {} while trying to write board!", e.to_string())
                                }
                            }
                            break;
                        }
                    }
                } else if let Err(e) = write!(f, " ") {
                    println!("Error {} while trying to write board!", e.to_string());
                }

                if c == 7 {
                    if let Err(e) = write!(f, "\n") {
                        println!("Error {} while trying to write board!", e.to_string());
                    }
                }
            }
        }
        write!(f, "")
    }
}

//MSB to LSB:
//4 unused
//Black queenside
//Black kingside
//White queenside
//White kingside
#[derive(Clone, Copy, Debug)]
pub struct CastleRights(u8);

impl CastleRights {
    pub const ALL_RIGHTS: CastleRights = CastleRights(15);
    pub const NO_RIGHTS: CastleRights = CastleRights(0);

    //Create a castling rights for kingside castling on one side.
    #[inline]
    pub fn king_castle(color: Color) -> CastleRights {
        match color {
            WHITE => CastleRights(1),
            BLACK => CastleRights(4),
            _ => CastleRights(0),
        }
    }

    //Create a castling rights for queenside castling on one side.
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
