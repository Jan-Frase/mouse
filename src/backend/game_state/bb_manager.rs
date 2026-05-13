use crate::backend::constants::PIECE_TYPE_COUNT;
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::piece::ALL_PIECES;
use crate::backend::types::piece::{Piece, Side};
use crate::backend::types::square::Square;

/// Manages bitboards used to represent chess pieces and their positions.
///
/// `BBManager` stores separate occupancy bitboards for each side, plus one
/// bitboard per piece type.
#[derive(Debug, Clone)]
pub struct BBManager {
    white_bb: BitBoard,
    black_bb: BitBoard,
    piece_bbs: [BitBoard; PIECE_TYPE_COUNT],
}

impl BBManager {
    /// Generates a new `BBManager` with all bitboards set to empty.
    pub fn new() -> Self {
        Self {
            // TODO: turn into another array
            white_bb: BitBoard::new(),
            black_bb: BitBoard::new(),
            piece_bbs: [BitBoard::new(); PIECE_TYPE_COUNT],
        }
    }

    fn piece_index(piece: Piece) -> usize {
        piece as usize
    }

    /// Retrieves a mutable reference to the bitboard associated with the given piece.
    pub fn get_piece_bb_mut(&mut self, piece: Piece) -> &mut BitBoard {
        &mut self.piece_bbs[Self::piece_index(piece)]
    }

    /// Retrieves a copy of the `BitBoard` associated with the specified piece.
    pub fn get_piece_bb(&self, piece: Piece) -> BitBoard {
        self.piece_bbs[Self::piece_index(piece)]
    }

    /// Returns a `BitBoard` containing all positions occupied by pieces of the specified side.
    pub fn get_side_bb(&self, side: Side) -> BitBoard {
        match side {
            Side::White => self.white_bb,
            Side::Black => self.black_bb,
        }
    }

    /// Returns a mutable bitboard containing all positions occupied by pieces of the specified side.
    pub fn get_side_bb_mut(&mut self, side: Side) -> &mut BitBoard {
        match side {
            Side::White => &mut self.white_bb,
            Side::Black => &mut self.black_bb,
        }
    }

    /// Returns a `BitBoard` containing all occupied positions.
    pub fn get_occupied_bb(&self) -> BitBoard {
        self.white_bb | self.black_bb
    }

    /// Returns a `BitBoard` containing all positions occupied by the specified piece and side.
    pub fn get_colored_piece_bb(&self, piece: Piece, side: Side) -> BitBoard {
        let piece_bb = self.get_piece_bb(piece);
        let side_bb = self.get_side_bb(side);

        piece_bb & side_bb
    }

    /// Retrieves the piece located at a specific square on the chessboard.
    pub fn get_piece_at_square(&self, square: Square) -> Option<Piece> {
        ALL_PIECES
            .iter()
            .copied()
            .find(|piece| self.get_piece_bb(*piece).get_square(square))
    }

    pub fn clear_square(&mut self, square: Square, piece: Piece, side: Side) {
        self.piece_bbs[piece as usize].clear_square(square);
        match side {
            Side::White => self.white_bb.clear_square(square),
            Side::Black => self.black_bb.clear_square(square),
        }
    }

    pub fn fill_square(&mut self, square: Square, piece: Piece, side: Side) {
        self.piece_bbs[piece as usize].fill_square(square);
        match side {
            Side::White => self.white_bb.fill_square(square),
            Side::Black => self.black_bb.fill_square(square),
        }
    }
}

impl Default for BBManager {
    fn default() -> Self {
        Self::new()
    }
}
