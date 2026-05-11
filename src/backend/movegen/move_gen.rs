use crate::backend::caches::{
    BETWEEN_TABLE, BISHOP_PEXT_INDEX, BISHOP_PEXT_MASK, BISHOP_XRAY_PEXT_INDEX,
    BISHOP_XRAY_PEXT_MASK, KING_MOVES, KNIGHT_MOVES, PEXT_TABLE, PEXT_XRAY_TABLE, ROOK_PEXT_INDEX,
    ROOK_PEXT_MASK, ROOK_XRAY_PEXT_INDEX, ROOK_XRAY_PEXT_MASK,
};
use crate::backend::constants::{
    C1, C8, D1, D8, E1, E8, F1, F8, G1, G8, LEFT_SIDE_BB, RIGHT_SIDE_BB, SIDE_LENGTH,
};
use crate::backend::game_state::irreversible_data::IrreversibleData;
use crate::backend::game_state::state::State;
use crate::backend::movegen::check_decider::{checkers, is_in_check_on_square};
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::{CastleType, Moove};
use crate::backend::types::piece::{Piece, PROMOTABLE_PIECES, Side};
use crate::backend::types::piece::Piece::*;
use crate::backend::types::square::{Square, back_by_one, get_file, get_rank, square_from_rank_and_file};
use std::arch::x86_64::_pext_u64;

const INITIAL_MOVE_CAPACITY: usize = 50;
const DOUBLE_CHECK_ATTACKERS: u32 = 2;
const NO_CHECK_ATTACKERS: u32 = 0;

const WHITE_PAWN_START_RANK_BB: BitBoard = BitBoard { value: 0xff00 };
const BLACK_PAWN_START_RANK_BB: BitBoard = BitBoard {
    value: 0xff000000000000,
};

const WHITE_PROMOTION_RANK_BB: BitBoard = BitBoard {
    value: 0xff00000000000000,
};
const BLACK_PROMOTION_RANK_BB: BitBoard = BitBoard { value: 0xff };

const PROMOTION_RANKS_BB: BitBoard = BitBoard {
    value: (BLACK_PROMOTION_RANK_BB.value | WHITE_PROMOTION_RANK_BB.value),
};

const WHITE_DOUBLE_PUSH_BB: BitBoard = BitBoard { value: 0xff000000 };
const BLACK_DOUBLE_PUSH_BB: BitBoard = BitBoard {
    value: 0xff00000000,
};

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

pub struct MoveGenerator<'a> {
    state: &'a mut State,
    active_side: Side,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    occupied_bb: BitBoard,
    king_square: Square,
    check_mask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
    is_double_check: bool,
    is_not_in_check: bool,
}

impl<'a> MoveGenerator<'a> {
    pub fn new(state: &'a mut State) -> Self {
        let active_side = state.active_side;
        let friendly_pieces_bb = state.bb_mngr.get_side_bb(active_side);
        let enemy_pieces_bb = state.bb_mngr.get_side_bb(active_side.oppo());
        let occupied_bb = friendly_pieces_bb | enemy_pieces_bb;

        let checking_squares = checkers(state);
        let checking_piece_count = checking_squares.value.count_ones();
        let is_double_check = checking_piece_count == DOUBLE_CHECK_ATTACKERS;
        let is_not_in_check = checking_piece_count == NO_CHECK_ATTACKERS;

        let king_square = state
            .bb_mngr
            .get_colored_piece_bb(King, active_side)
            .next()
            .unwrap();

        let check_mask = Self::build_check_mask(checking_squares, king_square, is_not_in_check);

        let (straight_pin_mask, diag_pin_mask) =
            Self::build_pin_masks(state, king_square, occupied_bb, enemy_pieces_bb);

        let mut mg = Self {
            state,
            active_side,
            friendly_pieces_bb,
            enemy_pieces_bb,
            occupied_bb,
            king_square,
            check_mask,
            straight_pin_mask,
            diag_pin_mask,
            is_double_check,
            is_not_in_check,
        };

        mg.remove_illegal_en_passant_if_pinned();

        mg
    }

