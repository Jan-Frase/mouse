use crate::backend::caches::{BETWEEN_TABLE, KING_MOVES, KNIGHT_MOVES};
use crate::backend::constants::SQUARES_AMOUNT;
use crate::backend::types::moove::Moove;
use crate::backend::movegen::move_gen_king::gen_castles;
use crate::backend::movegen::move_gen_pawn::gen_pawn_moves;
use crate::backend::movegen::move_gen_sliders::{get_slider_moves, get_slider_xray_moves_at_square};
use crate::backend::types::bitboard::BitBoard;
use crate::backend::game_state::state::State;
use crate::backend::movegen::check_decider::{get_checking_squares, is_in_check_on_square};
use crate::backend::types::piece::Piece::*;
use crate::backend::types::square::{back_by_one, Square};

/// Generates and returns all the pseudo legal moves for the current player's pieces
/// based on the provided game game_state. This is the entry point for the move generation.
///
/// # Parameters
///
/// * `game_state`: A reference to the current game game_state, which contains
///   information about the board, piece positions, and active color.
///
/// # Returns
///
/// * A `Vec<Moove>` containing all the computed pseudo legal moves for the current player's
///   pieces.
pub fn moves(state: &mut State) -> Vec<Moove> {
    let friendly_pieces_bb = state.bb_mngr.get_all_pieces_bb_off(state.active_side);
    let enemy_pieces_bb = state
        .bb_mngr
        .get_all_pieces_bb_off(state.active_side.oppo());

    // This is a bitboard marking each piece that is currently checking the king (at most 2).
    let checking_squares = get_checking_squares(state);
    let true_bit_amount = checking_squares.value.count_ones();
    let double_check = true_bit_amount == 2;
    let no_check = true_bit_amount == 0;

    let mut moves = Vec::with_capacity(50);

    // Move gen for king (excluding castles) ...
    iterate_over_bitboard_for_non_slider::<true>(
        &mut moves,
        KING_MOVES,
        state
            .bb_mngr
            .get_colored_piece_bb(King, state.active_side),
        !friendly_pieces_bb,
        state,
    );

    // Early return if we are in double check.
    if double_check { return moves; }

    // Completely filled with ones by default.
    // Idea from here: https://web.archive.org/web/20250910134338/https://www.codeproject.com/articles/Worlds-fastest-Bitboard-Chess-Movegenerator
    let mut check_mask = BitBoard{ value: u64::MAX };
    let king_square = state.bb_mngr
        .get_colored_piece_bb(King, state.active_side)
        .next().unwrap();
    if !no_check {
        check_mask.value = 0;
        for square in checking_squares {
            check_mask |= BETWEEN_TABLE[square as usize][king_square as usize];
        }
    }

    let straight_xray_bb = get_slider_xray_moves_at_square::<true>(king_square, friendly_pieces_bb | enemy_pieces_bb);
    let diag_xray_bb = get_slider_xray_moves_at_square::<false>(king_square, friendly_pieces_bb | enemy_pieces_bb);

    let straight_xray_attackers_bb = straight_xray_bb & (state.bb_mngr.get_piece_bb(Rook) | state.bb_mngr.get_piece_bb(Queen)) & enemy_pieces_bb;
    let diag_xray_attackers_bb = diag_xray_bb & (state.bb_mngr.get_piece_bb(Bishop) | state.bb_mngr.get_piece_bb(Queen)) & enemy_pieces_bb;

    let mut straight_pin_mask = BitBoard{ value: 0 };
    for square in straight_xray_attackers_bb {
        // the order of indices is important
        // the square of the first index is included
        // the second is not
        // this way its possible to capture the checker :)
        straight_pin_mask |= BETWEEN_TABLE[square as usize][king_square as usize];
    }

    let mut diag_pin_mask = BitBoard{ value: 0 };
    for square in  diag_xray_attackers_bb {
        // the order of indices is important
        // the square of the first index is included
        // the second is not
        // this way its possible to capture the checker :)
        diag_pin_mask |= BETWEEN_TABLE[square as usize][king_square as usize];
    }

    // This is for a scenario like this: 1k4q1/8/8/3pP3/8/1K6/8/8 w - d6 0 1
    // The ep takable pawn is pinned, thus the ep is not legal and has to be removed
    if state.irreversible_data.en_passant_square.is_some_and(|ep_square| (BitBoard::new_from_square(back_by_one(ep_square, state.active_side)) & diag_pin_mask).is_not_empty()) {
        state.irreversible_data.en_passant_square = None;
    }


    // ... and knights.
    iterate_over_bitboard_for_non_slider::<false>(
        &mut moves,
        KNIGHT_MOVES,
        state
            .bb_mngr
            .get_colored_piece_bb(Knight, state.active_side) & !(straight_pin_mask | diag_pin_mask),
        !friendly_pieces_bb & check_mask,
        state,
    );

    // Can't castle when in check.
    if no_check {
        gen_castles(&mut moves, state, state.bb_mngr.get_all_pieces_bb());
    }

    // Gen pawn moves, quiet, captures, double pushes
    // Pawns are not allowed to switch between pin masks!
    // See here: https://lichess.org/editor/1q4qk/8/8/1p6/2P5/1K6/8/8_w_-_-_0_1?color=white
    gen_pawn_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        straight_pin_mask,
        diag_pin_mask,
        state.active_side,
    );

    // ..bishop..
    get_slider_moves(
        &mut moves,
        Bishop,
        (state
            .bb_mngr
            .get_colored_piece_bb(Bishop, state.active_side) |
        state
            .bb_mngr
            .get_colored_piece_bb(Queen, state.active_side))
            & !straight_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        diag_pin_mask
    );
    //..and rook moves :)
    get_slider_moves(
        &mut moves,
        Rook,
        (state
            .bb_mngr
            .get_colored_piece_bb(Rook, state.active_side) |
            state
                .bb_mngr
                .get_colored_piece_bb(Queen, state.active_side))
            & !diag_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        straight_pin_mask
    );

    moves
}

