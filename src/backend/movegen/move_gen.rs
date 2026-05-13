use crate::backend::caches::{
    BETWEEN_TABLE, BISHOP_PEXT_INDEX, BISHOP_PEXT_MASK, BISHOP_XRAY_PEXT_INDEX,
    BISHOP_XRAY_PEXT_MASK, KING_MOVES, KNIGHT_MOVES, PAWN_CAPTURE_MOVES, PEXT_TABLE,
    PEXT_XRAY_TABLE, ROOK_PEXT_INDEX, ROOK_PEXT_MASK, ROOK_XRAY_PEXT_INDEX, ROOK_XRAY_PEXT_MASK,
};
use crate::backend::constants::{
    C1, C8, D1, D8, E1, E8, F1, F8, G1, G8, LEFT_SIDE_BB, RIGHT_SIDE_BB, SIDE_LENGTH,
};
use crate::backend::game_state::state::State;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::{CastleType, Moove};
use crate::backend::types::piece::Piece::*;
use crate::backend::types::piece::{ALL_PIECES, PROMOTABLE_PIECES, Piece, Side};
use crate::backend::types::square::{
    Square, back_by_one, get_file, get_rank, square_from_rank_and_file,
};
use std::arch::x86_64::_pext_u64;

const INITIAL_MOVE_CAPACITY: usize = 50;
const DOUBLE_CHECK_ATTACKERS: u32 = 2;
const NO_CHECK_ATTACKERS: u32 = 0;

// Masks for castle move generation.
// Made these values with: https://tearth.dev/bitboard-viewer/
const WHITE_LONG_CASTLE_MASK: BitBoard = BitBoard { value: 0xe };
const WHITE_SHORT_CASTLE_MASK: BitBoard = BitBoard { value: 0x60 };
const BLACK_LONG_CASTLE_MASK: BitBoard = BitBoard {
    value: 0xe00000000000000,
};
const BLACK_SHORT_CASTLE_MASK: BitBoard = BitBoard {
    value: 0x6000000000000000,
};

const WHITE_LONG_CASTLE_MOVE: Moove = Moove::new(E1, C1);
const WHITE_SHORT_CASTLE_MOVE: Moove = Moove::new(E1, G1);
const BLACK_LONG_CASTLE_MOVE: Moove = Moove::new(E8, C8);
const BLACK_SHORT_CASTLE_MOVE: Moove = Moove::new(E8, G8);

const WHITE_LONG_CASTLE_CHECK_SQUARES: [Square; 3] = [E1, D1, C1];
const WHITE_SHORT_CASTLE_CHECK_SQUARES: [Square; 3] = [E1, F1, G1];
const BLACK_LONG_CASTLE_CHECK_SQUARES: [Square; 3] = [E8, D8, C8];
const BLACK_SHORT_CASTLE_CHECK_SQUARES: [Square; 3] = [E8, F8, G8];

// Masks for pawn move generation:
// Made with https://tearth.dev/bitboard-viewer/
const BLACK_PROMOTION_RANK_BB: BitBoard = BitBoard { value: 0xff };
const WHITE_PROMOTION_RANK_BB: BitBoard = BitBoard {
    value: 0xff00000000000000,
};
const WHITE_PAWN_START_RANK_BB: BitBoard = BitBoard { value: 0xff00 };
const BLACK_PAWN_START_RANK_BB: BitBoard = BitBoard {
    value: 0xff000000000000,
};
const PROMOTION_RANKS_BB: BitBoard = BitBoard {
    value: (BLACK_PROMOTION_RANK_BB.value | WHITE_PROMOTION_RANK_BB.value),
};

const WHITE_DOUBLE_PUSH_BB: BitBoard = BitBoard { value: 0xff000000 };
const BLACK_DOUBLE_PUSH_BB: BitBoard = BitBoard {
    value: 0xff00000000,
};

// For check detection.
const CHECKING_PIECES_WITHOUT_KING: [Piece; 5] = [Rook, Knight, Bishop, Queen, Pawn];

pub struct MoveGenerator {
    moves: Vec<Moove>,

    check_mask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
}

