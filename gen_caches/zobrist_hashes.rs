use rand::RngExt;

pub struct ZobristRandoms {
    pub pieces: [[[u64; 2]; 6]; 64], // For each side for each piece for each square. This technically wastes 16 slots as pawns cannot access the last and first rank.
    pub en_passant_file: [u64; 8],
    pub white_long_castle_rights: u64,
    pub white_short_castle_rights: u64,
    pub black_long_castle_rights: u64,
    pub black_short_castle_rights: u64,
    pub black_to_move: u64,
}

pub fn zobrist_randoms() -> ZobristRandoms {
    let mut rng = rand::rng();
    ZobristRandoms {
        pieces: std::array::from_fn(|_| rng.random()),
        black_to_move: rng.random(),
        white_long_castle_rights: rng.random(),
        white_short_castle_rights: rng.random(),
        black_long_castle_rights: rng.random(),
        black_short_castle_rights: rng.random(),
        en_passant_file: std::array::from_fn(|_| rng.random()),
    }
}
