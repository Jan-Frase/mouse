/// Represents the different pieces.
///
/// Dear god, this is a bit hacky.
/// Do not change the order of this to differ from the PROMOTABLE_PIECES list.
#[derive(Copy, Clone, Debug, Ord, Eq, PartialEq, PartialOrd)]
pub enum Piece {
    Rook,
    Knight,
    Bishop,
    Queen,
    King,
    Pawn,
}

/// The order of this has to match with the Piece enum.
pub const ALL_PIECES: [Piece; 6] = [
    Piece::Rook,
    Piece::Knight,
    Piece::Bishop,
    Piece::Queen,
    Piece::King,
    Piece::Pawn,
];

pub const PROMOTABLE_PIECES: [Piece; 4] = [Piece::Rook, Piece::Knight, Piece::Bishop, Piece::Queen];

/// Represents the color of a piece.
#[derive(Copy, Clone, Debug)]
pub enum Side {
    White,
    Black,
}

impl Side {
    /// Returns the opposite color of the current `PieceColor`.
    pub fn oppo(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }

    pub fn get_all_colors() -> [Side; 2] {
        [Side::White, Side::Black]
    }
}
