use crate::Board;

use super::{Piece, Square};

use std::{
    fmt::{Display, Formatter},
    mem::transmute,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The information of one move, containing its from- and to-squares, as well as
/// its promote type, in one integer.
/// From MSB to LSB:
/// * 2 bits: flags (promotion, castling, or en passant)
/// * 2 bits: promote type
/// * 6 bits: from-square
/// * 6 bits: to-square
pub struct Move(u16);

impl Move {
    /// A sentinel value for a move which is illegal, or otherwise
    /// inexpressible.
    pub const BAD_MOVE: Move = Move(0xFFFF);

    /// The mask used to extract the flag bits from a move.
    const FLAG_MASK: u16 = 0xC000;

    /// The flag bits representing a move which is promotion.
    const PROMOTE_FLAG: u16 = 0x4000;

    /// The flag bits representing a move which is a castle.
    const CASTLE_FLAG: u16 = 0x8000;

    /// The flag bits representing a move which is en passant.
    const EN_PASSANT_FLAG: u16 = 0xC000;

    #[inline(always)]
    /// Make a new `Move` for a piece. Assumes that all the inputs are valid.
    pub const fn new(
        from_square: Square,
        to_square: Square,
        promote_type: Option<Piece>,
        castle: bool,
        en_passant: bool,
    ) -> Move {
        let mut bits = from_square as u16;
        bits |= (to_square as u16) << 6;

        if let Some(p) = promote_type {
            bits |= (p as u16) << 12 | Move::PROMOTE_FLAG; // promotion type
        }

        // apply flags
        bits |= match (castle, en_passant) {
            (false, false) => 0, // no special flag bits
            (false, true) => Move::EN_PASSANT_FLAG,
            (true, false) => Move::CASTLE_FLAG,
            (true, true) => panic!("move cannot be both castle and en passant"),
        };

        Move(bits)
    }

    #[inline(always)]
    /// Create a `Move` with no promotion type.
    pub const fn normal(from_square: Square, to_square: Square) -> Move {
        Move::new(from_square, to_square, None, false, false)
    }

    #[inline(always)]
    /// Create a `Move` with the given promotion type. The promote type must
    /// not be a pawn or a king.
    pub const fn promoting(from_square: Square, to_square: Square, promote_type: Piece) -> Move {
        Move::new(from_square, to_square, Some(promote_type), false, false)
    }

    #[inline(always)]
    /// Create a `Move` which is tagged as a castling move.
    pub const fn castling(from_square: Square, to_square: Square) -> Move {
        Move::new(from_square, to_square, None, true, false)
    }

    #[inline(always)]
    /// Create a `Move` which is tagged as a castling move.
    pub const fn en_passant(from_square: Square, to_square: Square) -> Move {
        Move::new(from_square, to_square, None, false, true)
    }

    #[inline(always)]
    /// Get the target square of this move.
    pub fn to_square(&self) -> Square {
        // Masking out the bottom bits will make this always valid.
        unsafe { transmute(((self.0 >> 6) & 63u16) as u8) }
    }

    #[inline(always)]
    /// Get the square that a piece moves from to execute this move.
    pub fn from_square(&self) -> Square {
        // Masking out the bottom bits will make this always valid
        unsafe { transmute((self.0 & 63u16) as u8) }
    }

    #[inline(always)]
    /// Determine whether this move is marked as a promotion.
    pub fn is_promotion(&self) -> bool {
        self.0 & Move::FLAG_MASK == Move::PROMOTE_FLAG
    }

    #[inline(always)]
    /// Determine whether this move is marked as a castle.
    pub fn is_castle(&self) -> bool {
        self.0 & Move::FLAG_MASK == Move::CASTLE_FLAG
    }

    #[inline(always)]
    /// Determine whether this move is marked as an en passant capture.
    pub fn is_en_passant(&self) -> bool {
        self.0 & Move::FLAG_MASK == Move::EN_PASSANT_FLAG
    }

    #[inline(always)]
    /// Get the promotion type of this move. The resulting type will never be a
    /// pawn or a king.
    pub fn promote_type(&self) -> Option<Piece> {
        if self.is_promotion() {
            Some(unsafe { std::mem::transmute(((self.0 >> 12) & 3u16) as u8) })
        } else {
            None
        }
    }

    /// Convert a move from its UCI representation. Requires the board the move
    /// was played on to determine extra flags about the move.
    pub fn from_uci(s: &str, board: &Board) -> Result<Move, &'static str> {
        if !(s.len() == 4 || s.len() == 5) {
            return Err("string was neither a normal move or a promotion");
        }
        let from_sq = Square::from_algebraic(&s[0..2])?;
        let to_sq = Square::from_algebraic(&s[2..4])?;
        let promote_type = if s.len() == 5 {
            // this is valid because we already checked the length of s
            let charcode = s.chars().nth(4).unwrap();
            let pt = Piece::from_code(charcode.to_ascii_uppercase());
            if pt == None {
                return Err("invalid promote type given");
            }
            pt
        } else {
            None
        };

        let is_castle = board[Piece::King].contains(from_sq) && from_sq.chebyshev_to(to_sq) > 1;

        let is_en_passant =
            board[Piece::Pawn].contains(from_sq) && board.en_passant_square == Some(to_sq);

        Ok(Move::new(
            from_sq,
            to_sq,
            promote_type,
            is_castle,
            is_en_passant,
        ))
    }

    /// Construct a UCI string version of this move.
    pub fn to_uci(&self) -> String {
        match self.promote_type() {
            None => format!("{}{}", self.from_square(), self.to_square()),
            Some(p) => format!(
                "{}{}{}",
                self.from_square(),
                self.to_square(),
                p.code().to_lowercase()
            ),
        }
    }

    #[inline(always)]
    /// Get a number representing this move uniquely. The value may change from
    /// version to version.
    pub const fn value(&self) -> u16 {
        self.0
    }

    #[inline(always)]
    /// Reconstruct a move based on its `value`. Should only be used with
    /// values returned from `Move::value()`.
    pub const fn from_val(val: u16) -> Move {
        Move(val)
    }
}

impl Display for Move {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.promote_type() {
            None => write!(f, "{} -> {}", self.from_square(), self.to_square())?,
            Some(p) => write!(f, "{} -> {} ={}", self.from_square(), self.to_square(), p)?,
        };
        if self.is_en_passant() {
            write!(f, " [e.p.]")?;
        }
        if self.is_castle() {
            write!(f, " [castle]")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uci_move_normal() {
        let b = Board::default();
        let m = Move::from_uci("e2e4", &b).unwrap();
        println!("{m}");
        assert_eq!(m, Move::new(Square::E2, Square::E4, None, false, false));
    }

    #[test]
    fn test_uci_move_promotion() {
        assert_eq!(
            Move::from_uci(
                "b7b8q",
                &Board::from_fen("r1b1kbnr/pPqppppp/2n5/8/8/8/P1PPPPPP/RNBQKBNR w KQkq - 1 5")
                    .unwrap(),
            )
            .unwrap(),
            Move::new(Square::B7, Square::B8, Some(Piece::Queen), false, false),
        );
    }
}
