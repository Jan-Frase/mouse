use std::os::linux::raw::stat;
use crate::backend::constants::{LEFT_SIDE_BB, RIGHT_SIDE_BB, SIDE_LENGTH};
use crate::backend::types::moove::Moove;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::game_state::state::State;
use crate::backend::movegen::move_gen_sliders::get_slider_moves_at_square;
use crate::backend::types::piece::Piece::{King, Pawn, Queen, Rook};
use crate::backend::types::piece::{PROMOTABLE_PIECES, Side};
use crate::backend::types::square::{Square, get_rank};
use crate::backend::types::square::{get_file, square_from_rank_and_file};

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
const BLACK_DOUBLE_PUSH_BB: BitBoard = BitBoard { value: 0xff00000000 };


pub fn gen_pawn_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    checkmask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
    active_color: Side,
) {
    let occupancy_bb = friendly_pieces_bb | enemy_pieces_bb;
    let pawn_bb = state.bb_mngr.get_colored_piece_bb(Pawn, active_color);

    let rank_offset = match active_color {
        Side::White => -1,
        Side::Black => 1,
    };

    // single push
    single_push(moves, active_color, occupancy_bb, checkmask, straight_pin_mask, pawn_bb & !diag_pin_mask, rank_offset);

    // double push
    double_push(moves, active_color, occupancy_bb, checkmask, straight_pin_mask, pawn_bb & !diag_pin_mask, rank_offset);

    let mut possible_captures_bb = enemy_pieces_bb;
    let mut capture_checkmask = checkmask;
    if is_ep_legal(state) {
        let ep_square = state.irreversible_data.en_passant_square.unwrap();

        // Add ep square to possible captures
        possible_captures_bb.fill_square(ep_square);

        // This is needed for situation where taking the ep pawn removes the check:
        // 8/8/8/1Ppp3r/RK3p1k/8/4P1P1/8 w - c6 0 1
        let ep_pawn_square = match state.active_side {
            Side::White => ep_square - SIDE_LENGTH as u8,
            Side::Black => ep_square + SIDE_LENGTH as u8
        };
        let ep_pawn_square = BitBoard::new_from_square(ep_pawn_square);
        if (ep_pawn_square & checkmask).is_not_empty() {
            capture_checkmask.fill_square(ep_square);
        }
    }

    // left captures
    let shift = match active_color {
        Side::White => 7,
        Side::Black => -9,
    };
    one_dir_capture(
        moves,
        possible_captures_bb,
        pawn_bb & !LEFT_SIDE_BB & !straight_pin_mask,
        capture_checkmask,
        diag_pin_mask,
        rank_offset,
        shift,
        1,
    );

    // right captures
    let shift = match active_color {
        Side::White => 9,
        Side::Black => -7,
    };
    one_dir_capture(
        moves,
        possible_captures_bb,
        pawn_bb & !RIGHT_SIDE_BB & !straight_pin_mask,
        capture_checkmask,
        diag_pin_mask,
        rank_offset,
        shift,
        -1,
    );
}
fn is_ep_legal(state: &State) -> bool {
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
    let opponent_sliders =
        state.bb_mngr.get_colored_piece_bb(Queen, opponent_side)
            | state.bb_mngr.get_colored_piece_bb(Rook, opponent_side);

    if (friendly_king & double_push_rank).is_empty()
        || (opponent_sliders & double_push_rank).is_empty()
    {
        return true;
    }

    let friendly_pawns = state.bb_mngr.get_colored_piece_bb(Pawn, active_side);
    let left_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !LEFT_SIDE_BB) >> 1);
    let right_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !RIGHT_SIDE_BB) << 1);

    let friendly_occupancy = state.bb_mngr.get_all_pieces_bb_off(active_side);
    let opponent_occupancy =
        state.bb_mngr.get_all_pieces_bb_off(opponent_side) & !captured_pawn_bb;

    let king_square = friendly_king.clone().next().unwrap();

    let would_expose_king_to_slider = |capturing_pawn: BitBoard| {
        if capturing_pawn.is_empty() {
            return false;
        }

        let king_slider_rays = get_slider_moves_at_square::<true>(
            king_square,
            friendly_occupancy & !capturing_pawn,
            opponent_occupancy,
        );

        (king_slider_rays & opponent_sliders).is_not_empty()
    };

    !would_expose_king_to_slider(left_capturing_pawn)
        && !would_expose_king_to_slider(right_capturing_pawn)
}
fn single_push(
    moves: &mut Vec<Moove>,
    active_color: Side,
    occupancy_bb: BitBoard,
    checkmask_bb: BitBoard,
    straight_pin_mask: BitBoard,
    pawn_bb: BitBoard,
    rank_offset: i8,
) {
    let mut push_pawn_bb = match active_color {
        Side::White => (pawn_bb & !straight_pin_mask) << 8,
        Side::Black => (pawn_bb & !straight_pin_mask) >> 8,
    };
    // cant go there if something is there or if the checkmask forbids it
    push_pawn_bb &= !occupancy_bb & checkmask_bb;
    pawn_bb_to_moves_no_promotion(moves, push_pawn_bb &!PROMOTION_RANKS_BB, 0, rank_offset);
    pawn_bb_to_moves_promotion(moves, push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);

    let mut push_pawn_bb = match active_color {
        Side::White => (pawn_bb & straight_pin_mask) << 8,
        Side::Black => (pawn_bb & straight_pin_mask) >> 8,
    };
    // cant go there if something is there or if the checkmask forbids it
    push_pawn_bb &= !occupancy_bb & checkmask_bb & straight_pin_mask;
    pawn_bb_to_moves_no_promotion(moves, push_pawn_bb &!PROMOTION_RANKS_BB, 0, rank_offset);
    pawn_bb_to_moves_promotion(moves, push_pawn_bb & PROMOTION_RANKS_BB, 0, rank_offset);
}