// ================================================= //
// MoveGenerator Core
// ================================================= //
impl MoveGenerator {
    pub fn new() -> Self {
        MoveGenerator {
            moves: Vec::with_capacity(0),
            check_mask: BitBoard::new(),
            straight_pin_mask: BitBoard::new(),
            diag_pin_mask: BitBoard::new(),
        }
    }

    /// Generates and returns all legal moves for the current player's pieces
    /// based on the provided game state.
    pub fn generate_moves(mut self, state: &mut State) -> Vec<Moove> {
        let checking_squares = self.checkers(state);
        let checking_piece_count = checking_squares.value.count_ones();
        let is_double_check = checking_piece_count == DOUBLE_CHECK_ATTACKERS;
        let is_not_in_check = checking_piece_count == NO_CHECK_ATTACKERS;

        self.moves = Vec::with_capacity(INITIAL_MOVE_CAPACITY);

        self.gen_king_moves(state);

        if is_double_check {
            return self.moves;
        }

        let king_square = state
            .bb_mngr
            .get_colored_piece_bb(King, state.active_side)
            .next()
            .unwrap();

        self.check_mask = self.build_check_mask(checking_squares, king_square, is_not_in_check);

        // Calc Pin Masks.
        let (straight_pin_mask, diag_pin_mask) = self.build_pin_masks(state, king_square);
        self.straight_pin_mask = straight_pin_mask;
        self.diag_pin_mask = diag_pin_mask;

        self.remove_illegal_en_passant_if_pinned(state);

        self.gen_knight_moves(state);

        if is_not_in_check {
            self.gen_castles(state, state.bb_mngr.get_occupied_bb());
        }

        self.gen_pawn_moves(state);

        self.gen_bishop_and_queen_moves(state);

        self.gen_rook_and_queen_moves(state);

        self.moves
    }

    /// Checking squares are all squares between the king and the attacker, including the attacker.
    /// They are used as a mask for legal move-gen.
    fn build_check_mask(
        &self,
        checking_squares: BitBoard,
        king_square: Square,
        is_not_in_check: bool,
    ) -> BitBoard {
        if is_not_in_check {
            return BitBoard { value: u64::MAX };
        }

        let mut check_mask = BitBoard { value: 0 };

        for checking_square in checking_squares {
            check_mask |= BETWEEN_TABLE[checking_square as usize][king_square as usize];
        }

        check_mask
    }

    /// Pin masks are all squares between the king and the attacker,
    /// with the assumption that attackers can move through the first piece.
    fn build_pin_masks(&self, state: &State, king_square: Square) -> (BitBoard, BitBoard) {
        let enemy_bb = state.bb_mngr.get_side_bb(state.active_side.oppo());

        let straight_xray_bb = self.get_slider_xray_moves_at_square::<true>(state, king_square);
        let diag_xray_bb = self.get_slider_xray_moves_at_square::<false>(state, king_square);

        let straight_xray_attackers_bb = straight_xray_bb
            & (state.bb_mngr.get_piece_bb(Rook) | state.bb_mngr.get_piece_bb(Queen))
            & enemy_bb;

        let diag_xray_attackers_bb = diag_xray_bb
            & (state.bb_mngr.get_piece_bb(Bishop) | state.bb_mngr.get_piece_bb(Queen))
            & enemy_bb;

        (
            self.build_pin_mask(straight_xray_attackers_bb, king_square),
            self.build_pin_mask(diag_xray_attackers_bb, king_square),
        )
    }

    /// Helper function for above :)
    fn build_pin_mask(&self, xray_attackers_bb: BitBoard, king_square: Square) -> BitBoard {
        let mut pin_mask = BitBoard { value: 0 };

        for attacker_square in xray_attackers_bb {
            // The order of indices is important:
            // the attacker square is included, while the king square is not.
            // This keeps capturing the pinning piece legal.
            pin_mask |= BETWEEN_TABLE[attacker_square as usize][king_square as usize];
        }

        pin_mask
    }

    /// An EP capturable pawn shall not be captured if its pinned diagonally.
    /// This function thus removed the EP square in that case.
    fn remove_illegal_en_passant_if_pinned(&self, state: &mut State) {
        let Some(en_passant_square) = state.irreversible_data.en_passant_square else {
            return;
        };

        let captured_pawn_square = back_by_one(en_passant_square, state.active_side);
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        if (captured_pawn_bb & self.diag_pin_mask).is_not_empty() {
            state.irreversible_data.en_passant_square = None;
        }
    }

