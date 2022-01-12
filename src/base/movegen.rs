use crate::base::constants::{Color, BLACK};
use crate::base::magic::{get_bishop_attacks, get_rook_attacks, MagicTable};
use crate::base::moves::Move;
use crate::base::square::Square;
use crate::base::util::{opposite_color, pawn_direction, pawn_start_rank};
use crate::base::Bitboard;
use crate::base::Board;
use crate::base::Direction;
use crate::base::PieceType;

#[derive(Clone, Debug)]
///
/// A struct which contains all the necessary data to create moves.
///
pub struct MoveGenerator {
    ///
    /// A magic move generator.
    ///
    mtable: MagicTable,
    #[allow(unused)]
    ///
    /// A bitboard of all the squares which a pawn on the given square can
    /// attack.
    ///
    pawn_attacks: [Bitboard; 64], //for now unused, will be used later
    ///
    /// A bitboard of all the squares a king can move to if his position is the
    /// index in the list.
    ///
    king_moves: [Bitboard; 64],
    ///
    /// A bitboard of all the squares a knight can move to if its position is
    /// the index of the list.
    ///
    knight_moves: [Bitboard; 64],
}

impl MoveGenerator {
    ///
    /// Load up a new MoveGenerator.
    ///
    pub fn new() -> MoveGenerator {
        MoveGenerator {
            mtable: MagicTable::load(),
            pawn_attacks: create_step_attacks(&vec![Direction::NORTHEAST, Direction::NORTHWEST], 1),
            king_moves: create_step_attacks(&get_king_steps(), 1),
            knight_moves: create_step_attacks(&get_knight_steps(), 2),
        }
    }

    #[inline]
    ///
    /// Get all the legal moves on a board.
    ///
    pub fn get_moves(&self, board: &Board) -> Vec<Move> {
        self.get_pseudolegal_moves(board, board.player_to_move)
            .into_iter()
            .filter(|m| self.is_pseudolegal_move_legal(board, m))
            .collect()
    }

    #[inline]
    ///
    /// Get moves which are "loud," i.e. captures or checks.
    ///
    pub fn get_loud_moves(&self, board: &Board) -> Vec<Move> {
        self.get_pseudolegal_loud_moves(board)
            .into_iter()
            .filter(|m| self.is_pseudolegal_move_legal(board, m))
            .collect()
    }

    ///
    /// Does the player to move have any legal moves in this position?
    ///
    pub fn has_moves(&self, board: &Board) -> bool {
        let player = board.player_to_move;
        let opponent = opposite_color(player);
        let king_square = Square::from(board.get_type_and_color(PieceType::KING, player));
        /*if king_square == crate::base::square::BAD_SQUARE {
            // no king found
            return false;
        }*/
        let king_attackers = self.get_square_attackers(board, king_square, opponent);

        // moves which can be generated from a given from-square
        let mut move_vec = Vec::new();
        if king_attackers != Bitboard::EMPTY {
            //king is in check

            //King can probably get out on his own
            let king_to_sqs = self.king_moves(board, king_square);
            let mut king_moves = Vec::with_capacity(king_to_sqs.0.count_ones() as usize);
            bitboard_to_moves(king_square, king_to_sqs, &mut king_moves);
            for m in king_moves {
                if !self.is_move_self_check(board, m) && !board.is_move_castle(m) {
                    return true;
                }
            }

            //king moves could not prevent checks
            //if this is a double check, we must be mated
            if king_attackers.0.count_ones() > 1 {
                return false;
            }

            //only blocks can save us from checks
        } else {
            // examine king moves normally
            for from_sq in board.get_type_and_color(PieceType::KING, player) {
                let to_bb = self.sq_pseudolegal_moves(board, from_sq, PieceType::KING);
                move_vec.reserve(to_bb.0.count_ones() as usize);

                // I would uses .drain() here normally, but that's not
                // yet supported.
                bitboard_to_moves(from_sq, to_bb, &mut move_vec);
                for m in move_vec.iter() {
                    if !self.is_move_self_check(board, *m) {
                        return true;
                    }
                }
                move_vec.clear();
            }
        }
        for pt in PieceType::NON_KING_TYPES {
            // examine moves that other pieces can make
            for from_sq in board.get_type_and_color(pt, player) {
                let to_bb = self.sq_pseudolegal_moves(board, from_sq, pt);
                move_vec.reserve(to_bb.0.count_ones() as usize);

                // we need not handle promotion because pawn promotion also
                // blocks. I would uses .drain() here normally, but that's not
                // yet supported.
                bitboard_to_moves(from_sq, to_bb, &mut move_vec);
                for m in move_vec.iter() {
                    if !self.is_move_self_check(board, *m) {
                        return true;
                    }
                }
                move_vec.clear();
            }
        }

        return false;
    }

