use crate::build_util::{square_from_rank_and_file, square_to_bb, square_to_file, square_to_rank};

pub fn gen_between_cache() -> [[u64; 64]; 64] {
    let mut between_cache = [[0u64; 64]; 64];

    for from_square in 0..64 {
        let from_rank = square_to_rank(from_square);
        let from_file = square_to_file(from_square);

        for to_square in 0..64 {
            let to_rank = square_to_rank(to_square);
            let to_file = square_to_file(to_square);

            // always fill the from_square
            between_cache[from_square as usize][to_square as usize] |= square_to_bb(from_square);

            // if they are on the same horizontal line
            if from_rank == to_rank {
                let dir = (to_file - from_file).signum();

                let mut current_file = from_file;
                while current_file != to_file {
                    let current_square = square_from_rank_and_file(from_rank, current_file);
                    let current_square_as_bb = square_to_bb(current_square);
                    between_cache[from_square as usize][to_square as usize] |= current_square_as_bb;

                    current_file += dir;
                }
            }

            // if they are on the same vertical line
            if from_file == to_file {
                let dir = (to_rank - from_rank).signum();

                let mut current_rank = from_rank;
                while current_rank != to_rank {
                    let current_square = square_from_rank_and_file(current_rank, from_file);
                    let current_square_as_bb = square_to_bb(current_square);
                    between_cache[from_square as usize][to_square as usize] |= current_square_as_bb;

                    current_rank += dir;
                }
            }

            // if they are on an diagonal
            let rank_diff = (from_rank - to_rank).abs();
            let file_diff = (from_file - to_file).abs();
            if rank_diff == file_diff {
                let rank_dir = (to_rank - from_rank).signum();
                let file_dir = (to_file - from_file).signum();

                let mut current_square = from_square;
                while current_square != to_square {
                    let current_square_as_bb = square_to_bb(current_square);
                    between_cache[from_square as usize][to_square as usize] |= current_square_as_bb;

                    current_square += file_dir + 8 * rank_dir;
                }
            }
        }
    }

    between_cache
}
