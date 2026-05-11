use crate::backend::caches::{BETWEEN_TABLE, KING_MOVES, KNIGHT_MOVES};
use crate::backend::game_state::state::State;
use crate::backend::movegen::check_decider::{checkers, is_in_check_on_square};
use crate::backend::movegen::move_gen_king::gen_castles;
use crate::backend::movegen::move_gen_pawn::gen_pawn_moves;
use crate::backend::movegen::move_gen_sliders::{
    get_slider_moves, get_slider_xray_moves_at_square,
};
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::Moove;
use crate::backend::types::piece::Piece::*;
use crate::backend::types::square::{Square, back_by_one};

const INITIAL_MOVE_CAPACITY: usize = 50;
const DOUBLE_CHECK_ATTACKERS: u32 = 2;
const NO_CHECK_ATTACKERS: u32 = 0;

/// Generates and returns all legal moves for the current player's pieces
/// based on the provided game state.
pub fn moves(state: &mut State) -> Vec<Moove> {
    let active_side = state.active_side;
    let friendly_pieces_bb = state.bb_mngr.get_side_bb(active_side);
    let enemy_pieces_bb = state.bb_mngr.get_side_bb(active_side.oppo());

    let checking_squares = checkers(state);
    let checking_piece_count = checking_squares.value.count_ones();
    let is_double_check = checking_piece_count == DOUBLE_CHECK_ATTACKERS;
    let is_not_in_check = checking_piece_count == NO_CHECK_ATTACKERS;

    let mut moves = Vec::with_capacity(INITIAL_MOVE_CAPACITY);

    gen_king_moves(&mut moves, state, friendly_pieces_bb);

    if is_double_check {
        return moves;
    }

    let king_square = state
        .bb_mngr
        .get_colored_piece_bb(King, active_side)
        .next()
        .unwrap();

    let check_mask = build_check_mask(checking_squares, king_square, is_not_in_check);

    let occupied_bb = friendly_pieces_bb | enemy_pieces_bb;
    let (straight_pin_mask, diag_pin_mask) =
        build_pin_masks(state, king_square, occupied_bb, enemy_pieces_bb);

    remove_illegal_en_passant_if_pinned(state, diag_pin_mask);

    gen_knight_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        check_mask,
        straight_pin_mask,
        diag_pin_mask,
    );

    if is_not_in_check {
        gen_castles(&mut moves, state, state.bb_mngr.get_occupied_bb());
    }

    gen_pawn_moves(
        &mut moves,
        state,
        check_mask,
        straight_pin_mask,
        diag_pin_mask,
        active_side,
    );

    gen_bishop_and_queen_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        straight_pin_mask,
        diag_pin_mask,
    );

    gen_rook_and_queen_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        straight_pin_mask,
        diag_pin_mask,
    );

    moves
}

fn gen_king_moves(moves: &mut Vec<Moove>, state: &mut State, friendly_pieces_bb: BitBoard) {
    let king_bb = state.bb_mngr.get_colored_piece_bb(King, state.active_side);

    for from_square in king_bb {
        let legal_targets_bb = KING_MOVES[from_square as usize] & !friendly_pieces_bb;

        state
            .bb_mngr
            .get_piece_bb_mut(King)
            .clear_square(from_square);
        state
            .bb_mngr
            .get_side_bb_mut(state.active_side)
            .clear_square(from_square);

        for to_square in legal_targets_bb {
            if !is_in_check_on_square(state, state.active_side, to_square) {
                moves.push(Moove::new(from_square, to_square));
            }
        }

        state
            .bb_mngr
            .get_piece_bb_mut(King)
            .fill_square(from_square);
        state
            .bb_mngr
            .get_side_bb_mut(state.active_side)
            .fill_square(from_square);
    }
}

// Checking squares are all squares between the king and the attacker, including the attacker.
// They are used as a mask for legal move-gen.
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
    let straight_xray_bb = get_slider_xray_moves_at_square::<true>(king_square, occupied_bb);
    let diag_xray_bb = get_slider_xray_moves_at_square::<false>(king_square, occupied_bb);

    let straight_xray_attackers_bb = straight_xray_bb
        & (state.bb_mngr.get_piece_bb(Rook) | state.bb_mngr.get_piece_bb(Queen))
        & enemy_pieces_bb;

    let diag_xray_attackers_bb = diag_xray_bb
        & (state.bb_mngr.get_piece_bb(Bishop) | state.bb_mngr.get_piece_bb(Queen))
        & enemy_pieces_bb;

    (
        build_pin_mask(straight_xray_attackers_bb, king_square),
        build_pin_mask(diag_xray_attackers_bb, king_square),
    )
}

fn build_pin_mask(xray_attackers_bb: BitBoard, king_square: Square) -> BitBoard {
    let mut pin_mask = BitBoard { value: 0 };

    for attacker_square in xray_attackers_bb {
        // The order of indices is important:
        // the attacker square is included, while the king square is not.
        // This keeps capturing the pinning piece legal.
        pin_mask |= BETWEEN_TABLE[attacker_square as usize][king_square as usize];
    }

    pin_mask
}

fn remove_illegal_en_passant_if_pinned(state: &mut State, diag_pin_mask: BitBoard) {
    let Some(en_passant_square) = state.irreversible_data.en_passant_square else {
        return;
    };

    let captured_pawn_square = back_by_one(en_passant_square, state.active_side);
    let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

    if (captured_pawn_bb & diag_pin_mask).is_not_empty() {
        state.irreversible_data.en_passant_square = None;
    }
}

fn gen_knight_moves(
    moves: &mut Vec<Moove>,
    state: &mut State,
    friendly_pieces_bb: BitBoard,
    check_mask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
) {
    let knights = state
        .bb_mngr
        .get_colored_piece_bb(Knight, state.active_side)
        & !(straight_pin_mask | diag_pin_mask);

    for from_square in knights {
        let legal_targets_bb =
            KNIGHT_MOVES[from_square as usize] & check_mask & !friendly_pieces_bb;

        convert_bitboard_to_moves(moves, from_square, legal_targets_bb);
    }
}

fn gen_bishop_and_queen_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    check_mask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
) {
    let bishop_like_pieces_bb = state
        .bb_mngr
        .get_colored_piece_bb(Bishop, state.active_side)
        | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

    get_slider_moves(
        moves,
        Bishop,
        bishop_like_pieces_bb & !straight_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        diag_pin_mask,
    );
}

fn gen_rook_and_queen_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    check_mask: BitBoard,
    straight_pin_mask: BitBoard,
    diag_pin_mask: BitBoard,
) {
    let rook_like_pieces_bb = state.bb_mngr.get_colored_piece_bb(Rook, state.active_side)
        | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

    get_slider_moves(
        moves,
        Rook,
        rook_like_pieces_bb & !diag_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        straight_pin_mask,
    );
}

pub fn convert_bitboard_to_moves(
    moves: &mut Vec<Moove>,
    from_square: Square,
    moves_bitboard: BitBoard,
) {
    for to_square in moves_bitboard {
        moves.push(Moove::new(from_square, to_square));
    }
}
