// Some functions here are somewhat duplicates of what already exists in the core code.
// However, this section cannot access those.
// This thus seemed like a sensible compromise.

pub fn is_square_at_right_edge(square: i8) -> bool {
    square % 8 == 7
}

pub fn is_square_at_left_edge(square: i8) -> bool {
    square % 8 == 0
}

pub fn is_square_at_top_edge(square: i8) -> bool {
    square / 8 == 7
}

pub fn is_square_at_bottom_edge(square: i8) -> bool {
    square / 8 == 0
}

pub fn is_square_valid(rank: i8, file: i8) -> bool {
    (0..8).contains(&rank) && (0..8).contains(&file)
}

pub fn square_to_bb(square: i8) -> u64 {
    1 << square
}

pub fn square_to_rank(square: i8) -> i8 {
    square / 8
}

pub fn square_to_file(square: i8) -> i8 {
    square % 8
}

pub fn square_from_rank_and_file(rank: i8, file: i8) -> i8 {
    rank * 8 + file
}
