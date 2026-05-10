use crate::backend::types::piece::Piece::*;
use crate::backend::caches::{KING_MOVES, KNIGHT_MOVES, PAWN_CAPTURE_MOVES};
use crate::backend::movegen::move_gen_sliders::{get_slider_moves_at_square, get_slider_xray_moves_at_square};
use crate::backend::game_state::state::State;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::piece::{ALL_PIECES, Piece, Side};
use crate::backend::types::square::Square;

const CHECKING_PIECES_WITHOUT_KING: [Piece; 5] = [Rook, Knight, Bishop, Queen, Pawn];

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
pub fn is_in_check_on_square(state: &State, side: Side, king_square: Square) -> bool {
    let friendly_bb = state.bb_mngr.get_all_pieces_bb_off(side);
    let enemy_bb = state.bb_mngr.get_all_pieces_bb_off(side.oppo());

    for piece_type in ALL_PIECES {
        let attackers_bb = get_attackers_of_piece_type_on_square(
            state,
            side,
            king_square,
            friendly_bb,
            enemy_bb,
            piece_type,
        );

        if attackers_bb.is_not_empty() {
            return true;
        }
    }

    false
}

// Checking squares are all squares between the king and the attacker, including the attacker.
// They are used as a mask for legal move-gen.
pub fn get_checking_squares(state: &State) -> BitBoard {
    let side = state.active_side;
    let mut king_bb = state.bb_mngr.get_piece_bb(King) & state.bb_mngr.get_all_pieces_bb_off(side);
    let king_square = king_bb.next().unwrap();

    let friendly_bb = state.bb_mngr.get_all_pieces_bb_off(side);
    let enemy_bb = state.bb_mngr.get_all_pieces_bb_off(side.oppo());
    let mut checking_squares_bb = BitBoard::new();

    for piece_type in CHECKING_PIECES_WITHOUT_KING {
        checking_squares_bb |= get_attackers_of_piece_type_on_square(
            state,
            side,
            king_square,
            friendly_bb,
            enemy_bb,
            piece_type,
        );
    }

    checking_squares_bb
}

fn get_attackers_of_piece_type_on_square(
    state: &State,
    side: Side,
    target_square: Square,
    friendly_bb: BitBoard,
    enemy_bb: BitBoard,
    piece_type: Piece,
) -> BitBoard {
    let attacked_squares = match piece_type {
        King => KING_MOVES[target_square as usize],
        Knight => KNIGHT_MOVES[target_square as usize],
        Pawn => PAWN_CAPTURE_MOVES[side as usize][target_square as usize],
        Rook => get_slider_moves_at_square::<true>(target_square, friendly_bb, enemy_bb),
        Bishop => get_slider_moves_at_square::<false>(target_square, friendly_bb, enemy_bb),
        Queen => {
            get_slider_moves_at_square::<true>(target_square, friendly_bb, enemy_bb)
                | get_slider_moves_at_square::<false>(target_square, friendly_bb, enemy_bb)
        }
    };

    let enemy_pieces_of_type = state.bb_mngr.get_piece_bb(piece_type) & enemy_bb;

    attacked_squares & enemy_pieces_of_type
}