    ///
    /// Given a pseudolegal move, is that move legal?
    ///
    fn is_pseudolegal_move_legal(&self, board: &Board, m: &Move) -> bool {
        if self.is_move_self_check(board, *m) {
            return false;
        }
        if board.is_move_castle(*m) {
            let is_queen_castle = m.to_square().file() == 2;
            let mut king_passthru_min = 4;
            let mut king_passthru_max = 7;
            if is_queen_castle {
                king_passthru_min = 2;
                king_passthru_max = 5;
            }
            for file in king_passthru_min..king_passthru_max {
                let target_sq = Square::new(m.from_square().rank(), file);
                if self.is_square_attacked_by(
                    board,
                    target_sq,
                    opposite_color(board.player_to_move),
                ) {
                    return false;
                }
            }
        }

        return true;
    }

    ///
    /// In a given board state, is a move illegal because it would be a
    /// self-check?
    ///
    pub fn is_move_self_check(&self, board: &Board, m: Move) -> bool {
        let player = board.color_at_square(m.from_square());
        let player_king_bb = board.get_type_and_color(PieceType::KING, player);
        /*if player_king_bb == Bitboard::EMPTY {
            panic!("king not found!");
        }*/
        let is_king_move = player_king_bb.contains(m.from_square());
        // Square where the king will be after this move ends.
        let mut king_square = Square::from(player_king_bb);
        let opponent = opposite_color(player);

        if is_king_move {
            if self.is_square_attacked_by(board, m.to_square(), opponent) {
                return true;
            }
            // The previous check skips moves where the king blocks himself. We
            // can use magic bitboards to find out the rest.
            king_square = m.to_square();
        }
        // Self checks can only happen by discovery (including by moving the
        // king "out of its own way"), or by doing nothing about a check.
        // Typically, only one square is emptied by moving. However, in en
        // passant, two squares are emptied. We can check the results by masking
        // out the squares which were emptied, and then seeing which attacks
        // went through using magic bitboards.

        let mut squares_emptied = Bitboard::from(m.from_square());
        if board.is_move_en_passant(m) {
            squares_emptied |= Bitboard::from(board.en_passant_square);
        }
        let occupancy = (board.get_occupancy() & !squares_emptied) | Bitboard::from(m.to_square());

        let attackers =
            self.square_attackers_with_occupancy(board, king_square, opponent, occupancy);

        //attackers which we will capture are not a threat
        return (attackers & !Bitboard::from(m.to_square())) != Bitboard::EMPTY;
    }

    #[inline]
    ///
    /// In a given board state, is a square attacked by the given color?
    ///
    pub fn is_square_attacked_by(&self, board: &Board, sq: Square, color: Color) -> bool {
        return self.get_square_attackers(board, sq, color) != Bitboard::EMPTY;
    }

    #[inline]
    ///
    /// Get the attackers of a given color on a square as a `Bitboard`
    /// representing the squares of the attackers.
    ///
    pub fn get_square_attackers(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        self.square_attackers_with_occupancy(board, sq, color, board.get_occupancy())
    }

