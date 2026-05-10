use crate::build_util::{is_square_valid, square_to_file, square_to_rank};

pub const PEXT_TABLE_SIZE: usize = 107_648;

// Made with: https://tearth.dev/bitboard-viewer
const EDGE_OF_BOARD_MASK: u64 =
    0b11111111_10000001_10000001_10000001_10000001_10000001_10000001_11111111;

const LEFT_SIDE_MASK: u64 = 282578800148736;

const RIGHT_SIDE_MASK: u64 = 36170086419038208;

const TOP_SIDE_MASK: u64 = 9079256848778919936;

const BOTTOM_SIDE_MASK: u64 = 126;

// ----------------------------------------
// PEXT GEN LOGIC
// ----------------------------------------
// https://lorentzvedeler.com/2025/03/03/Pext-Tables/

enum Piece {
    Rook,
    Bishop,
}

pub struct PextData {
    pub rook_pext_mask: [u64; 64],
    pub rook_pext_index: [usize; 64],
    pub bishop_pext_mask: [u64; 64],
    pub bishop_pext_index: [usize; 64],
    pub pext_table: [u64; PEXT_TABLE_SIZE],
}

pub fn gen_cache_sliders(xray: bool) -> PextData {
    let mut rook_pext_mask = [0; 64];
    let mut rook_pext_index = [0usize; 64];

    let mut bishop_pext_mask = [0; 64];
    let mut bishop_pext_index = [0usize; 64];

    let mut pext_table = [0; 107_648];
    let mut current_pext_table_index = 0;

    gen_for_piece(
        xray,
        &Piece::Rook,
        &mut rook_pext_mask,
        &mut rook_pext_index,
        &mut pext_table,
        &mut current_pext_table_index,
    );

    gen_for_piece(
        xray,
        &Piece::Bishop,
        &mut bishop_pext_mask,
        &mut bishop_pext_index,
        &mut pext_table,
        &mut current_pext_table_index,
    );

    PextData {
        rook_pext_mask,
        rook_pext_index,
        bishop_pext_mask,
        bishop_pext_index,
        pext_table,
    }
}

fn gen_for_piece(
    xray: bool,
    piece: &Piece,
    piece_pext_mask: &mut [u64; 64],
    piece_pext_index: &mut [usize; 64],
    pext_table: &mut [u64; PEXT_TABLE_SIZE],
    current_pext_table_index: &mut usize,
) {
    for square in 0..64 {
        piece_pext_index[square as usize] = *current_pext_table_index;

        let mut relevant_squares = calculate_slider_move_bitboard(xray, piece, square, 0);
        relevant_squares &= !adjust_edge_of_board(square);
        piece_pext_mask[square as usize] = relevant_squares;

        let amount_of_blocker_squares = relevant_squares.count_ones();
        let amount_of_possible_blocker_configurations = 2u64.pow(amount_of_blocker_squares);
        // Iterate over all possible permutations of blocker configurations
        for blocker_config_index in 0..amount_of_possible_blocker_configurations {
            let blockers: u64 = pdep64(blocker_config_index, relevant_squares);

            let moves_bb = calculate_slider_move_bitboard(xray, piece, square, blockers);
            pext_table[*current_pext_table_index] = moves_bb;
            *current_pext_table_index += 1;
        }
    }
}

fn adjust_edge_of_board(square: i8) -> u64 {
    let rank = square_to_rank(square);
    let file = square_to_file(square);

    let mut adjustment_mask = 0u64;
    if file == 0 {
        adjustment_mask |= LEFT_SIDE_MASK;
    }
    if file == 7 {
        adjustment_mask |= RIGHT_SIDE_MASK;
    }
    if rank == 0 {
        adjustment_mask |= BOTTOM_SIDE_MASK;
    }
    if rank == 7 {
        adjustment_mask |= TOP_SIDE_MASK;
    }

    adjustment_mask = EDGE_OF_BOARD_MASK & !adjustment_mask;
    adjustment_mask
}

/// this exists to provide a const alternative to the pdep intrinsic.
/// its implementations is based on:
/// https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html#text=_pdep_u64&ig_expand=490
fn pdep64(word: u64, mask: u64) -> u64 {
    let mut out = 0;
    let mut input_idx = 0;

    let mut i = 0;
    while i < 64 {
        let ith_mask_bit = (mask >> i) & 1;
        if ith_mask_bit == 1 {
            let next_word_bit = (word >> input_idx) & 1;
            out |= next_word_bit << i;
            input_idx += 1;
        }

        i += 1;
    }

    out
}

// ----------------------------------------
// NORMAL SLIDER MOVE GEN
// ----------------------------------------

enum SlideDirection {
    Up,
    UpRight,
    Right,
    DownRight,
    Down,
    DownLeft,
    Left,
    UpLeft,
}

impl SlideDirection {
    fn next(&self, file: i8, rank: i8) -> (i8, i8) {
        match self {
            SlideDirection::Up => (file, rank + 1),
            SlideDirection::UpRight => (file + 1, rank + 1),
            SlideDirection::Right => (file + 1, rank),
            SlideDirection::DownRight => (file + 1, rank - 1),
            SlideDirection::Down => (file, rank - 1),
            SlideDirection::DownLeft => (file - 1, rank - 1),
            SlideDirection::Left => (file - 1, rank),
            SlideDirection::UpLeft => (file - 1, rank + 1),
        }
    }
}

const ROOK_DIR: [SlideDirection; 4] = [
    SlideDirection::Up,
    SlideDirection::Down,
    SlideDirection::Left,
    SlideDirection::Right,
];
const BISHOP_DIR: [SlideDirection; 4] = [
    SlideDirection::UpRight,
    SlideDirection::DownRight,
    SlideDirection::DownLeft,
    SlideDirection::UpLeft,
];

fn calculate_slider_move_bitboard(
    xray: bool,
    piece_type: &Piece,
    square: i8,
    blocker_bb: u64,
) -> u64 {
    let mut move_bb = 0u64;

    match piece_type {
        Piece::Rook => {
            for dir in ROOK_DIR {
                move_bb |= calculate_max_slide_range(xray, square, &dir, blocker_bb);
            }
        }
        Piece::Bishop => {
            for dir in BISHOP_DIR {
                move_bb |= calculate_max_slide_range(xray, square, &dir, blocker_bb);
            }
        }
    }

    move_bb
}

/// Computes a bitboard containing all squares
/// that the piece on the given square can slide to in the given direction
fn calculate_max_slide_range(
    xray: bool,
    square: i8,
    direction: &SlideDirection,
    blocker_bb: u64,
) -> u64 {
    let mut result = 0u64;
    let rank = square_to_rank(square);
    let file = square_to_file(square);
    let mut next = direction.next(file, rank);

    let mut blockers_hit = 0;

    while is_square_valid(next.1, next.0) {
        let bb = 1 << (next.1 * 8 + next.0);
        result |= bb;

        let blocker_hit = blocker_bb & bb != 0;

        // if we hit a blocker, return!
        if !xray && blocker_hit {
            return result;
        }

        if xray && blocker_hit {
            blockers_hit += 1;

            if blockers_hit == 2 {
                return result;
            }
        }

        next = direction.next(next.0, next.1);
    }
    result
}