    /// Takes a square and a bitboard and appends all corresponding moves to the list.
    pub fn convert_bitboard_to_moves(&mut self, from_square: Square, moves_bitboard: BitBoard) {
        for to_square in moves_bitboard {
            self.moves.push(Moove::new(from_square, to_square));
        }
    }
}

// ================================================= //
// MoveGenerator King
// ================================================= //
impl MoveGenerator {
    fn gen_king_moves(&mut self, state: &mut State) {
        let king_square = state
            .bb_mngr
            .get_colored_piece_bb(King, state.active_side)
            .next()
            .unwrap();

        let friendly_pieces_bb = state.bb_mngr.get_side_bb(state.active_side);
        let legal_targets_bb = KING_MOVES[king_square as usize] & !friendly_pieces_bb;

        // Temporarily remove the king during the legality check.
        state
            .bb_mngr
            .clear_square(king_square, King, state.active_side);

        for to_square in legal_targets_bb {
            if !self.is_in_check_on_square(state, state.active_side, to_square) {
                self.moves.push(Moove::new(king_square, to_square));
            }
        }

        // Re-add it.
        state
            .bb_mngr
            .fill_square(king_square, King, state.active_side);
    }

    pub fn gen_castles(&mut self, state: &State, combined_bb: BitBoard) {
        for castle_type in CastleType::get_all_types() {
            // Big match :)
            let (castling_rights, squares_the_king_moves_through, between_king_rook_bb, moove) =
                match castle_type {
                    CastleType::Long => match state.active_side {
                        Side::White => (
                            state
                                .irreversible_data
                                .get_long_castle_rights(state.active_side),
                            WHITE_LONG_CASTLE_CHECK_SQUARES,
                            WHITE_LONG_CASTLE_MASK,
                            WHITE_LONG_CASTLE_MOVE,
                        ),
                        Side::Black => (
                            state
                                .irreversible_data
                                .get_long_castle_rights(state.active_side),
                            BLACK_LONG_CASTLE_CHECK_SQUARES,
                            BLACK_LONG_CASTLE_MASK,
                            BLACK_LONG_CASTLE_MOVE,
                        ),
                    },
                    CastleType::Short => match state.active_side {
                        Side::White => (
                            state
                                .irreversible_data
                                .get_short_castle_rights(state.active_side),
                            WHITE_SHORT_CASTLE_CHECK_SQUARES,
                            WHITE_SHORT_CASTLE_MASK,
                            WHITE_SHORT_CASTLE_MOVE,
                        ),
                        Side::Black => (
                            state
                                .irreversible_data
                                .get_short_castle_rights(state.active_side),
                            BLACK_SHORT_CASTLE_CHECK_SQUARES,
                            BLACK_SHORT_CASTLE_MASK,
                            BLACK_SHORT_CASTLE_MOVE,
                        ),
                    },
                };
            // do we have castling rights for this type of castle?
            if !castling_rights {
                return;
            }

            // are we moving through checks?
            for square in squares_the_king_moves_through.iter() {
                // if so -> stop
                if self.is_in_check_on_square(state, state.active_side, *square) {
                    return;
                }
            }

            // are the squares between the king and the rook empty?
            // TODO: if we had attack bbs, we could also and them here
            let squares_between = combined_bb & between_king_rook_bb;
            // if something is in the way -> stop
            if !squares_between.is_empty() {
                return;
            }

            self.moves.push(moove);
        }
    }
}

// ================================================= //
// MoveGenerator Knight
// ================================================= //
impl MoveGenerator {
    fn gen_knight_moves(&mut self, state: &mut State) {
        let friendly_pieces_bb = state.bb_mngr.get_side_bb(state.active_side);
        let knights = state
            .bb_mngr
            .get_colored_piece_bb(Knight, state.active_side)
            & !(self.straight_pin_mask | self.diag_pin_mask);

        for from_square in knights {
            let legal_targets_bb =
                KNIGHT_MOVES[from_square as usize] & self.check_mask & !friendly_pieces_bb;

            self.convert_bitboard_to_moves(from_square, legal_targets_bb);
        }
    }
}