fn double_push(
    moves: &mut Vec<Moove>,
    active_color: Side,
    occupancy_bb: BitBoard,
    checkmask_bb: BitBoard,
    straight_pin_mask: BitBoard,
    pawn_bb: BitBoard,
    rank_offset: i8,
) {
    let double_push_bb = match active_color {
        Side::White => {
            (((pawn_bb & WHITE_PAWN_START_RANK_BB & !straight_pin_mask) << 8) & !occupancy_bb) << 8 & !occupancy_bb
        }
        Side::Black => {
            (((pawn_bb & BLACK_PAWN_START_RANK_BB & !straight_pin_mask) >> 8) & !occupancy_bb) >> 8 & !occupancy_bb
        }
    };
    pawn_bb_to_moves_no_promotion(moves, double_push_bb & checkmask_bb, 0, 2 * rank_offset);

    let double_push_bb = match active_color {
        Side::White => {
            (((pawn_bb & WHITE_PAWN_START_RANK_BB & straight_pin_mask) << 8) & !occupancy_bb) << 8 & !occupancy_bb
        }
        Side::Black => {
            (((pawn_bb & BLACK_PAWN_START_RANK_BB & straight_pin_mask) >> 8) & !occupancy_bb) >> 8 & !occupancy_bb
        }
    };
    pawn_bb_to_moves_no_promotion(moves, double_push_bb & checkmask_bb & straight_pin_mask, 0, 2 * rank_offset);
}

fn one_dir_capture(
    moves: &mut Vec<Moove>,
    enemy_pieces_bb: BitBoard,
    mut pawn_bb: BitBoard,
    checkmask: BitBoard,
    diag_pin_mask: BitBoard,
    rank_offset: i8,
    shift: i32,
    file_offset: i8,
) {
    let free_pawns: BitBoard = match shift.is_negative() {
        true => {
            (pawn_bb & !diag_pin_mask) >> shift.unsigned_abs() as i32
        },
        false => {
            (pawn_bb & !diag_pin_mask) << shift
        },
    };
    let capture_bb = free_pawns & enemy_pieces_bb & checkmask;

    pawn_bb_to_moves_no_promotion(moves, capture_bb & !PROMOTION_RANKS_BB, file_offset, rank_offset);
    pawn_bb_to_moves_promotion(moves, capture_bb & PROMOTION_RANKS_BB, file_offset, rank_offset);

    let free_pawns: BitBoard = match shift.is_negative() {
        true => {
            (pawn_bb & diag_pin_mask) >> shift.unsigned_abs() as i32
        },
        false => {
            (pawn_bb & diag_pin_mask) << shift
        },
    };
    let capture_bb = free_pawns & enemy_pieces_bb & checkmask & diag_pin_mask;
    pawn_bb_to_moves_no_promotion(moves, capture_bb & !PROMOTION_RANKS_BB, file_offset, rank_offset);
    pawn_bb_to_moves_promotion(moves, capture_bb & PROMOTION_RANKS_BB, file_offset, rank_offset);
}

fn pawn_bb_to_moves_no_promotion(
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
