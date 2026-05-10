use crate::backend::caches::{
    BISHOP_PEXT_INDEX, BISHOP_PEXT_MASK, BISHOP_XRAY_PEXT_INDEX, BISHOP_XRAY_PEXT_MASK,
    PEXT_TABLE, PEXT_XRAY_TABLE, ROOK_PEXT_INDEX, ROOK_PEXT_MASK, ROOK_XRAY_PEXT_INDEX,
    ROOK_XRAY_PEXT_MASK,
};
use crate::backend::movegen::move_gen::convert_bitboard_to_moves;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::Moove;
use crate::backend::types::piece::Piece;
use crate::backend::types::square::Square;
use std::arch::x86_64::_pext_u64;

pub fn get_slider_moves(
    moves: &mut Vec<Moove>,
    piece_type: Piece,
    piece_bb: BitBoard,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    checkmask: BitBoard,
    pin_mask: BitBoard,
) {
    let unpinned_pieces = piece_bb & !pin_mask;
    let pinned_pieces = piece_bb & pin_mask;

    append_slider_moves_for_squares(
        moves,
        piece_type,
        unpinned_pieces,
        friendly_pieces_bb,
        enemy_pieces_bb,
        checkmask,
    );

    append_slider_moves_for_squares(
        moves,
        piece_type,
        pinned_pieces,
        friendly_pieces_bb,
        enemy_pieces_bb,
        checkmask & pin_mask,
    );
}

fn append_slider_moves_for_squares(
    moves: &mut Vec<Moove>,
    piece_type: Piece,
    piece_squares: BitBoard,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    legal_move_mask: BitBoard,
) {
    for square in piece_squares {
        let moves_for_piece_bb =
            slider_attacks_for_piece(piece_type, friendly_pieces_bb, enemy_pieces_bb, square)
                & legal_move_mask;

        convert_bitboard_to_moves(moves, square, moves_for_piece_bb);
    }
}

fn slider_attacks_for_piece(
    piece_type: Piece,
    friendly_pieces_bb: BitBoard,
    enemy_pieces_bb: BitBoard,
    square: Square,
) -> BitBoard {
    match piece_type {
        Piece::Rook => get_slider_moves_at_square::<true>(square, friendly_pieces_bb, enemy_pieces_bb),
        Piece::Bishop => {
            get_slider_moves_at_square::<false>(square, friendly_pieces_bb, enemy_pieces_bb)
        }
        Piece::Queen => {
            get_slider_moves_at_square::<true>(square, friendly_pieces_bb, enemy_pieces_bb)
                | get_slider_moves_at_square::<false>(square, friendly_pieces_bb, enemy_pieces_bb)
        }
        _ => unreachable!("slider move generation was called for a non-slider piece"),
    }
}

pub fn get_slider_xray_moves_at_square<const IS_STRAIGHT: bool>(
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

    pext_table_lookup(&PEXT_XRAY_TABLE, pext_index, pext_mask, occ_bb)
}

/// Computes the sliding piece moves, either rook-like or bishop-like, for a
/// given square based on occupancy bitboards.
///
/// # Type Parameters
/// - `IS_STRAIGHT`:
///   - `true` for rook-like horizontal and vertical moves.
///   - `false` for bishop-like diagonal moves.
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

    pext_table_lookup(&PEXT_TABLE, pext_index, pext_mask, occupied_bb) & !friendly_bb
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