    pub fn generate_moves(mut self) -> Vec<Moove> {
        let mut moves = Vec::with_capacity(INITIAL_MOVE_CAPACITY);

        self.gen_king_moves(&mut moves);

        if self.is_double_check {
            return moves;
        }

        self.gen_knight_moves(&mut moves);

        if self.is_not_in_check {
            self.gen_castles(&mut moves);
        }

        self.gen_pawn_moves(&mut moves);
        self.gen_bishop_and_queen_moves(&mut moves);
        self.gen_rook_and_queen_moves(&mut moves);

        moves
    }

    fn build_check_mask(
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

    fn build_pin_masks(
        state: &State,
        king_square: Square,
        occupied_bb: BitBoard,
        enemy_pieces_bb: BitBoard,
    ) -> (BitBoard, BitBoard) {
        let straight_xray_bb =
            Self::get_slider_xray_moves_at_square::<true>(king_square, occupied_bb);
        let diag_xray_bb = Self::get_slider_xray_moves_at_square::<false>(king_square, occupied_bb);

        let straight_xray_attackers_bb = straight_xray_bb
            & (state.bb_mngr.get_piece_bb(Rook) | state.bb_mngr.get_piece_bb(Queen))
            & enemy_pieces_bb;

        let diag_xray_attackers_bb = diag_xray_bb
            & (state.bb_mngr.get_piece_bb(Bishop) | state.bb_mngr.get_piece_bb(Queen))
            & enemy_pieces_bb;

        (
            Self::build_pin_mask(straight_xray_attackers_bb, king_square),
            Self::build_pin_mask(diag_xray_attackers_bb, king_square),
        )
    }

    fn build_pin_mask(xray_attackers_bb: BitBoard, king_square: Square) -> BitBoard {
        let mut pin_mask = BitBoard { value: 0 };

        for attacker_square in xray_attackers_bb {
            pin_mask |= BETWEEN_TABLE[attacker_square as usize][king_square as usize];
        }

        pin_mask
    }

    fn gen_king_moves(&mut self, moves: &mut Vec<Moove>) {
        let king_bb = self.state.bb_mngr.get_colored_piece_bb(King, self.active_side);

        for from_square in king_bb {
            let legal_targets_bb = KING_MOVES[from_square as usize] & !self.friendly_pieces_bb;

            self.state
                .bb_mngr
                .get_piece_bb_mut(King)
                .clear_square(from_square);
            self.state
                .bb_mngr
                .get_side_bb_mut(self.active_side)
                .clear_square(from_square);

            for to_square in legal_targets_bb {
                if !is_in_check_on_square(self.state, self.active_side, to_square) {
                    moves.push(Moove::new(from_square, to_square));
                }
            }

            self.state
                .bb_mngr
                .get_piece_bb_mut(King)
                .fill_square(from_square);
            self.state
                .bb_mngr
                .get_side_bb_mut(self.active_side)
                .fill_square(from_square);
        }
    }

    fn remove_illegal_en_passant_if_pinned(&mut self) {
        let Some(en_passant_square) = self.state.irreversible_data.en_passant_square else {
            return;
        };

        let captured_pawn_square = back_by_one(en_passant_square, self.active_side);
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        if (captured_pawn_bb & self.diag_pin_mask).is_not_empty() {
            self.state.irreversible_data.en_passant_square = None;
        }
    }

    fn gen_knight_moves(&mut self, moves: &mut Vec<Moove>) {
        let knights = self.state
            .bb_mngr
            .get_colored_piece_bb(Knight, self.active_side)
            & !(self.straight_pin_mask | self.diag_pin_mask);

        for from_square in knights {
            let legal_targets_bb =
                KNIGHT_MOVES[from_square as usize] & self.check_mask & !self.friendly_pieces_bb;

            self.convert_bitboard_to_moves(moves, from_square, legal_targets_bb);
        }
    }

    fn gen_bishop_and_queen_moves(&mut self, moves: &mut Vec<Moove>) {
        let bishop_like_pieces_bb = self.state
            .bb_mngr
            .get_colored_piece_bb(Bishop, self.active_side)
            | self.state.bb_mngr.get_colored_piece_bb(Queen, self.active_side);

        self.get_slider_moves(
            moves,
            Bishop,
            bishop_like_pieces_bb & !self.straight_pin_mask,
            self.diag_pin_mask,
        );
    }

    fn gen_rook_and_queen_moves(&mut self, moves: &mut Vec<Moove>) {
        let rook_like_pieces_bb = self.state.bb_mngr.get_colored_piece_bb(Rook, self.active_side)
            | self.state.bb_mngr.get_colored_piece_bb(Queen, self.active_side);

        self.get_slider_moves(
            moves,
            Rook,
            rook_like_pieces_bb & !self.diag_pin_mask,
            self.straight_pin_mask,
        );
    }

    fn convert_bitboard_to_moves(
        &self,
        moves: &mut Vec<Moove>,
        from_square: Square,
        moves_bitboard: BitBoard,
    ) {
        for to_square in moves_bitboard {
            moves.push(Moove::new(from_square, to_square));
        }
    }

    fn gen_castles(&self, moves: &mut Vec<Moove>) {
        let irreversible_data = &self.state.irreversible_data;

        for castle_type in CastleType::get_all_types() {
            let (castling_rights, squares_the_king_moves_through, between_king_rook_bb, moove) =
                self.get_needed_constants(irreversible_data, &castle_type, self.active_side);

            self.gen_castle(
                moves,
                castling_rights,
                squares_the_king_moves_through,
                between_king_rook_bb,
                moove,
            );
        }
    }

    fn gen_castle(
        &self,
        all_pseudo_legal_moves: &mut Vec<Moove>,
        castling_rights: bool,
        squares_the_king_moves_through: [Square; 3],
        between_king_rook_bb: BitBoard,
        moove: Moove,
    ) {
        if !castling_rights {
            return;
        }

        for square in squares_the_king_moves_through.iter() {
            if is_in_check_on_square(self.state, self.active_side, *square) {
                return;
            }
        }

        let squares_between = self.occupied_bb & between_king_rook_bb;
        if !squares_between.is_empty() {
            return;
        }

        all_pseudo_legal_moves.push(moove);
    }

    fn get_needed_constants(
        &self,
        irreversible_data: &IrreversibleData,
        castle_types: &CastleType,
        side: Side,
    ) -> (bool, [Square; 3], BitBoard, Moove) {
        match castle_types {
            CastleType::Long => match side {
                Side::White => (
                    irreversible_data.get_long_castle_rights(side),
                    WHITE_LONG_CASTLE_CHECK_SQUARES,
                    WHITE_LONG_CASTLE_MASK,
                    WHITE_LONG_CASTLE_MOVE,
                ),
                Side::Black => (
                    irreversible_data.get_long_castle_rights(side),
                    BLACK_LONG_CASTLE_CHECK_SQUARES,
                    BLACK_LONG_CASTLE_MASK,
                    BLACK_LONG_CASTLE_MOVE,
                ),
            },
            CastleType::Short => match side {
                Side::White => (
                    irreversible_data.get_short_castle_rights(side),
                    WHITE_SHORT_CASTLE_CHECK_SQUARES,
                    WHITE_SHORT_CASTLE_MASK,
                    WHITE_SHORT_CASTLE_MOVE,
                ),
                Side::Black => (
                    irreversible_data.get_short_castle_rights(side),
                    BLACK_SHORT_CASTLE_CHECK_SQUARES,
                    BLACK_SHORT_CASTLE_MASK,
                    BLACK_SHORT_CASTLE_MOVE,
                ),
            },
        }
    }

    fn gen_pawn_moves(&self, moves: &mut Vec<Moove>) {
        let pawn_bb = self.state.bb_mngr.get_colored_piece_bb(Pawn, self.active_side);

        let rank_offset = match self.active_side {
            Side::White => -1,
            Side::Black => 1,
        };

        self.single_push(
            moves,
            self.check_mask,
            self.straight_pin_mask,
            pawn_bb & !self.diag_pin_mask,
            rank_offset,
        );

        self.double_push(
            moves,
            self.check_mask,
            self.straight_pin_mask,
            pawn_bb & !self.diag_pin_mask,
            rank_offset,
        );

        let mut possible_captures_bb = self.enemy_pieces_bb;
        let mut capture_checkmask = self.check_mask;
        if self.is_ep_legal() {
            let ep_square = self.state.irreversible_data.en_passant_square.unwrap();

            possible_captures_bb.fill_square(ep_square);

            let ep_pawn_square = match self.active_side {
                Side::White => ep_square - SIDE_LENGTH as u8,
                Side::Black => ep_square + SIDE_LENGTH as u8,
            };
            let ep_pawn_bb = BitBoard::new_from_square(ep_pawn_square);
            if (ep_pawn_bb & self.check_mask).is_not_empty() {
                capture_checkmask.fill_square(ep_square);
            }
        }

        let shift = match self.active_side {
            Side::White => 7,
            Side::Black => -9,
        };
        self.one_dir_capture(
            moves,
            possible_captures_bb,
            pawn_bb & !LEFT_SIDE_BB & !self.straight_pin_mask,
            capture_checkmask,
            self.diag_pin_mask,
            rank_offset,
            shift,
            1,
        );

        let shift = match self.active_side {
            Side::White => 9,
            Side::Black => -7,
        };
        self.one_dir_capture(
            moves,
            possible_captures_bb,
            pawn_bb & !RIGHT_SIDE_BB & !self.straight_pin_mask,
            capture_checkmask,
            self.diag_pin_mask,
            rank_offset,
            shift,
            -1,
        );
    }

    fn is_ep_legal(&self) -> bool {
        let Some(en_passant_square) = self.state.irreversible_data.en_passant_square else {
            return false;
        };

        let opponent_side = self.active_side.oppo();

        let double_push_rank = match self.active_side {
            Side::White => BLACK_DOUBLE_PUSH_BB,
            Side::Black => WHITE_DOUBLE_PUSH_BB,
        };

        let captured_pawn_square = match self.active_side {
            Side::White => en_passant_square - SIDE_LENGTH as u8,
            Side::Black => en_passant_square + SIDE_LENGTH as u8,
        };
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        let friendly_king_bb = self.state.bb_mngr.get_colored_piece_bb(King, self.active_side);
        let opponent_sliders = self.state.bb_mngr.get_colored_piece_bb(Queen, opponent_side)
            | self.state.bb_mngr.get_colored_piece_bb(Rook, opponent_side);

        if (friendly_king_bb & double_push_rank).is_empty()
            || (opponent_sliders & double_push_rank).is_empty()
        {
            return true;
        }

        let friendly_pawns = self.state.bb_mngr.get_colored_piece_bb(Pawn, self.active_side);
        let left_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !LEFT_SIDE_BB) >> 1);
        let right_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !RIGHT_SIDE_BB) << 1);

        let friendly_occupancy = self.friendly_pieces_bb;
        let opponent_occupancy = self.enemy_pieces_bb & !captured_pawn_bb;

        let would_expose_king_to_slider = |capturing_pawn: BitBoard| {
            if capturing_pawn.is_empty() {
                return false;
            }

            let king_slider_rays = Self::get_slider_moves_at_square::<true>(
                self.king_square,
                friendly_occupancy & !capturing_pawn,
                opponent_occupancy,
            );

            (king_slider_rays & opponent_sliders).is_not_empty()
        };

        !would_expose_king_to_slider(left_capturing_pawn)
            && !would_expose_king_to_slider(right_capturing_pawn)
    }

    fn single_push(
        &self,
        moves: &mut Vec<Moove>,
        checkmask_bb: BitBoard,
        straight_pin_mask: BitBoard,
        pawn_bb: BitBoard,
        rank_offset: i8,
    ) {
        let mut push_pawn_bb = match self.active_side {
            Side::White => (pawn_bb & !straight_pin_mask) << 8,
            Side::Black => (pawn_bb & !straight_pin_mask) >> 8,
        };
        push_pawn_bb &= !self.occupied_bb & checkmask_bb;
        self.pawn_bb_to_moves_no_promotion(moves, push_pawn_bb & !PROMOTION_RANKS_BB, 0, rank_offset);
        self.pawn_bb_to_moves_promotion(moves, push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);

        let mut push_pawn_bb = match self.active_side {
            Side::White => (pawn_bb & straight_pin_mask) << 8,
            Side::Black => (pawn_bb & straight_pin_mask) >> 8,
        };
        push_pawn_bb &= !self.occupied_bb & checkmask_bb & straight_pin_mask;
        self.pawn_bb_to_moves_no_promotion(moves, push_pawn_bb & !PROMOTION_RANKS_BB, 0, rank_offset);
        self.pawn_bb_to_moves_promotion(moves, push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);
    }

    fn double_push(
        &self,
        moves: &mut Vec<Moove>,
        checkmask_bb: BitBoard,
        straight_pin_mask: BitBoard,
        pawn_bb: BitBoard,
        rank_offset: i8,
    ) {
        let double_push_bb = match self.active_side {
            Side::White => {
                (((pawn_bb & WHITE_PAWN_START_RANK_BB & !straight_pin_mask) << 8) & !self.occupied_bb) << 8
                    & !self.occupied_bb
            }
            Side::Black => {
                (((pawn_bb & BLACK_PAWN_START_RANK_BB & !straight_pin_mask) >> 8) & !self.occupied_bb) >> 8
                    & !self.occupied_bb
            }
        };
        self.pawn_bb_to_moves_no_promotion(moves, double_push_bb & checkmask_bb, 0, 2 * rank_offset);

        let double_push_bb = match self.active_side {
            Side::White => {
                (((pawn_bb & WHITE_PAWN_START_RANK_BB & straight_pin_mask) << 8) & !self.occupied_bb) << 8
                    & !self.occupied_bb
            }
            Side::Black => {
                (((pawn_bb & BLACK_PAWN_START_RANK_BB & straight_pin_mask) >> 8) & !self.occupied_bb) >> 8
                    & !self.occupied_bb
            }
        };
        self.pawn_bb_to_moves_no_promotion(
            moves,
            double_push_bb & checkmask_bb & straight_pin_mask,
            0,
            2 * rank_offset,
        );
    }

    fn one_dir_capture(
        &self,
        moves: &mut Vec<Moove>,
        enemy_pieces_bb: BitBoard,
        pawn_bb: BitBoard,
        checkmask: BitBoard,
        diag_pin_mask: BitBoard,
        rank_offset: i8,
        shift: i32,
        file_offset: i8,
    ) {
        let free_pawns: BitBoard = match shift.is_negative() {
            true => (pawn_bb & !diag_pin_mask) >> shift.unsigned_abs() as i32,
            false => (pawn_bb & !diag_pin_mask) << shift,
        };
        let capture_bb = free_pawns & enemy_pieces_bb & checkmask;

        self.pawn_bb_to_moves_no_promotion(
            moves,
            capture_bb & !PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );
        self.pawn_bb_to_moves_promotion(
            moves,
            capture_bb & PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );

        let free_pawns: BitBoard = match shift.is_negative() {
            true => (pawn_bb & diag_pin_mask) >> shift.unsigned_abs() as i32,
            false => (pawn_bb & diag_pin_mask) << shift,
        };
        let capture_bb = free_pawns & enemy_pieces_bb & checkmask & diag_pin_mask;
        self.pawn_bb_to_moves_no_promotion(
            moves,
            capture_bb & !PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );
        self.pawn_bb_to_moves_promotion(
            moves,
            capture_bb & PROMOTION_RANKS_BB,
            file_offset,
            rank_offset,
        );
    }

    fn pawn_bb_to_moves_no_promotion(
        &self,
        moves: &mut Vec<Moove>,
        pawn_bb: BitBoard,
        file_offset: i8,
        rank_offset: i8,
    ) {
        for square in pawn_bb {
            let from_square = (square as i8 + 8 * rank_offset + file_offset) as Square;
            let moove = Moove::new(from_square, square);
            moves.push(moove);
        }
    }

    fn pawn_bb_to_moves_promotion(
        &self,
        moves: &mut Vec<Moove>,
        pawn_bb: BitBoard,
        file_offset: i8,
        rank_offset: i8,
    ) {
        for square in pawn_bb {
            let file = get_file(square);
            let rank = get_rank(square);
            let offset_square = square_from_rank_and_file(rank + rank_offset, file + file_offset);
            for piece_type in PROMOTABLE_PIECES {
                let moove = Moove::new_promotion(offset_square, square, piece_type);
                moves.push(moove);
            }
        }
    }

    fn pext_table_lookup(
        table: &[BitBoard],
        pext_index: usize,
        pext_mask: BitBoard,
        occupied_bb: BitBoard,
    ) -> BitBoard {
        let blockers_index = unsafe { _pext_u64(occupied_bb.value, pext_mask.value) as usize };

        table[pext_index + blockers_index]
    }

    fn get_slider_moves(
        &self,
        moves: &mut Vec<Moove>,
        piece_type: Piece,
        piece_bb: BitBoard,
        pin_mask: BitBoard,
    ) {
        let unpinned_pieces = piece_bb & !pin_mask;
        let pinned_pieces = piece_bb & pin_mask;

        self.append_slider_moves_for_squares(
            moves,
            piece_type,
            unpinned_pieces,
            self.check_mask,
        );

        self.append_slider_moves_for_squares(
            moves,
            piece_type,
            pinned_pieces,
            self.check_mask & pin_mask,
        );
    }

    fn append_slider_moves_for_squares(
        &self,
        moves: &mut Vec<Moove>,
        piece_type: Piece,
        piece_squares: BitBoard,
        legal_move_mask: BitBoard,
    ) {
        for square in piece_squares {
            let moves_for_piece_bb =
                self.slider_attacks_for_piece(piece_type, square)
                    & legal_move_mask;

            self.convert_bitboard_to_moves(moves, square, moves_for_piece_bb);
        }
    }

    fn slider_attacks_for_piece(&self, piece_type: Piece, square: Square) -> BitBoard {
        match piece_type {
            Piece::Rook => {
                Self::get_slider_moves_at_square::<true>(square, self.friendly_pieces_bb, self.enemy_pieces_bb)
            }
            Piece::Bishop => {
                Self::get_slider_moves_at_square::<false>(square, self.friendly_pieces_bb, self.enemy_pieces_bb)
            }
            Piece::Queen => {
                Self::get_slider_moves_at_square::<true>(square, self.friendly_pieces_bb, self.enemy_pieces_bb)
                    | Self::get_slider_moves_at_square::<false>(square, self.friendly_pieces_bb, self.enemy_pieces_bb)
            }
            _ => unreachable!("slider move generation was called for a non-slider piece"),
        }
    }

    fn get_slider_xray_moves_at_square<const IS_STRAIGHT: bool>(
        square: Square,
        occ_bb: BitBoard,
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

        Self::pext_table_lookup(&PEXT_XRAY_TABLE, pext_index, pext_mask, occ_bb)
    }

    pub fn get_slider_moves_at_square<const IS_STRAIGHT: bool>(
        square: Square,
        friendly_bb: BitBoard,
        enemy_bb: BitBoard,
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

        let occupied_bb = friendly_bb | enemy_bb;

        Self::pext_table_lookup(&PEXT_TABLE, pext_index, pext_mask, occupied_bb) & !friendly_bb
    }
}

pub fn moves(state: &mut State) -> Vec<Moove> {
    MoveGenerator::new(state).generate_moves()
}