    ///
    /// Enumerate the pseudolegal moves a player of the given color would be
    /// able to make if it were their turn to move.
    ///
    fn get_pseudolegal_moves(&self, board: &Board, color: Color) -> Vec<Move> {
        let about_to_promote_bb = pawn_start_rank(opposite_color(color));

        let pawns = board.get_type_and_color(PieceType::PAWN, color);
        let promoting_pawns = pawns & about_to_promote_bb;
        let non_promoting_pawns = pawns ^ promoting_pawns;
        // Number of start squares
        let num_promotion_from_squares = promoting_pawns.0.count_ones() as usize;
        let mut normal_bitboards = Vec::with_capacity(
            board.get_color_occupancy(color).0.count_ones() as usize - num_promotion_from_squares,
        );
        let mut promotion_bitboards = Vec::with_capacity(num_promotion_from_squares);

        for sq in non_promoting_pawns {
            normal_bitboards.push((sq, self.pawn_moves(board, sq)));
        }
        for sq in promoting_pawns {
            promotion_bitboards.push((sq, self.pawn_moves(board, sq)));
        }

        //iterate through all the pieces of this color and enumerate their moves
        for pt in PieceType::NON_PAWN_TYPES {
            let pieces_to_move = board.get_type_and_color(pt, color);
            for sq in pieces_to_move {
                normal_bitboards.push((sq, self.sq_pseudolegal_moves(board, sq, pt)));
            }
        }

        let mut num_moves: u32 = normal_bitboards.iter().map(|x| x.1 .0.count_ones()).sum();
        num_moves += (PieceType::NUM_PROMOTE_TYPES as u32)
            * promotion_bitboards
                .iter()
                .map(|x| x.1 .0.count_ones())
                .sum::<u32>();
        let mut moves = Vec::with_capacity(num_moves as usize);
        for (from_sq, bb) in normal_bitboards {
            bitboard_to_moves(from_sq, bb, &mut moves);
        }
        for (from_sq, bb) in promotion_bitboards {
            for promote_type in PieceType::PROMOTE_TYPES {
                bitboard_to_promotions(from_sq, bb, promote_type, &mut moves);
            }
        }

        return moves;
    }

    ///
    /// Enumerate the "loud" pseudolegal moves for a given board.
    ///
    fn get_pseudolegal_loud_moves(&self, board: &Board) -> Vec<Move> {
        let player = board.player_to_move;
        let opponent = opposite_color(player);
        let opponents_bb = board.get_color_occupancy(opponent);
        let about_to_promote_bb = pawn_start_rank(opponent);
        let pawns = board.get_type_and_color(PieceType::PAWN, player);
        let non_promoting_pawns = pawns & !about_to_promote_bb;
        let promoting_pawns = pawns & about_to_promote_bb;
        // Number of start squares
        let num_promotion_from_squares = promoting_pawns.0.count_ones() as usize;
        let mut normal_bitboards = Vec::with_capacity(
            board.get_color_occupancy(player).0.count_ones() as usize - num_promotion_from_squares,
        );
        let mut promotion_bitboards = Vec::with_capacity(num_promotion_from_squares);

        for sq in non_promoting_pawns {
            normal_bitboards.push((sq, self.pawn_captures(board, sq)));
        }
        for sq in promoting_pawns {
            promotion_bitboards.push((sq, self.pawn_captures(board, sq)));
        }

        //iterate through all the pieces of this color and enumerate their moves
        for pt in PieceType::NON_PAWN_TYPES {
            let pieces_to_move = board.get_type_and_color(pt, player);
            for sq in pieces_to_move {
                normal_bitboards
                    .push((sq, self.sq_pseudolegal_moves(board, sq, pt) & opponents_bb));
            }
        }

        let mut num_moves: u32 = normal_bitboards.iter().map(|x| x.1 .0.count_ones()).sum();
        num_moves += (PieceType::NUM_PROMOTE_TYPES as u32)
            * promotion_bitboards
                .iter()
                .map(|x| x.1 .0.count_ones())
                .sum::<u32>();
        let mut moves = Vec::with_capacity(num_moves as usize);
        for (from_sq, bb) in normal_bitboards {
            bitboard_to_moves(from_sq, bb, &mut moves);
        }
        for (from_sq, bb) in promotion_bitboards {
            for promote_type in PieceType::PROMOTE_TYPES {
                bitboard_to_promotions(from_sq, bb, promote_type, &mut moves);
            }
        }

        return moves;
    }

