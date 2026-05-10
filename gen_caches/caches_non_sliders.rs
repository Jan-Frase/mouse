use crate::build_util::{
    is_square_at_bottom_edge, is_square_at_left_edge, is_square_at_right_edge,
    is_square_at_top_edge, is_square_valid, square_from_rank_and_file, square_to_bb,
    square_to_file, square_to_rank,
};

/// Generates a `BitBoard` with all possible moves for a king piece from a given `Square`.
pub fn gen_king_moves(square: i8) -> u64 {
    let mut bitboard = 0;

    let rank = square_to_rank(square);
    let file = square_to_file(square);

    for file_offset in -1..=1 {
        for rank_offset in -1..=1 {
            // skip the current square
            if file_offset == 0 && rank_offset == 0 {
                continue;
            }
            // create the relevant square
            let current_rank = rank + rank_offset;
            let current_file = file + file_offset;

            // and add it if it's valid
            if is_square_valid(current_rank, current_file) {
                let current_square = current_rank * 8 + current_file;
                let bb = square_to_bb(current_square);
                bitboard |= bb;
            }
        }
    }
    bitboard
}

/// Same as above but for the knight.
pub fn gen_knight_moves(square: i8) -> u64 {
    let mut bitboard = 0;

    let rank = square_to_rank(square);
    let file = square_to_file(square);

    // The offsets for the knight moves. Starting in the top left corner.
    // `_ _ _ _ _ _ _ _`
    // `_ _ 2 _ 3 _ _ _`
    // `_ 1 _ _ _ 4 _ _`
    // `_ _ _ K _ _ _ _`
    // `_ 8 _ _ _ 5 _ _`
    // `_ _ 7 _ 6 _ _ _`
    // `_ _ _ _ _ _ _ _`
    // `_ _ _ _ _ _ _ _`
    let file_offset = [-2, -1, 1, 2, 2, 1, -1, -2];
    let rank_offset = [1, 2, 2, 1, -1, -2, -2, -1];

    for index in 0..8 {
        // Calculate the current square.
        let current_file = file + file_offset[index];
        let current_rank = rank + rank_offset[index];

        // If it is valid, add it to the bitboard.
        if is_square_valid(current_rank, current_file) {
            let current_square = square_from_rank_and_file(current_rank, current_file);
            let bb = square_to_bb(current_square);
            bitboard |= bb;
        }
    }

    bitboard
}

pub fn gen_pawn_captures() -> [[u64; 64]; 2] {
    let mut quiet_moves = [[0; 64]; 2];

    for (side, quiet_moves_per_side) in quiet_moves.iter_mut().enumerate() {
        // iterate over all squares
        for square in 0..64 {
            // and generate the moves for that square
            quiet_moves_per_side[square as usize] = gen_pawn_captures_at_square(square, side == 0);
        }
    }

    quiet_moves
}

fn gen_pawn_captures_at_square(square: i8, is_white: bool) -> u64 {
    let mut bitboard: u64 = 0;
    let mut current_square = square;

    match is_white {
        true => {
            if is_square_at_top_edge(current_square) {
                return bitboard;
            }
            current_square += 8;
        }
        false => {
            if is_square_at_bottom_edge(current_square) {
                return bitboard;
            }
            current_square -= 8;
        }
    }

    if !is_square_at_right_edge(current_square) {
        bitboard |= square_to_bb(current_square + 1);
    }
    if !is_square_at_left_edge(current_square) {
        bitboard |= square_to_bb(current_square - 1);
    }

    bitboard
}