// ================================================= //
// MoveGenerator Slider
// ================================================= //
impl MoveGenerator {
    fn gen_bishop_and_queen_moves(&mut self, state: &State) {
        let bishop_like_pieces_bb = state
            .bb_mngr
            .get_colored_piece_bb(Bishop, state.active_side)
            | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

        self.get_slider_moves(
            state,
            Bishop,
            bishop_like_pieces_bb & !self.straight_pin_mask,
            self.diag_pin_mask,
        );
    }

    fn gen_rook_and_queen_moves(&mut self, state: &State) {
        let rook_like_pieces_bb = state.bb_mngr.get_colored_piece_bb(Rook, state.active_side)
            | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

        self.get_slider_moves(
            state,
            Rook,
            rook_like_pieces_bb & !self.diag_pin_mask,
            self.straight_pin_mask,
        );
    }

    pub fn get_slider_moves(
        &mut self,
        state: &State,
        piece_type: Piece,
        piece_bb: BitBoard,
        pin_mask: BitBoard,
    ) {
        let unpinned_pieces = piece_bb & !pin_mask;
        let pinned_pieces = piece_bb & pin_mask;

        self.append_slider_moves_for_squares(state, piece_type, unpinned_pieces, self.check_mask);

        self.append_slider_moves_for_squares(
            state,
            piece_type,
            pinned_pieces,
            self.check_mask & pin_mask,
        );
    }

    fn append_slider_moves_for_squares(
        &mut self,
        state: &State,
        piece_type: Piece,
        piece_squares: BitBoard,
        legal_move_mask: BitBoard,
    ) {
        for square in piece_squares {
            let moves_for_piece_bb =
                self.slider_attacks_for_piece(state, piece_type, square) & legal_move_mask;

            self.convert_bitboard_to_moves(square, moves_for_piece_bb);
        }
    }

    fn slider_attacks_for_piece(
        &self,
        state: &State,
        piece_type: Piece,
        square: Square,
    ) -> BitBoard {
        let friendly_bb = state.bb_mngr.get_side_bb(state.active_side);
        match piece_type {
            Rook => self.get_slider_moves_at_square::<true>(state, square, friendly_bb),
            Bishop => self.get_slider_moves_at_square::<false>(state, square, friendly_bb),
            _ => unreachable!("slider move generation was called for a non-slider piece"),
        }
    }

    pub fn get_slider_xray_moves_at_square<const IS_STRAIGHT: bool>(
        &self,
        state: &State,
        square: Square,
    ) -> BitBoard {
        let square_index = square as usize;

        let pext_mask = if IS_STRAIGHT {
            ROOK_XRAY_PEXT_MASK[square_index]
        } else {
            BISHOP_XRAY_PEXT_MASK[square_index]
        };

        let pext_index = if IS_STRAIGHT {
            ROOK_XRAY_PEXT_INDEX[square_index]
        } else {
            BISHOP_XRAY_PEXT_INDEX[square_index]
        };

        let occ_bb = state.bb_mngr.get_occupied_bb();
        self.pext_table_lookup(&PEXT_XRAY_TABLE, pext_index, pext_mask, occ_bb)
    }

    /// Computes the sliding piece moves, either rook-like or bishop-like, for a
    /// given square based on occupancy bitboards.
    ///
    /// # Type Parameters
    /// - `IS_STRAIGHT`:
    ///   - `true` for rook-like horizontal and vertical moves.
    ///   - `false` for bishop-like diagonal moves.
    pub fn get_slider_moves_at_square<const IS_STRAIGHT: bool>(
        &self,
        state: &State,
        square: Square,
        friendly_bb: BitBoard,
    ) -> BitBoard {
        let square_index = square as usize;

        let pext_mask = if IS_STRAIGHT {
            ROOK_PEXT_MASK[square_index]
        } else {
            BISHOP_PEXT_MASK[square_index]
        };

        let pext_index = if IS_STRAIGHT {
            ROOK_PEXT_INDEX[square_index]
        } else {
            BISHOP_PEXT_INDEX[square_index]
        };

        let occupied_bb = state.bb_mngr.get_occupied_bb();

        self.pext_table_lookup(&PEXT_TABLE, pext_index, pext_mask, occupied_bb) & !friendly_bb
    }

