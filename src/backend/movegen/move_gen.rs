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
pub fn moves(state: &State, captures_only: bool) -> Vec<Moove> {
    let active_side = state.active_side;
    let friendly_pieces_bb = state.bb_mngr.get_side_bb(active_side);
    let enemy_pieces_bb = state.bb_mngr.get_side_bb(active_side.oppo());

    let captures_only_mask = if captures_only {enemy_pieces_bb} else {BitBoard { value: u64::MAX }};

    let checking_squares = checkers(state);
    let checking_piece_count = checking_squares.value.count_ones();
    let is_double_check = checking_piece_count == DOUBLE_CHECK_ATTACKERS;
    let is_not_in_check = checking_piece_count == NO_CHECK_ATTACKERS;

    let mut moves = Vec::with_capacity(INITIAL_MOVE_CAPACITY);

    gen_king_moves(&mut moves, state, friendly_pieces_bb, captures_only_mask);

    if is_double_check {
        return moves;
    }

    let king_square = state
        .bb_mngr
        .get_colored_piece_bb(King, active_side)
        .next()
        .unwrap();

    let check_mask = build_check_mask(checking_squares, king_square, is_not_in_check);

    gen_knight_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        check_mask,
        captures_only_mask
    );

    if !captures_only & is_not_in_check {
        gen_castles(&mut moves, state, state.bb_mngr.get_occupied_bb());
    }

    gen_pawn_moves(
        &mut moves,
        state,
        check_mask,
        active_side,
        captures_only
    );

    gen_bishop_and_queen_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        captures_only_mask
    );

    gen_rook_and_queen_moves(
        &mut moves,
        state,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        captures_only_mask
    );

    moves
}

fn gen_king_moves(moves: &mut Vec<Moove>, state: &State, friendly_pieces_bb: BitBoard, mask: BitBoard) {
    let king_bb = state.bb_mngr.get_colored_piece_bb(King, state.active_side);
    // this is needed for cases like: k3rR2/8/8/5n2/8/4K3/8/8 w - - 0 1
    // if we attempt t move down, the king blocks its own check otherwise
    let friendly_bb_without_king = friendly_pieces_bb & !king_bb;

    for from_square in king_bb {
        let legal_targets_bb = KING_MOVES[from_square as usize] & !friendly_pieces_bb & mask;

        for to_square in legal_targets_bb {
            if !is_in_check_on_square(state, state.active_side, to_square, friendly_bb_without_king) {
                moves.push(Moove::new(from_square, to_square));
            }
        }
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

fn gen_knight_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    check_mask: BitBoard,
    captures_only_mask: BitBoard
) {
    let knights = state
        .bb_mngr
        .get_colored_piece_bb(Knight, state.active_side)
        & !(state.straight_pin_mask | state.diag_pin_mask);

    for from_square in knights {
        let legal_targets_bb =
            KNIGHT_MOVES[from_square as usize] & check_mask & !friendly_pieces_bb & captures_only_mask;

        convert_bitboard_to_moves(moves, from_square, legal_targets_bb);
    }
}

fn gen_bishop_and_queen_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    check_mask: BitBoard,
    captures_only_mask: BitBoard
) {
    let bishop_like_pieces_bb = state
        .bb_mngr
        .get_colored_piece_bb(Bishop, state.active_side)
        | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

    get_slider_moves(
        moves,
        Bishop,
        bishop_like_pieces_bb & !state.straight_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        state.diag_pin_mask,
        captures_only_mask
    );
}

fn gen_rook_and_queen_moves(
    moves: &mut Vec<Moove>,
    state: &State,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    check_mask: BitBoard,
    captures_only_mask: BitBoard
) {
    let rook_like_pieces_bb = state.bb_mngr.get_colored_piece_bb(Rook, state.active_side)
        | state.bb_mngr.get_colored_piece_bb(Queen, state.active_side);

    get_slider_moves(
        moves,
        Rook,
        rook_like_pieces_bb & !state.diag_pin_mask,
        friendly_pieces_bb,
        enemy_pieces_bb,
        check_mask,
        state.straight_pin_mask,
        captures_only_mask
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::game_state::state::State;

    fn is_capture(state: &State, moove: &Moove) -> bool {
        let enemy_pieces_bb = state.bb_mngr.get_side_bb(state.active_side.oppo());
        let to_square_bb = BitBoard::new_from_square(moove.get_to());
        (enemy_pieces_bb & to_square_bb).is_not_empty()
    }

    #[test]
    fn test_captures_only_generates_all_captures() {
        // Test with starting position
        let state = State::new_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");
        verify_captures_only(&state);

        // Test with position 2
        let state = State::new_from_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
        verify_captures_only(&state);

        // Test with position 3
        let state = State::new_from_fen("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1");
        verify_captures_only(&state);

        // Test with position 4
        let state = State::new_from_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
        verify_captures_only(&state);
    }

    fn verify_captures_only(state: &State) {
        let all_moves = moves(state, false);
        let captures_from_all: Vec<Moove> = all_moves
            .iter()
            .filter(|m| is_capture(state, m))
            .copied()
            .collect();

        let mut captures_only = moves(state, true);

        // Sort both lists for comparison
        let mut expected = captures_from_all.clone();
        expected.sort();
        captures_only.sort();

        assert_eq!(
            captures_only.len(),
            expected.len(),
            "Number of captures should match. captures_only={}, expected={}",
            captures_only.len(),
            expected.len()
        );

        for (i, (actual, expected)) in captures_only.iter().zip(expected.iter()).enumerate() {
            assert_eq!(
                actual, expected,
                "Capture move at index {} differs: actual={}, expected={}",
                i, actual, expected
            );
        }
    }
}
