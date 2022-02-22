use crate::base::magic::{get_bishop_attacks, get_rook_attacks, MagicTable};
use crate::base::moves::Move;
use crate::base::Bitboard;
use crate::base::Board;
use crate::base::Color;
use crate::base::Direction;
use crate::base::Piece;
use crate::base::Square;

use std::convert::TryFrom;

#[derive(Clone, Debug)]
///
/// A struct which contains all the necessary data to create moves.
///
pub struct MoveGenerator {
    ///
    /// A magic move generator.
    ///
    mtable: MagicTable,
    ///
    /// A bitboard of all the squares which a pawn on the given square can
    /// attack. The first index is for White's pawn attacks, the second is for
    /// Black's.
    ///
    pawn_attacks: [[Bitboard; 64]; 2],
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
    ///
    /// A lookup table for the squares "between" two other squares, either down
    /// a row like a rook or on a diagonal like a bishop. `between[A1][A3]`
    /// would return a `Bitboard` with A2 as its only active square.
    ///
    between: [[Bitboard; 64]; 64],
    ///
    /// A lookup table for the squares on a line between any two squares,
    /// either down a row like a rook or diagonal like a bishop.
    /// `lines[A1][B2]` would return a bitboard with active squares down the
    /// main diagonal.
    ///
    lines: [[Bitboard; 64]; 64],
}

///
/// A struct containing information for generating moves without self-checking,
/// such as the necessary pieces to block the king. When a `Game` makes a move,
/// this information is expected to be lost.
///
pub struct CheckInfo {
    ///
    /// The locations of pieces which are checking the king in the current
    /// position.
    ///
    checkers: Bitboard,
    ///
    /// The locations of pieces that are blocking would-be checkers from the
    /// opponent.
    king_blockers: [Bitboard; 2],
    #[allow(unused)]
    ///
    /// The locations of pieces which are pinning their corresponding blockers
    /// in `king_blockers`.
    ///
    pinners: [Bitboard; 2],
    #[allow(unused)]
    ///
    /// The squares which each piece could move to to check the opposing king.
    ///
    check_squares: [Bitboard; Piece::NUM_TYPES],
}

impl MoveGenerator {
    #[inline]
    ///
    /// Get all the legal moves on a board.
    ///
    pub fn get_moves(&self, board: &Board) -> Vec<Move> {
        let check_info = self.create_check_info(board);

        let mut moves = Vec::with_capacity(218);
        let in_check = check_info.checkers != Bitboard::EMPTY;

        match in_check {
            false => self.all_moves(board, board.player_to_move, &check_info, &mut moves),
            true => self.evasions(board, board.player_to_move, &check_info, &mut moves),
        };

        // Eliminate moves that would put us in check.
        moves
            .into_iter()
            .filter(|&m| self.validate(board, &check_info, m))
            .collect()
    }

    #[inline]
    ///
    /// Get moves which are "loud," i.e. captures or checks.
    ///
    pub fn get_loud_moves(&self, board: &Board) -> Vec<Move> {
        let check_info = self.create_check_info(board);
        if check_info.checkers != Bitboard::EMPTY {
            panic!("loud moves cannot be requested for board where king is checked");
        }

        let mut moves = Vec::with_capacity(50);

        self.loud_moves(board, &check_info, &mut moves);

        // Eliminate moves that would put us in check.
        moves
            .into_iter()
            .filter(|&m| self.validate(board, &check_info, m))
            .collect()
    }