    ///
    /// Same functionality as get_square_attackers, but uses the provided
    /// occupancy bitboard (as opposed to the board's occupancy.)
    ///
    fn square_attackers_with_occupancy(
        &self,
        board: &Board,
        sq: Square,
        color: Color,
        occupancy: Bitboard,
    ) -> Bitboard {
        /*if sq.0 == 64 {
            println!("found an error board!");
            println!("{}", board);
        }*/
        let mut attackers = Bitboard::EMPTY;
        // Check for pawn attacks
        let attackee_pawn_dir = pawn_direction(opposite_color(color));
        let left_pawn_sight = sq + attackee_pawn_dir + Direction::WEST;
        let right_pawn_sight = sq + attackee_pawn_dir + Direction::EAST;
        let mut pawn_vision = Bitboard::EMPTY;
        if left_pawn_sight.chebyshev_to(sq) <= 2 {
            pawn_vision |= Bitboard::from(left_pawn_sight);
        }
        if right_pawn_sight.chebyshev_to(sq) <= 2 {
            pawn_vision |= Bitboard::from(right_pawn_sight);
        }
        attackers |= pawn_vision & board.get_type_and_color(PieceType::PAWN, color);

        // Check for knight attacks
        let knight_vision = self.knight_moves[sq.0 as usize];
        attackers |= knight_vision & board.get_type_and_color(PieceType::KNIGHT, color);

        let enemy_queen_bb = board.get_type_and_color(PieceType::QUEEN, color);

        // Check for rook/horizontal queen attacks
        let rook_vision = get_rook_attacks(occupancy, sq, &self.mtable);
        attackers |=
            rook_vision & (enemy_queen_bb | board.get_type_and_color(PieceType::ROOK, color));

        // Check for bishop/diagonal queen attacks
        let bishop_vision = get_bishop_attacks(occupancy, sq, &self.mtable);
        attackers |=
            bishop_vision & (enemy_queen_bb | board.get_type_and_color(PieceType::BISHOP, color));

        // Check for king attacks
        let king_vision = self.king_moves[sq.0 as usize];
        attackers |= king_vision & board.get_type_and_color(PieceType::KING, color);

        return attackers;
    }