    fn pext_table_lookup(
        &self,
        table: &[BitBoard],
        pext_index: usize,
        pext_mask: BitBoard,
        occupied_bb: BitBoard,
    ) -> BitBoard {
        let blockers_index = unsafe { _pext_u64(occupied_bb.value, pext_mask.value) as usize };

        table[pext_index + blockers_index]
    }
}

// ================================================= //
// MoveGenerator Pawn
// ================================================= //

impl MoveGenerator {
    pub fn gen_pawn_moves(&mut self, state: &State) {
        let friendly_pieces_bb = state.bb_mngr.get_side_bb(state.active_side);
        let enemy_pieces_bb = state.bb_mngr.get_side_bb(state.active_side.oppo());
        let occupancy_bb = friendly_pieces_bb | enemy_pieces_bb;
        let pawn_bb = state.bb_mngr.get_colored_piece_bb(Pawn, state.active_side);

        let rank_offset = match state.active_side {
            Side::White => -1,
            Side::Black => 1,
        };

        // single push
        self.single_push(state, pawn_bb & !self.diag_pin_mask, rank_offset);

        // double push
        self.double_push(state, pawn_bb & !self.diag_pin_mask, rank_offset);

        let mut possible_captures_bb = enemy_pieces_bb;
        let mut capture_checkmask = self.check_mask;
        if self.ep_edgecase_check(state) {
            let ep_square = state.irreversible_data.en_passant_square.unwrap();

            // Add ep square to possible captures
            possible_captures_bb.fill_square(ep_square);

            // This is needed for situation where taking the ep pawn removes the check:
            // 8/8/8/1Ppp3r/RK3p1k/8/4P1P1/8 w - c6 0 1
            let ep_pawn_square = match state.active_side {
                Side::White => ep_square - SIDE_LENGTH as u8,
                Side::Black => ep_square + SIDE_LENGTH as u8,
            };
            let ep_pawn_square = BitBoard::new_from_square(ep_pawn_square);
            if (ep_pawn_square & self.check_mask).is_not_empty() {
                capture_checkmask.fill_square(ep_square);
            }
        }

        // left captures
        let shift = match state.active_side {
            Side::White => 7,
            Side::Black => -9,
        };
        self.one_dir_capture(
            state,
            pawn_bb & !LEFT_SIDE_BB & !self.straight_pin_mask,
            rank_offset,
            shift,
            1,
        );

        // right captures
        let shift = match state.active_side {
            Side::White => 9,
            Side::Black => -7,
        };
        self.one_dir_capture(
            state,
            pawn_bb & !RIGHT_SIDE_BB & !self.straight_pin_mask,
            rank_offset,
            shift,
            -1,
        );
    }
    fn ep_edgecase_check(&mut self, state: &State) -> bool {
        let Some(en_passant_square) = state.irreversible_data.en_passant_square else {
            return false;
        };

        let active_side = state.active_side;
        let opponent_side = active_side.oppo();

        let double_push_rank = match active_side {
            Side::White => BLACK_DOUBLE_PUSH_BB,
            Side::Black => WHITE_DOUBLE_PUSH_BB,
        };

        let captured_pawn_square = match active_side {
            Side::White => en_passant_square - SIDE_LENGTH as u8,
            Side::Black => en_passant_square + SIDE_LENGTH as u8,
        };
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        let friendly_king = state.bb_mngr.get_colored_piece_bb(King, active_side);
        let opponent_sliders = state.bb_mngr.get_colored_piece_bb(Queen, opponent_side)
            | state.bb_mngr.get_colored_piece_bb(Rook, opponent_side);

        if (friendly_king & double_push_rank).is_empty()
            || (opponent_sliders & double_push_rank).is_empty()
        {
            return true;
        }

        let friendly_pawns = state.bb_mngr.get_colored_piece_bb(Pawn, active_side);
        let left_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !LEFT_SIDE_BB) >> 1);
        let right_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !RIGHT_SIDE_BB) << 1);

        let friendly_occupancy = state.bb_mngr.get_side_bb(active_side);
        let opponent_occupancy = state.bb_mngr.get_side_bb(opponent_side) & !captured_pawn_bb;

        let king_square = friendly_king.clone().next().unwrap();

        let would_expose_king_to_slider = |capturing_pawn: BitBoard| {
            if capturing_pawn.is_empty() {
                return false;
            }

            let king_slider_rays = self.get_slider_moves_at_square::<true>(
                state,
                king_square,
                friendly_occupancy & !capturing_pawn,
            );

            (king_slider_rays & opponent_sliders).is_not_empty()
        };

        !would_expose_king_to_slider(left_capturing_pawn)
            && !would_expose_king_to_slider(right_capturing_pawn)
    }
    fn single_push(&mut self, state: &State, pawn_bb: BitBoard, rank_offset: i8) {
        let mut push_pawn_bb = match state.active_side {
            Side::White => (pawn_bb & !self.straight_pin_mask) << 8,
            Side::Black => (pawn_bb & !self.straight_pin_mask) >> 8,
        };
        let occupancy_bb = state.bb_mngr.get_occupied_bb();
        // cant go there if something is there or if the checkmask forbids it
        push_pawn_bb &= !occupancy_bb & self.check_mask;
        self.pawn_bb_to_moves_no_promotion(push_pawn_bb & !PROMOTION_RANKS_BB, 0, rank_offset);
        self.pawn_bb_to_moves_promotion(push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);

        let mut push_pawn_bb = match state.active_side {
            Side::White => (pawn_bb & self.straight_pin_mask) << 8,
            Side::Black => (pawn_bb & self.straight_pin_mask) >> 8,
        };
        // cant go there if something is there or if the checkmask forbids it
        push_pawn_bb &= !occupancy_bb & self.check_mask & self.straight_pin_mask;
        self.pawn_bb_to_moves_no_promotion(push_pawn_bb & !PROMOTION_RANKS_BB, 0, rank_offset);
        self.pawn_bb_to_moves_promotion(push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);
    }

    fn double_push(&mut self, state: &State, pawn_bb: BitBoard, rank_offset: i8) {
        let occupancy_bb = state.bb_mngr.get_occupied_bb();
        let double_push_bb = match state.active_side {
            Side::White => {
                (((pawn_bb & WHITE_PAWN_START_RANK_BB & !self.straight_pin_mask) << 8)
                    & !occupancy_bb)
                    << 8
                    & !occupancy_bb
            }
            Side::Black => {
                (((pawn_bb & BLACK_PAWN_START_RANK_BB & !self.straight_pin_mask) >> 8)
                    & !occupancy_bb)
                    >> 8
                    & !occupancy_bb
            }
        };
        self.pawn_bb_to_moves_no_promotion(double_push_bb & self.check_mask, 0, 2 * rank_offset);

        let double_push_bb = match state.active_side {
            Side::White => {
                (((pawn_bb & WHITE_PAWN_START_RANK_BB & self.straight_pin_mask) << 8)
                    & !occupancy_bb)
                    << 8
                    & !occupancy_bb
            }
            Side::Black => {
                (((pawn_bb & BLACK_PAWN_START_RANK_BB & self.straight_pin_mask) >> 8)
                    & !occupancy_bb)
                    >> 8
                    & !occupancy_bb
            }
        };
        self.pawn_bb_to_moves_no_promotion(
            double_push_bb & self.check_mask & self.straight_pin_mask,
            0,
            2 * rank_offset,
        );
    }

    fn one_dir_capture(
        &mut self,
        state: &State,
        pawn_bb: BitBoard,
        rank_offset: i8,
        shift: i32,
        file_offset: i8,
    ) {
        let free_pawns: BitBoard = match shift.is_negative() {
            true => (pawn_bb & !self.diag_pin_mask) >> shift.unsigned_abs() as i32,
            false => (pawn_bb & !self.diag_pin_mask) << shift,
        };
        let enemy_pieces_bb = state.bb_mngr.get_side_bb(state.active_side.oppo());

        let capture_bb = free_pawns & enemy_pieces_bb & self.check_mask;

        self.pawn_bb_to_moves_no_promotion(
            capture_bb & !PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );
        self.pawn_bb_to_moves_promotion(capture_bb & PROMOTION_RANKS_BB, file_offset, rank_offset);

        let free_pawns: BitBoard = match shift.is_negative() {
            true => (pawn_bb & self.diag_pin_mask) >> shift.unsigned_abs() as i32,
            false => (pawn_bb & self.diag_pin_mask) << shift,
        };
        let capture_bb = free_pawns & enemy_pieces_bb & self.check_mask & self.diag_pin_mask;
        self.pawn_bb_to_moves_no_promotion(
            capture_bb & !PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );
        self.pawn_bb_to_moves_promotion(capture_bb & PROMOTION_RANKS_BB, file_offset, rank_offset);
    }

    fn pawn_bb_to_moves_no_promotion(
        &mut self,
        pawn_bb: BitBoard,
        file_offset: i8,
        rank_offset: i8,
    ) {
        for square in pawn_bb {
            let from_square = (square as i8 + 8 * rank_offset + file_offset) as Square;
            let moove = Moove::new(from_square, square);
            self.moves.push(moove);
        }
    }

    fn pawn_bb_to_moves_promotion(&mut self, pawn_bb: BitBoard, file_offset: i8, rank_offset: i8) {
        for square in pawn_bb {
            let file = get_file(square);
            let rank = get_rank(square);
            let offset_square = square_from_rank_and_file(rank + rank_offset, file + file_offset);
            for piece_type in PROMOTABLE_PIECES {
                let moove = Moove::new_promotion(offset_square, square, piece_type);
                self.moves.push(moove);
            }
        }
    }
}