    ///
    /// Does the player to move have any legal moves in this position?
    ///
    pub fn has_moves(&self, board: &Board) -> bool {
        let player = board.player_to_move;
        let player_occupancy = board[player];
        let opponent = !player;
        let king_square = Square::try_from(board[Piece::King] & player_occupancy).unwrap();
        let king_attackers = self.get_square_attackers(board, king_square, opponent);

        // moves which can be generated from a given from-square
        // 28 is the maximum number of moves that a single piece can make
        let mut move_vec = Vec::with_capacity(28);
        if king_attackers != Bitboard::EMPTY {
            //king is in check

            //King can probably get out on his own
            let king_to_sqs = self.king_moves(board, king_square, player);
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
            let to_bb = self.king_moves(board, king_square, player);

            // I would uses .drain() here normally, but that's not
            // yet supported.
            bitboard_to_moves(king_square, to_bb, &mut move_vec);
            for m in move_vec.iter() {
                if !self.is_move_self_check(board, *m) {
                    return true;
                }
            }
            move_vec.clear();
        }
        for pt in Piece::NON_KING_TYPES {
            // examine moves that other pieces can make
            for from_sq in board[pt] & player_occupancy {
                let to_bb = match pt {
                    Piece::Pawn => self.pawn_moves(board, from_sq, player),
                    Piece::Bishop => self.bishop_moves(board, from_sq, player),
                    Piece::Rook => self.rook_moves(board, from_sq, player),
                    Piece::Queen => {
                        self.bishop_moves(board, from_sq, player)
                            | self.rook_moves(board, from_sq, player)
                    }
                    Piece::Knight => self.knight_moves(board, from_sq, player),
                    _ => Bitboard::EMPTY,
                };

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

        false
    }

    ///
    /// Construct check information for a given board.
    ///
    pub fn create_check_info(&self, b: &Board) -> CheckInfo {
        let white_king_sq = Square::try_from(b[Piece::King] & b[Color::White]).unwrap();
        let black_king_sq = Square::try_from(b[Piece::King] & b[Color::Black]).unwrap();

        let (blockers_white, pinners_black) = self.analyze_pins(b, b[Color::Black], white_king_sq);
        let (blockers_black, pinners_white) = self.analyze_pins(b, b[Color::White], black_king_sq);

        let king_sq = Square::try_from(b[Piece::King] & b[b.player_to_move]).unwrap();

        // TODO this ignores checks by retreating the rook?
        let rook_check_sqs = get_rook_attacks(b.occupancy(), king_sq, &self.mtable);
        let bishop_check_sqs = get_bishop_attacks(b.occupancy(), king_sq, &self.mtable);

        CheckInfo {
            checkers: self.get_square_attackers(b, king_sq, !b.player_to_move),
            king_blockers: [blockers_white, blockers_black],
            pinners: [pinners_white, pinners_black],
            check_squares: [
                self.pawn_attacks[b.player_to_move as usize][king_sq as usize],
                self.knight_moves[king_sq as usize],
                bishop_check_sqs,
                rook_check_sqs,
                bishop_check_sqs | rook_check_sqs,
                Bitboard::EMPTY,
            ],
        }
    }

    ///
    /// Examine the pins in a position to generate the set of pinners and
    /// blockers on the square `sq`. The first return val is the set of
    /// blockers, and the second return val is the set of pinners. The blockers
    /// are pieces of either color that prevent an attack on `sq`. `sliders` is
    /// the set of all squares containing attackers we are interested in -
    /// typically, this is the set of all pieces owned by one color.
    ///  
    fn analyze_pins(&self, board: &Board, sliders: Bitboard, sq: Square) -> (Bitboard, Bitboard) {
        let mut blockers = Bitboard::EMPTY;
        let mut pinners = Bitboard::EMPTY;
        let sq_color = board.color_at_square(sq);
        let occupancy = board.occupancy();

        let rook_mask = get_rook_attacks(Bitboard::EMPTY, sq, &self.mtable);
        let bishop_mask = get_bishop_attacks(Bitboard::EMPTY, sq, &self.mtable);

        // snipers are pieces that could be pinners
        let snipers = sliders
            & ((rook_mask & (board[Piece::Queen] | board[Piece::Rook]))
                | (bishop_mask & (board[Piece::Queen] | board[Piece::Bishop])));

        // find the snipers which are blocked by only one piece
        for sniper_sq in snipers {
            let between_bb = self.between(sq, sniper_sq);

            if (between_bb & occupancy).0.count_ones() == 1 {
                blockers |= between_bb;
                if let Some(color) = sq_color {
                    if board[color] & between_bb != Bitboard::EMPTY {
                        pinners |= Bitboard::from(sniper_sq);
                    }
                }
            }
        }

        (blockers, pinners)
    }

    #[inline]
    ///
    /// Determine whether a move is valid in the position on the board, given
    /// that it was generated during the `get_moves` process.
    ///
    fn validate(&self, board: &Board, check_info: &CheckInfo, m: Move) -> bool {
        // the pieces which are pinned
        let pinned = check_info.king_blockers[board.player_to_move as usize];
        let from_bb = Bitboard::from(m.from_square());
        let to_bb = Bitboard::from(m.to_square());

        // verify that taking en passant does not result in self-check
        if board.is_move_en_passant(m) {
            let player = board.player_to_move;
            let king_sq = Square::try_from(board[Piece::King] & board[player]).unwrap();
            let enemy = board[!board.player_to_move];

            let capture_bb = match board.player_to_move {
                Color::White => to_bb >> 8,
                Color::Black => to_bb << 8,
            };

            let new_occupancy = board.occupancy() ^ from_bb ^ capture_bb ^ to_bb;

            return (get_rook_attacks(new_occupancy, king_sq, &self.mtable)
                & (board[Piece::Rook] | board[Piece::Queen])
                & enemy
                == Bitboard::EMPTY)
                && (get_bishop_attacks(new_occupancy, king_sq, &self.mtable)
                    & (board[Piece::Bishop] | board[Piece::Queen])
                    & enemy
                    == Bitboard::EMPTY);
        }

        // Validate passthrough squares for castling
        if board.is_move_castle(m) {
            let is_queen_castle = m.to_square().file() == 2;
            let mut king_passthru_min = 4;
            let mut king_passthru_max = 7;
            if is_queen_castle {
                king_passthru_min = 2;
                king_passthru_max = 5;
            }
            for file in king_passthru_min..king_passthru_max {
                let target_sq = Square::new(m.from_square().rank(), file).unwrap();
                if self.is_square_attacked_by(board, target_sq, !board.player_to_move) {
                    return false;
                }
            }
        }

        // Other king moves must make sure they don't step into check
        if board[Piece::King] & from_bb != Bitboard::EMPTY {
            let new_occupancy = (board.occupancy() ^ from_bb) | to_bb;
            return self.square_attackers_with_occupancy(
                board,
                Square::try_from(to_bb).unwrap(),
                !board.player_to_move,
                new_occupancy,
            ) == Bitboard::EMPTY;
        }

        let king_sq = Square::try_from(board[Piece::King] & board[board.player_to_move]).unwrap();
        (pinned & from_bb == Bitboard::EMPTY)
            || self.aligned(m.from_square(), m.to_square(), king_sq)
    }

    ///
    /// In a given board state, is a move illegal because it would be a
    /// self-check?
    ///
    pub fn is_move_self_check(&self, board: &Board, m: Move) -> bool {
        let player = board.player_to_move;
        let player_king_bb = board[Piece::King] & board[player];
        let is_king_move = player_king_bb.contains(m.from_square());
        // Square where the king will be after this move ends.
        let mut king_square = Square::try_from(player_king_bb).unwrap();
        let opponent = !player;

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
            squares_emptied |=
                Bitboard::from(board.en_passant_square.unwrap() + opponent.pawn_direction());
        }
        let occupancy = (board.occupancy() & !squares_emptied) | Bitboard::from(m.to_square());

        let attackers =
            self.square_attackers_with_occupancy(board, king_square, opponent, occupancy);

        //attackers which we will capture are not a threat
        (attackers & !Bitboard::from(m.to_square())) != Bitboard::EMPTY
    }

    #[inline]
    ///
    /// In a given board state, is a square attacked by the given color?
    ///
    pub fn is_square_attacked_by(&self, board: &Board, sq: Square, color: Color) -> bool {
        self.get_square_attackers(board, sq, color) != Bitboard::EMPTY
    }

    #[inline]
    ///
    /// Get the attackers of a given color on a square as a `Bitboard`
    /// representing the squares of the attackers.
    ///
    pub fn get_square_attackers(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        self.square_attackers_with_occupancy(board, sq, color, board.occupancy())
    }

    ///
    /// Enumerate the pseudolegal moves a player of the given color would be
    /// able to make if it were their turn to move.
    ///
    fn all_moves(
        &self,
        board: &Board,
        color: Color,
        _check_info: &CheckInfo,
        moves: &mut Vec<Move>,
    ) {
        self.normal_piece_assistant(board, color, moves, Bitboard::ALL);
        self.pawn_assistant(board, color, moves, Bitboard::ALL);
        for sq in board[Piece::King] & board[color] {
            bitboard_to_moves(sq, self.king_moves(board, sq, color), moves);
        }
    }

    #[allow(unused)]
    ///
    /// Enumerate the evasions in a position, in case the king is in check.
    ///
    fn evasions(&self, board: &Board, color: Color, check_info: &CheckInfo, moves: &mut Vec<Move>) {
        let king_sq = Square::try_from(board[Piece::King] & board[board.player_to_move]).unwrap();

        // only look at king moves if we are in double check
        if check_info.checkers.0.count_ones() == 1 {
            let checker_sq = Square::try_from(check_info.checkers).unwrap();
            // Look for blocks or captures
            let target_sqs = self.between(king_sq, checker_sq) | check_info.checkers;
            let mut pawn_targets = target_sqs;
            if let Some(ep_sq) = board.en_passant_square {
                // can en passant save us from check?
                let ep_target = ep_sq - color.pawn_direction();
                if check_info.checkers.contains(ep_sq) {
                    pawn_targets |= Bitboard::from(ep_sq);
                }
            }

            self.pawn_assistant(board, color, moves, pawn_targets);
            self.normal_piece_assistant(board, color, moves, target_sqs);
        }

        for sq in board[Piece::King] & board[color] {
            bitboard_to_moves(sq, self.king_moves(board, sq, color), moves);
        }
    }

    #[inline]
    ///
    /// Enumerate the "loud" pseudolegal moves for a given board.
    ///
    fn loud_moves(&self, board: &Board, _check_info: &CheckInfo, moves: &mut Vec<Move>) {
        let player = board.player_to_move;
        let target_sqs = board[!player];
        let mut pawn_targets = target_sqs | player.pawn_promote_rank();
        if let Some(sq) = board.en_passant_square {
            pawn_targets |= Bitboard::from(sq);
        }

        self.normal_piece_assistant(board, player, moves, target_sqs);
        self.loud_pawn_assistant(board, player, moves, pawn_targets);
        for sq in board[Piece::King] & board[player] {
            bitboard_to_moves(sq, self.king_moves(board, sq, player) & target_sqs, moves);
        }
    }

    ///
    /// Generate the moves all pawns can make and populate `moves` with those
    /// moves.
    ///
    fn pawn_assistant(&self, board: &Board, color: Color, moves: &mut Vec<Move>, target: Bitboard) {
        let about_to_promote_bb = (!color).pawn_start_rank();
        let color_occupancy = board[color];
        let pawns = board[Piece::Pawn] & color_occupancy;
        let promoting_pawns = pawns & about_to_promote_bb;
        let non_promoting_pawns = pawns ^ promoting_pawns;

        // TODO use bitshifts to accelerate pawn move generation
        for sq in non_promoting_pawns {
            bitboard_to_moves(sq, self.pawn_moves(board, sq, color) & target, moves);
        }
        for sq in promoting_pawns {
            let pmoves_bb = self.pawn_moves(board, sq, color) & target;
            bitboard_to_promotions(sq, pmoves_bb, Some(Piece::Queen), moves);
            bitboard_to_promotions(sq, pmoves_bb, Some(Piece::Knight), moves);
            bitboard_to_promotions(sq, pmoves_bb, Some(Piece::Bishop), moves);
            bitboard_to_promotions(sq, pmoves_bb, Some(Piece::Rook), moves);
        }
    }

    ///
    /// Generate the loud moves all pawns can make and populate `moves` with
    /// those moves.
    ///
    fn loud_pawn_assistant(
        &self,
        board: &Board,
        color: Color,
        moves: &mut Vec<Move>,
        target: Bitboard,
    ) {
        let about_to_promote_bb = (!color).pawn_start_rank();
        let color_occupancy = board[color];
        let pawns = board[Piece::Pawn] & color_occupancy;
        let promoting_pawns = pawns & about_to_promote_bb;
        let non_promoting_pawns = pawns ^ promoting_pawns;

        // TODO use bitshifts to accelerate pawn move generation
        for sq in non_promoting_pawns {
            bitboard_to_moves(sq, self.pawn_moves(board, sq, color) & target, moves);
        }
        for sq in promoting_pawns {
            let pmoves_bb = self.pawn_moves(board, sq, color) & target;
            bitboard_to_promotions(sq, pmoves_bb, Some(Piece::Queen), moves);
        }
    }

    ///
    /// Generate all the moves for a knight, bishop, rook, or queen which end
    /// up on the target.
    ///
    fn normal_piece_assistant(
        &self,
        board: &Board,
        color: Color,
        moves: &mut Vec<Move>,
        target: Bitboard,
    ) {
        let color_occupancy = board[color];
        let queens = board[Piece::Queen];
        let rook_movers = (board[Piece::Rook] | queens) & color_occupancy;
        let bishop_movers = (board[Piece::Bishop] | queens) & color_occupancy;

        for sq in board[Piece::Knight] & color_occupancy {
            bitboard_to_moves(sq, self.knight_moves(board, sq, color) & target, moves);
        }
        for sq in bishop_movers {
            bitboard_to_moves(sq, self.bishop_moves(board, sq, color) & target, moves);
        }
        for sq in rook_movers {
            bitboard_to_moves(sq, self.rook_moves(board, sq, color) & target, moves);
        }
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
        let mut attackers = Bitboard::EMPTY;
        let color_bb = board[color];
        // Check for pawn attacks
        let pawn_vision = self.pawn_captures(board, sq, !color);
        attackers |= pawn_vision & board[Piece::Pawn];

        // Check for knight attacks
        let knight_vision = self.knight_moves[sq as usize];
        attackers |= knight_vision & board[Piece::Knight];

        let queens_bb = board[Piece::Queen];

        // Check for rook/horizontal queen attacks
        let rook_vision = get_rook_attacks(occupancy, sq, &self.mtable);
        attackers |= rook_vision & (queens_bb | board[Piece::Rook]);

        // Check for bishop/diagonal queen attacks
        let bishop_vision = get_bishop_attacks(occupancy, sq, &self.mtable);
        attackers |= bishop_vision & (queens_bb | board[Piece::Bishop]);

        // Check for king attacks
        let king_vision = self.king_moves[sq as usize];
        attackers |= king_vision & board[Piece::King];

        attackers & color_bb
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a knight on the square `sq` could make in
    /// this position. `color` is the color of the piece at `sq`.
    /// bob seger.
    ///
    fn knight_moves(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        self.knight_moves[sq as usize] & !board[color]
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a king on square `sq` could make in this
    /// position. Does not check if castling can be done through or out of
    /// check. `color` is the color of the piece at `sq`.
    ///
    fn king_moves(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        let mut moves = self.king_moves[sq as usize] & !board[color];

        //castling
        let kingside_castle_passthrough_sqs = match board.player_to_move {
            Color::White => Bitboard(0x0000000000000060),
            Color::Black => Bitboard(0x6000000000000000),
        };
        let queenside_castle_passthrough_sqs = match board.player_to_move {
            Color::White => Bitboard(0x000000000000000E),
            Color::Black => Bitboard(0x0E00000000000000),
        };

        let can_kingside_castle = board
            .castle_rights
            .is_kingside_castle_legal(board.player_to_move)
            && board.occupancy() & kingside_castle_passthrough_sqs == Bitboard::EMPTY;
        let can_queenside_castle = board
            .castle_rights
            .is_queenside_castle_legal(board.player_to_move)
            && board.occupancy() & queenside_castle_passthrough_sqs == Bitboard::EMPTY;

        if can_kingside_castle {
            moves |= Bitboard::from(Square::new(sq.rank(), 6).unwrap());
        }
        if can_queenside_castle {
            moves |= Bitboard::from(Square::new(sq.rank(), 2).unwrap());
        }

        moves
    }

    ///
    /// Get the pseudolegal moves that a pawn on square `sq` with color `color`
    /// could make in this position.
    ///
    fn pawn_moves(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        let dir = color.pawn_direction();
        let start_rank = color.pawn_start_rank();
        let from_bb = Bitboard::from(sq);
        let occupancy = board.occupancy();
        let mut target_squares = Bitboard::EMPTY;
        //this will never be out of bounds because pawns don't live on promotion rank
        if !occupancy.contains(sq + dir) {
            target_squares |= Bitboard::from(sq + dir);
            //pawn is on start rank and double-move square is not occupied
            if (start_rank & from_bb) != Bitboard::EMPTY && !occupancy.contains(sq + 2 * dir) {
                target_squares |= Bitboard::from(sq + 2 * dir);
            }
        }
        target_squares |= self.pawn_captures(board, sq, color);
        target_squares &= !board[color];

        target_squares
    }

    ///
    /// Get the captures a pawn can make in the current position. The given
    /// color is the color that a pawn would be to generate the captures from
    /// this square. `color` is the color of the piece at `sq`.
    ///
    fn pawn_captures(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        let mut capture_mask = board[!color];
        if let Some(ep_square) = board.en_passant_square {
            capture_mask |= Bitboard::from(ep_square);
        }

        self.pawn_attacks[color as usize][sq as usize] & capture_mask
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a bishop on square `sq` could make in
    /// this position. `color` is the color of the piece at `sq`.
    ///
    fn bishop_moves(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        get_bishop_attacks(board.occupancy(), sq, &self.mtable) & !board[color]
    }

    #[inline]
    ///
    /// Get the pseudolegal moves that a rook on square `sq` could make in this
    /// position. `color` is the color of the piece at `sq`.
    ///
    fn rook_moves(&self, board: &Board, sq: Square, color: Color) -> Bitboard {
        get_rook_attacks(board.occupancy(), sq, &self.mtable) & !board[color]
    }

    #[inline]
    ///
    /// Get a bitboard of all the squares between the two given squares, along
    /// the moves of a bishop or rook.
    ///
    pub fn between(&self, sq1: Square, sq2: Square) -> Bitboard {
        self.between[sq1 as usize][sq2 as usize]
    }

    #[inline]
    ///
    /// Determine whether three squares are aligned according to rook or bishop
    /// directions.
    ///
    pub fn aligned(&self, sq1: Square, sq2: Square, sq3: Square) -> bool {
        self.lines[sq1 as usize][sq2 as usize] & Bitboard::from(sq3) != Bitboard::EMPTY
    }
}

impl Default for MoveGenerator {
    fn default() -> MoveGenerator {
        let mut mgen = MoveGenerator {
            mtable: MagicTable::load(),
            pawn_attacks: [
                create_step_attacks(&[Direction::NORTHEAST, Direction::NORTHWEST], 1),
                create_step_attacks(&[Direction::SOUTHEAST, Direction::SOUTHWEST], 1),
            ],
            king_moves: create_step_attacks(&get_king_steps(), 1),
            knight_moves: create_step_attacks(&get_knight_steps(), 2),
            between: [[Bitboard::EMPTY; 64]; 64],
            lines: [[Bitboard::EMPTY; 64]; 64],
        };

        // populate `between`
        for sq1 in Bitboard::ALL {
            let ln_bishop_1 = get_bishop_attacks(Bitboard::EMPTY, sq1, &mgen.mtable);
            let ln_rook_1 = get_rook_attacks(Bitboard::EMPTY, sq1, &mgen.mtable);
            for sq2 in Bitboard::ALL {
                if ln_bishop_1.contains(sq2) {
                    let ln_bishop_2 = get_bishop_attacks(Bitboard::EMPTY, sq2, &mgen.mtable);
                    let bt_bishop1 = get_bishop_attacks(Bitboard::from(sq2), sq1, &mgen.mtable);
                    let bt_bishop2 = get_bishop_attacks(Bitboard::from(sq1), sq2, &mgen.mtable);
                    mgen.lines[sq1 as usize][sq2 as usize] |=
                        Bitboard::from(sq1) | Bitboard::from(sq2);
                    mgen.lines[sq1 as usize][sq2 as usize] |= ln_bishop_1 & ln_bishop_2;
                    mgen.between[sq1 as usize][sq2 as usize] |= bt_bishop1 & bt_bishop2;
                }
                if ln_rook_1.contains(sq2) {
                    let ln_rook_2 = get_rook_attacks(Bitboard::EMPTY, sq2, &mgen.mtable);
                    let bt_rook1 = get_rook_attacks(Bitboard::from(sq2), sq1, &mgen.mtable);
                    let bt_rook2 = get_rook_attacks(Bitboard::from(sq1), sq2, &mgen.mtable);
                    mgen.lines[sq1 as usize][sq2 as usize] |=
                        Bitboard::from(sq1) | Bitboard::from(sq2);
                    mgen.between[sq1 as usize][sq2 as usize] |= bt_rook1 & bt_rook2;
                    mgen.lines[sq1 as usize][sq2 as usize] |= ln_rook_1 & ln_rook_2;
                }
            }
        }

        mgen
    }
}

///
/// Get the step attacks that could be made by moving in `dirs` from each point
/// in the square. Exclude the steps that travel more than `max_dist` (this
/// prevents overflow around the edges of the board).
///
fn create_step_attacks(dirs: &[Direction], max_dist: u8) -> [Bitboard; 64] {
    let mut attacks = [Bitboard(0); 64];
    for (i, item) in attacks.iter_mut().enumerate() {
        for dir in dirs {
            let start_sq = Square::try_from(i as u8).unwrap();
            let target_sq = start_sq + *dir;
            if target_sq.chebyshev_to(start_sq) <= max_dist {
                *item |= Bitboard::from(target_sq);
            }
        }
    }

    attacks
}

#[inline]
///
/// Given a bitboard of possible to-squares and a fixed from-square, convert
/// this to a list of `Move`s with promotion type `NO_TYPE`.
///
fn bitboard_to_moves(from_sq: Square, bb: Bitboard, target: &mut Vec<Move>) {
    bitboard_to_promotions(from_sq, bb, None, target);
}

///
/// Given a bitboard of possible to-squares and a fixed from-square, convert
/// this to a list of `Move`s with the given promotion type and push them onto
/// the target.
///
fn bitboard_to_promotions(
    from_sq: Square,
    bb: Bitboard,
    promote_type: Option<Piece>,
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

    #[test]
    fn test_opening_moveset() {
        let mg = MoveGenerator::default();
        let moves = mg.get_moves(&Board::default());
        print!("{{");
        for m in moves.iter() {
            print!("{m}, ");
        }
        print!("}}");
    }

    #[test]
    ///
    /// Test that we can play Qf3+, the critical move in the Fried Liver
    /// opening.
    ///
    fn test_best_queen_fried_liver() {
        let mg = MoveGenerator::default();
        let m = Move::new(Square::D1, Square::F3, None);
        let b = Board::from_fen(FRIED_LIVER_FEN).unwrap();
        let pms = mg.get_moves(&b);
        for m2 in pms.iter() {
            println!("{m2}");
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
        let mgen = MoveGenerator::default();
        let m = Move::new(Square::E4, Square::F5, None);
        for m in mgen.get_moves(&b) {
            println!("{}", m);
        }
        assert!(mgen.get_moves(&b).contains(&m));
        assert!(mgen.get_loud_moves(&b).contains(&m));
    }

    #[test]
    ///
    /// The pawn is checking the king. Is move enumeration correct?
    ///
    fn test_enumerate_pawn_checking_king() {
        let mgen = MoveGenerator::default();
        let b = Board::from_fen(PAWN_CHECKING_KING_FEN).unwrap();

        let moves = mgen.get_moves(&b);

        for m2 in moves.iter() {
            println!("{m2}");
        }
    }

    #[test]
    ///
    /// In a mated position, make sure that the king has no moves.
    ///
    fn test_white_mated_has_no_moves() {
        let b = Board::from_fen(WHITE_MATED_FEN).unwrap();
        let mgen = MoveGenerator::default();
        assert!(!mgen.has_moves(&b));
        let moves = mgen.get_moves(&b);
        for m in moves {
            print!("{m}, ");
        }
        assert!(mgen.get_moves(&b).is_empty());
    }

    #[test]
    ///
    /// Check that the king has exactly one move in this position.
    ///
    fn test_king_has_only_one_move() {
        let b = Board::from_fen(KING_HAS_ONE_MOVE_FEN).unwrap();
        let mgen = MoveGenerator::default();
        assert!(mgen.has_moves(&b));
        assert!(mgen.get_moves(&b).len() == 1);
    }

    #[test]
    ///
    /// Test that queenside castling actually works.
    ///
    fn test_queenside_castle() {
        let b = Board::from_fen(BLACKQUEENSIDE_CASTLE_READY_FEN).unwrap();
        let mgen = MoveGenerator::default();
        assert!(mgen
            .get_moves(&b)
            .contains(&Move::normal(Square::E8, Square::C8)));
    }

    #[test]
    ///
    /// Test that Black cannot castle because there is a knight in the way.
    ///
    fn test_no_queenside_castle_through_knight() {
        let b = Board::from_fen(KNIGHT_PREVENTS_LONG_CASTLE_FEN).unwrap();
        let mgen = MoveGenerator::default();
        assert!(!mgen
            .get_moves(&b)
            .contains(&Move::normal(Square::E8, Square::C8)));
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
    fn test_get_loud_moves_en_passant() {
        loud_moves_helper(EN_PASSANT_READY_FEN);
    }

    #[test]
    fn test_get_loud_moves_pawn_capture() {
        loud_moves_helper(PAWN_CAPTURE_FEN);
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
        let mgen = MoveGenerator::default();
        let moves = mgen.get_moves(&b);
        let expected_moves = vec![
            Move::normal(Square::E6, Square::D6),
            Move::normal(Square::E6, Square::F7),
            Move::normal(Square::E6, Square::E7),
            Move::normal(Square::F6, Square::G4),
        ];
        for m in moves.iter() {
            println!("has {m}");
            assert!(expected_moves.contains(m));
        }
        for em in expected_moves.iter() {
            println!("expect {em}");
            assert!(moves.contains(em));
        }
    }

    #[test]
    ///
    /// Test that Black can promote a piece (on e1).
    ///
    fn test_black_can_promote() {
        let b = Board::from_fen("8/8/5k2/3K4/8/8/4p3/8 b - - 0 1").unwrap();
        let mgen = MoveGenerator::default();
        let moves = mgen.get_moves(&b);
        for m in moves.iter() {
            print!("{m}, ")
        }
        assert!(moves.contains(&Move::promoting(Square::E2, Square::E1, Piece::Queen)));
    }

    #[test]
    ///
    /// Test that a pawn cannot en passant if doing so would put the king in
    /// check.
    ///
    fn test_en_passant_pinned() {
        let b = Board::from_fen("8/2p5/3p4/KPr5/2R1Pp1k/8/6P1/8 b - e3 0 2").unwrap();
        let mgen = MoveGenerator::default();
        let moves = mgen.get_moves(&b);
        assert!(!moves.contains(&Move::normal(Square::F4, Square::E3)));
    }

    #[test]
    ///
    /// Test that a pinned piece cannot make a capture if it does not defend
    /// against the pin.
    ///
    fn test_pinned_knight_capture() {
        let b = Board::from_fen("r2q1b1r/ppp2kpp/2n5/3npb2/2B5/2N5/PPPP1PPP/R1BQ1RK1 b - - 3 8")
            .unwrap();
        let illegal_move = Move::normal(Square::D5, Square::C3);
        let mgen = MoveGenerator::default();

        assert!(!mgen.get_moves(&b).contains(&illegal_move));
        assert!(!mgen.get_loud_moves(&b).contains(&illegal_move));
    }

    #[test]
    ///
    /// Test that en passant moves are generated correctly.
    ///
    fn test_en_passant_generated() {
        let b = Board::from_fen(EN_PASSANT_READY_FEN).unwrap();
        let mgen = MoveGenerator::default();
        let m = Move::normal(Square::E5, Square::F6);

        assert!(mgen.get_moves(&b).contains(&m));
        assert!(mgen.get_loud_moves(&b).contains(&m));
    }

    #[test]
    ///
    /// Test that a position where a rook is horizontal to the king is mate.
    ///
    fn test_horizontal_rook_mate() {
        let b = Board::from_fen("r1b2k1R/3n1p2/p7/3P4/6Qp/2P3b1/6P1/4R2K b - - 0 32").unwrap();
        let mgen = MoveGenerator::default();

        assert!(mgen.get_moves(&b).is_empty());
        assert!(!mgen.has_moves(&b));
    }

    ///
    /// A helper function that will force that the given FEN will have loud
    /// moves generated correctly.
    ///
    fn loud_moves_helper(fen: &str) {
        let b = Board::from_fen(fen).unwrap();
        let mgen = MoveGenerator::default();

        let moves = mgen.get_moves(&b);
        let loud_moves = mgen.get_loud_moves(&b);

        for loud_move in loud_moves.iter() {
            println!("{loud_move}");
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