    #[inline]
    ///
    /// Enumerate all the pseudolegal moves that can be made by a given piece
    /// type at the given position.
    ///
    fn sq_pseudolegal_moves(&self, board: &Board, sq: Square, pt: PieceType) -> Bitboard {
        match pt {
            PieceType::PAWN => self.pawn_moves(board, sq),
            PieceType::KNIGHT => self.knight_moves(board, sq),
            PieceType::KING => self.king_moves(board, sq),
            PieceType::BISHOP => self.bishop_moves(board, sq),
            PieceType::ROOK => self.rook_moves(board, sq),
            PieceType::QUEEN => self.queen_moves(board, sq),
            //bad type gets no moves
            _ => Bitboard::EMPTY,
        }
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a knight on the square `sq` could make in
    /// this position. Also, haha bob seger.
    ///
    fn knight_moves(&self, board: &Board, sq: Square) -> Bitboard {
        self.knight_moves[sq.0 as usize] & !board.get_color_occupancy(board.color_at_square(sq))
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a king on square `sq` could make in this
    /// position. Does not check if castling can be done through or out of
    /// check.
    ///
    fn king_moves(&self, board: &Board, sq: Square) -> Bitboard {
        let mut moves =
            self.king_moves[sq.0 as usize] & !board.get_color_occupancy(board.color_at_square(sq));

        //castling
        let kingside_castle_passthrough_sqs = match board.player_to_move {
            BLACK => Bitboard(0x6000000000000000),
            _ => Bitboard(0x0000000000000060),
        };
        let queenside_castle_passthrough_sqs = match board.player_to_move {
            BLACK => Bitboard(0x0C00000000000000),
            _ => Bitboard(0x000000000000000C),
        };

        let can_kingside_castle = board
            .castle_rights
            .is_kingside_castle_legal(board.player_to_move)
            && board.get_occupancy() & kingside_castle_passthrough_sqs == Bitboard::EMPTY;
        let can_queenside_castle = board
            .castle_rights
            .is_queenside_castle_legal(board.player_to_move)
            && board.get_occupancy() & queenside_castle_passthrough_sqs == Bitboard::EMPTY;

        if can_kingside_castle {
            moves |= Bitboard::from(Square::new(sq.rank(), 6));
        }
        if can_queenside_castle {
            moves |= Bitboard::from(Square::new(sq.rank(), 2));
        }
        return moves;
    }

    ///
    /// Get the pseudolegal moves that a pawn on square `sq` could make in this
    /// position.
    ///
    fn pawn_moves(&self, board: &Board, sq: Square) -> Bitboard {
        let player_color = board.color_at_square(sq);
        let dir = pawn_direction(player_color);
        let start_rank = pawn_start_rank(player_color);
        let from_bb = Bitboard::from(sq);
        let occupancy = board.get_occupancy();
        let mut target_squares = Bitboard::EMPTY;
        //this will never be out of bounds because pawns don't live on promotion rank
        if !occupancy.contains(sq + dir) {
            target_squares |= Bitboard::from(sq + dir);
            //pawn is on start rank and double-move square is not occupied
            if (start_rank & from_bb) != Bitboard::EMPTY && !occupancy.contains(sq + 2 * dir) {
                target_squares |= Bitboard::from(sq + 2 * dir);
            }
        }
        target_squares |= self.pawn_captures(board, sq);
        target_squares &= !board.get_color_occupancy(player_color);
        return target_squares;
    }

    ///
    /// Get the captures a pawn can make in the current position.
    ///
    fn pawn_captures(&self, board: &Board, sq: Square) -> Bitboard {
        let opponents = board.get_color_occupancy(opposite_color(board.player_to_move));
        let dir = pawn_direction(board.color_at_square(sq));
        let capture_sqs = [sq + dir + Direction::EAST, sq + dir + Direction::WEST];
        let mut target_squares = Bitboard::EMPTY;
        //captures
        for capture_sq in capture_sqs {
            if capture_sq.is_inbounds() && capture_sq.chebyshev_to(sq) < 3 {
                if capture_sq == board.en_passant_square {
                    target_squares |= Bitboard::from(capture_sq);
                    continue;
                }
                let capture_bb = Bitboard::from(capture_sq);
                target_squares |= capture_bb & opponents;
            }
        }

        target_squares
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a bishop on square `sq` could make in
    /// this position.
    ///
    fn bishop_moves(&self, board: &Board, sq: Square) -> Bitboard {
        get_bishop_attacks(board.get_occupancy(), sq, &self.mtable)
            & !board.get_color_occupancy(board.color_at_square(sq))
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a rook on square `sq` could make in this
    /// position.
    ///
    fn rook_moves(&self, board: &Board, sq: Square) -> Bitboard {
        get_rook_attacks(board.get_occupancy(), sq, &self.mtable)
            & !board.get_color_occupancy(board.color_at_square(sq))
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a queen on square `sq` could make in this
    /// position.
    ///
    fn queen_moves(&self, board: &Board, sq: Square) -> Bitboard {
        self.bishop_moves(board, sq) | self.rook_moves(board, sq)
    }
}

///
/// Get the step attacks that could be made by moving in `dirs` from each point/// in the square. Exclude the steps that travel more than `max_dist` (this
/// prevents overflow around the edges of the board).
///
fn create_step_attacks(dirs: &[Direction], max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard(0); 64];
    for i in 0..64usize {
        for dir in dirs {
            let start_sq = Square(i as u8);
            let target_sq = start_sq + *dir;
            if target_sq.chebyshev_to(start_sq) <= max_dist {
                attacks[i] |= Bitboard::from(target_sq);
            }
        }
    }
    return attacks;
}

#[inline]
///
/// Given a bitboard of possible to-squares and a fixed from-square, convert
/// this to a list of `Move`s with promotion type `NO_TYPE`.
///
fn bitboard_to_moves(from_sq: Square, bb: Bitboard, target: &mut Vec<Move>) {
    bitboard_to_promotions(from_sq, bb, PieceType::NO_TYPE, target);
}

///
/// Given a bitboard of possible to-squares and a fixed from-square, convert
/// this to a list of `Move`s with the given promotion type and push them onto
/// the target.
///
fn bitboard_to_promotions(
    from_sq: Square,
    bb: Bitboard,
    promote_type: PieceType,
    target: &mut Vec<Move>,
) {
    for to_sq in bb {
        target.push(Move::new(from_sq, to_sq, promote_type));
    }
}

///
/// Get the steps a king can make.
///
fn get_king_steps() -> Vec<Direction> {
    vec![
        Direction::NORTH,
        Direction::NORTHEAST,
        Direction::EAST,
        Direction::SOUTHEAST,
        Direction::SOUTH,
        Direction::SOUTHWEST,
        Direction::WEST,
        Direction::NORTHWEST,
    ]
}

///
/// Get the steps a knight can make.
///
fn get_knight_steps() -> Vec<Direction> {
    vec![
        Direction::NNW,
        Direction::NNE,
        Direction::NEE,
        Direction::SEE,
        Direction::SSE,
        Direction::SSW,
        Direction::SWW,
        Direction::NWW,
    ]
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::fens::*;
    use crate::base::square::*;

    #[test]
    fn test_opening_moveset() {
        let mg = MoveGenerator::new();
        let moves = mg.get_moves(&Board::default());
        print!("{{");
        for m in moves.iter() {
            print!("{}, ", m);
        }
        print!("}}");
    }

    #[test]
    ///
    /// Test that we can play Qf3+, the critical move in the Fried Liver
    /// opening.
    ///
    fn test_best_queen_fried_liver() {
        let mg = MoveGenerator::new();
        let m = Move::new(D1, F3, PieceType::NO_TYPE);
        let b = Board::from_fen(FRIED_LIVER_FEN).unwrap();
        let pms = mg.get_pseudolegal_moves(&b, crate::base::constants::WHITE);
        for m2 in pms.iter() {
            println!("{}", m2);
        }
        assert!(pms.contains(&m));
        let moves = mg.get_moves(&b);
        assert!(moves.contains(&m));
    }

    #[test]
    ///
    /// Test that capturing a pawn is parsed correctly.
    ///
    fn test_pawn_capture_generated() {
        let b = Board::from_fen(PAWN_CAPTURE_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let m = Move::new(E4, F5, PieceType::NO_TYPE);

        assert!(mgen.get_moves(&b).contains(&m));
        assert!(mgen.get_loud_moves(&b).contains(&m));
    }

    #[test]
    ///
    /// The pawn is checking the king. Is move enumeration correct?
    ///
    fn test_enumerate_pawn_checking_king() {
        let mgen = MoveGenerator::new();
        let b = Board::from_fen(PAWN_CHECKING_KING_FEN).unwrap();

        let moves = mgen.get_moves(&b);

        for m2 in moves.iter() {
            println!("{}", m2);
        }

        println!("---");

        for lm in mgen.get_loud_moves(&b).iter() {
            println!("{}", lm);
        }
    }

    #[test]
    ///
    /// In a mated position, make sure that the king has no moves.
    ///
    fn test_white_mated_has_no_moves() {
        let b = Board::from_fen(WHITE_MATED_FEN).unwrap();
        let mgen = MoveGenerator::new();
        assert!(!mgen.has_moves(&b));
        assert!(mgen.get_moves(&b).len() == 0);
        assert!(mgen.get_loud_moves(&b).len() == 0);
    }

    #[test]
    ///
    /// Check that the king has exactly one move in this position.
    ///
    fn test_king_has_only_one_move() {
        let b = Board::from_fen(KING_HAS_ONE_MOVE_FEN).unwrap();
        let mgen = MoveGenerator::new();
        assert!(mgen.has_moves(&b));
        assert!(mgen.get_moves(&b).len() == 1);
    }

    #[test]
    ///
    /// Test that queenside castling actually works.
    ///
    fn test_queenside_castle() {
        let b = Board::from_fen(BLACK_QUEENSIDE_CASTLE_READY_FEN).unwrap();
        let mgen = MoveGenerator::new();
        assert!(mgen
            .get_moves(&b)
            .contains(&Move::new(E8, C8, PieceType::NO_TYPE)));
    }

    #[test]
    ///
    /// Test that loud moves are generated correctly on the Fried Liver
    /// position.
    ///
    fn test_get_loud_moves_fried_liver() {
        loud_moves_helper(FRIED_LIVER_FEN);
    }

    #[test]
    fn test_get_loud_moves_scholars_mate() {
        loud_moves_helper(SCHOLARS_MATE_FEN);
    }

    #[test]
    fn test_get_loud_moves_mate_in_4() {
        loud_moves_helper(MATE_IN_4_FEN);
    }

    #[test]
    fn test_get_loud_moves_en_passant() {
        loud_moves_helper(EN_PASSANT_READY_FEN);
    }

    #[test]
    fn test_get_loud_moves_pawn_capture() {
        loud_moves_helper(PAWN_CAPTURE_FEN);
    }

    #[test]
    fn test_get_loud_moves_king_checked() {
        loud_moves_helper(PAWN_CHECKING_KING_FEN);
    }

    #[test]
    fn test_get_loud_moves_rook_hanging() {
        loud_moves_helper(ROOK_HANGING_FEN);
    }

    #[test]
    fn test_recapture_knight_loud_move() {
        loud_moves_helper("r2q1bkr/ppp3pp/2n5/3Np3/6Q1/8/PPPP1PPP/R1B1K2R b KQ - 0 10");
    }

    #[test]
    ///
    /// Test that a king can escape check without capturing the checker.
    ///
    fn test_king_escape_without_capture() {
        let b = Board::from_fen(KING_MUST_ESCAPE_FEN).unwrap();
        let mgen = MoveGenerator::new();
        let moves = mgen.get_moves(&b);
        let expected_moves = vec![
            Move::new(E6, D6, PieceType::NO_TYPE),
            Move::new(E6, F7, PieceType::NO_TYPE),
            Move::new(E6, E7, PieceType::NO_TYPE),
            Move::new(F6, G4, PieceType::NO_TYPE),
        ];
        for m in moves.iter() {
            assert!(expected_moves.contains(m));
        }
        for em in expected_moves.iter() {
            assert!(moves.contains(em));
        }
    }

    ///
    /// A helper function that will force that the given FEN will have loud
    /// moves generated correctly.
    ///
    fn loud_moves_helper(fen: &str) {
        let b = Board::from_fen(fen).unwrap();
        let mgen = MoveGenerator::new();

        let moves = mgen.get_moves(&b);
        let loud_moves = mgen.get_loud_moves(&b);

        for loud_move in loud_moves.iter() {
            println!("{}", loud_move);
            assert!(moves.contains(&loud_move));
            assert!(b.is_move_capture(*loud_move));
        }

        for normal_move in moves.iter() {
            if b.is_move_capture(*normal_move) {
                assert!(loud_moves.contains(normal_move));
            }
        }
    }
}
