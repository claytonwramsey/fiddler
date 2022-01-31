#[allow(unused)]
///
/// The FEN of the official starting position for any chess game.
///
pub const BOARD_START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[allow(unused)]
///
/// A board with a black king on H8 and white king on A1, and nothing else.
/// White to  move.
///
pub const TWO_KINGS_BOARD_FEN: &str = "7k/8/8/8/8/8/8/K7 w - - 0 1";

#[allow(unused)]
///
/// A board where White can play exf6 as en passant.
///
pub const EN_PASSANT_READY_FEN: &str =
    "rnbqkb1r/ppppp1pp/7n/4Pp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3";

#[allow(unused)]
///
/// A board where White is ready to castle on the kingside.
///
pub const WHITE_KINGSIDE_CASTLE_READY_FEN: &str =
    "r1bqk1nr/pppp1ppp/2n5/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4";

#[allow(unused)]
///
/// A board where White is ready to promote the f7-pawn. Also, the position is
/// mate in 7.
///
pub const WHITE_READY_TO_PROMOTE_FEN: &str = "8/5P2/2k5/4K3/8/8/8/8 w - - 0 1";

#[allow(unused)]
///
/// A board where White has played the Fried Liver Attack and is ready to bring
/// his king. The only winning move is Qf3+.
///
pub const FRIED_LIVER_FEN: &str = "r1bq1b1r/ppp2kpp/2n5/3np3/2B5/8/PPPP1PPP/RNBQK2R w KQ - 0 7";

#[allow(unused)]
///
/// A board where White can mate in 1 with Rb8#
///
pub const MATE_IN_1_FEN: &str = "3k4/R7/1R6/5K2/8/8/8/8 w - - 0 1";

#[allow(unused)]
///
/// A board where White can mate in 4 plies with ...Kc8 Rg7 Kb8 Rd8# and ...Ke8
/// Rd6 Kf8 Rd8#
///
pub const MATE_IN_4_FEN: &str = "3k4/R7/8/5K2/3R4/8/8/8 b - - 0 1";

#[allow(unused)]
///
/// A very special puzzle that Ian wrote for me. The only winning move is Rxf7
/// +, yielding mate in 5.
///
pub const MY_PUZZLE_FEN: &str = "2r2r2/3p1p1k/p3p1p1/3P3n/q3P1Q1/1p5P/1PP2R2/1K4R1 w - - 0 30";

#[allow(unused)]
///
/// The position in a game immediately after White has pulled off Scholar's
/// Mate.
///
pub const SCHOLARS_MATE_FEN: &str =
    "rnbqk2r/pppp1Qpp/5n2/2b1p3/2B1P3/8/PPPP1PPP/RNB1K1NR b KQkq - 0 4";

#[allow(unused)]
///
/// White is ready to capture the pawn on f5 with exf5
///
pub const PAWN_CAPTURE_FEN: &str = "rnbqkbnr/ppppp1pp/8/5p2/4P3/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 2";

#[allow(unused)]
///
/// A board where a pawn is checking a king.
///
pub const PAWN_CHECKING_KING_FEN: &str =
    "r1bq1b1r/ppp2kpp/2n5/3n4/2B5/8/PPP1pPPP/RN1Q1K1R w - - 0 10";

#[allow(unused)]
///
/// A board where the black queen on E2 has mated White's king.
///
pub const WHITE_MATED_FEN: &str = "r1b2b1r/ppp2kpp/8/4p3/3n4/2Q5/PP1PqPPP/RNB1K2R w KQ - 4 11";

#[allow(unused)]
///
/// A FEN where the Black king has only one legal move (Kc8).
///
pub const KING_HAS_ONE_MOVE_FEN: &str = "2k5/4R3/8/5K2/3R4/8/8/8 b - - 2 2";

#[allow(unused)]
///
/// A FEN where Black is ready to castle queenside.
///
pub const BLACKQUEENSIDE_CASTLE_READY_FEN: &str =
    "r3kb1r/ppp1p1pp/2nq1n2/1B1p4/3P4/2N2Q2/PPP2PPP/R1B1K2R b KQkq - 0 8";

#[allow(unused)]
///
/// A FEN where Black's rook on H8 is hanging, but if it isn't taken, Black can
/// castle kingside.
///
pub const ROOK_HANGING_FEN: &str =
    "rnbqk2r/ppppnp1p/4p1pb/8/4P3/1P1P4/PBP2PPP/RN1QKBNR w KQkq - 1 5";

#[allow(unused)]
///
/// A FEN where the only legal moves are non-capturing king moves and one
/// knight block.
///
pub const KING_MUST_ESCAPE_FEN: &str =
    "r2q1b1r/ppp3pp/2n1kn2/4p3/8/2N4Q/PPPP1PPP/R1B1K2R b KQ - 1 10";

#[allow(unused)]
///
/// A FEN where Black cannot play O-O-O on this move because the knight is in
/// the way.
///
pub const KNIGHT_PREVENTS_LONG_CASTLE_FEN: &str =
    "rn2kbnr/ppp1pppp/3q4/3p4/6b1/8/PPPPPPPP/RNBQKBNR b KQkq - 5 4";