// ------------------------------------
// Move gen core logic
// ------------------------------------

pub(crate) fn iterate_over_bitboard_for_non_slider<const IS_KING: bool>(
    moves: &mut Vec<Moove>,
    moves_cache: [BitBoard; SQUARES_AMOUNT],
    piece_bb: BitBoard,
    mask_bitboard: BitBoard,
    state: &mut State,
) {
    // Example: We are doing this for all knights.
    // The `moves_cache` array would for each square contain all viable moves for a knight.

    // We iterate over all squares with a knight on it...
    for square in piece_bb {
        // ... get the potential moves for the piece on that square...
        // SLIDER: (This only works this easily for non-sliders)
        let mut potential_moves_bb = moves_cache[square as usize];
        // ... apply the mask ...
        potential_moves_bb &= mask_bitboard;

        // TODO: This can surely be sped up! For example: Loop over enemy pieces and generate their seen squares :)
        // Whenever the king moves, check if the square it moved to can be seen by an enemy.
        if IS_KING {
            state.bb_mngr.get_piece_bb_mut(King).clear_square(square);
            state.bb_mngr.get_all_pieces_bb_off_mut(state.active_side).clear_square(square);

            for to_square in potential_moves_bb {
                let would_put_in_check = is_in_check_on_square(state, state.active_side, to_square);
                if !would_put_in_check {
                    moves.push(Moove::new(square, to_square))
                }
            }

            state.bb_mngr.get_piece_bb_mut(King).fill_square(square);
            state.bb_mngr.get_all_pieces_bb_off_mut(state.active_side).fill_square(square);
        } else {
            //... and convert the resulting bitboard to a list of moves.
            convert_bitboard_to_moves(moves, square, potential_moves_bb);
        }

    }
}

pub fn convert_bitboard_to_moves(moves: &mut Vec<Moove>, square: Square, moves_bitboard: BitBoard) {
    // generate all the moves
    for to_square in moves_bitboard {
        moves.push(Moove::new(square, to_square))
    }
}