impl MoveGenerator {
    // Idea:
    // If, for example, side == white, we want to figure out if white is currently in check.
    // We then pretend that the white king is one after the other replaced by: pawn, rook, bishop, queen, king.
    // We then calculate all possible attacks for each of these pieces as a bitboard.
    // Since that's what we already do for movegen, we can just reuse that.
    // If we have the white king on A1 and a black bishop on C3:
    // We can now generate all possible attacks for a (imaginary) bishop on A1 as a bitboard.
    // We can & this bitboard with the bitboard for black bishops and realize that it is not empty.
    // Thus, we now know that the white king is in check by a black bishop.
    // I hope this makes sense :)
    pub fn is_in_check_on_square(&self, state: &State, side: Side, king_square: Square) -> bool {
        for piece_type in ALL_PIECES {
            let attackers_bb =
                self.get_attackers_of_piece_type_on_square(state, side, king_square, piece_type);

            if attackers_bb.is_not_empty() {
                return true;
            }
        }

        false
    }

    pub fn checkers(&self, state: &State) -> BitBoard {
        let side = state.active_side;
        let mut king_bb = state.bb_mngr.get_piece_bb(King) & state.bb_mngr.get_side_bb(side);
        let king_square = king_bb.next().unwrap();

        let mut checking_squares_bb = BitBoard::new();

        for piece_type in CHECKING_PIECES_WITHOUT_KING {
            checking_squares_bb |=
                self.get_attackers_of_piece_type_on_square(state, side, king_square, piece_type);
        }

        checking_squares_bb
    }

    fn get_attackers_of_piece_type_on_square(
        &self,
        state: &State,
        side: Side,
        target_square: Square,
        piece_type: Piece,
    ) -> BitBoard {
        let friendly_bb = state.bb_mngr.get_side_bb(state.active_side);

        let attacked_squares = match piece_type {
            King => KING_MOVES[target_square as usize],
            Knight => KNIGHT_MOVES[target_square as usize],
            Pawn => PAWN_CAPTURE_MOVES[side as usize][target_square as usize],
            Rook => self.get_slider_moves_at_square::<true>(state, target_square, friendly_bb),
            Bishop => self.get_slider_moves_at_square::<false>(state, target_square, friendly_bb),
            Queen => {
                self.get_slider_moves_at_square::<true>(state, target_square, friendly_bb)
                    | self.get_slider_moves_at_square::<false>(state, target_square, friendly_bb)
            }
        };

        let enemy_pieces_of_type = state.bb_mngr.get_piece_bb(piece_type)
            & state.bb_mngr.get_side_bb(state.active_side.oppo());

        attacked_squares & enemy_pieces_of_type
    }
}
