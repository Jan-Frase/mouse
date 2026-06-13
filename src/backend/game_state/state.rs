use crate::backend::caches::{BETWEEN_TABLE, ZOBRIST_BLACK_LONG_CASTLE_RNGS, ZOBRIST_BLACK_SHORT_CASTLE_RNGS, ZOBRIST_BLACK_TO_MOVE, ZOBRIST_EP_RNGS, ZOBRIST_PIECES_RNGS, ZOBRIST_WHITE_LONG_CASTLE_RNGS, ZOBRIST_WHITE_SHORT_CASTLE_RNGS};
use crate::backend::constants::*;
use crate::backend::game_state::bb_manager::BBManager;
use crate::backend::game_state::fen_parser::parse_fen;
use crate::backend::game_state::irreversible_data::IrreversibleData;
use crate::backend::movegen::check_decider::is_in_check;
use crate::backend::movegen::move_gen::moves;
use crate::backend::movegen::move_gen_sliders::{get_slider_moves_at_square, get_slider_xray_moves_at_square};
use crate::backend::types::bitboard::BitBoard;
use crate::backend::types::moove::{CastleType, Moove};
use crate::backend::types::piece::Piece::{Bishop, King, Pawn, Queen, Rook};
use crate::backend::types::piece::Side::{Black, White};
use crate::backend::types::piece::{ALL_PIECES, ALL_SIDES, Side};
use crate::backend::types::square::{Square, back_by_one, get_file};

const ROOK_SWAP_WHITE_LONG_CASTLE_BB: BitBoard = BitBoard { value: 0x9 };
const ROOK_SWAP_WHITE_SHORT_CASTLE_BB: BitBoard = BitBoard { value: 0xa0 };
const ROOK_SWAP_BLACK_LONG_CASTLE_BB: BitBoard = BitBoard {
    value: 0x900000000000000,
};
const ROOK_SWAP_BLACK_SHORT_CASTLE_BB: BitBoard = BitBoard {
    value: 0xa000000000000000,
};

const WHITE_DOUBLE_PUSH_BB: BitBoard = BitBoard { value: 0xff000000 };
const BLACK_DOUBLE_PUSH_BB: BitBoard = BitBoard {
    value: 0xff00000000,
};

#[derive(Debug, Clone)]
pub struct State {
    pub bb_mngr: BBManager,
    pub irreversible_data: IrreversibleData,
    pub active_side: Side,
    pub half_move_clock: u16,
    pub zobrist_hash: u64,
    pub diag_pin_mask: BitBoard,
    pub straight_pin_mask: BitBoard,
}

// The core of state
impl State {
    pub fn new(
        bb_mngr: BBManager,
        active_side: Side,
        irreversible_data: IrreversibleData,
        half_move_clock: u16,
    ) -> State {
        let mut state = State {
            bb_mngr,
            active_side,
            irreversible_data,
            half_move_clock,
            zobrist_hash: 0,
            diag_pin_mask: BitBoard::new(),
            straight_pin_mask: BitBoard::new(),
        };

        // Piece positions
        for side in ALL_SIDES {
            for piece in ALL_PIECES {
                let piece_bb = state.bb_mngr.get_colored_piece_bb(piece, side);
                for square in piece_bb {
                    state.zobrist_hash ^=
                        ZOBRIST_PIECES_RNGS[square as usize][piece as usize][side as usize];
                }
            }
        }

        // EP file
        if let Some(ep_square) = state.irreversible_data.en_passant_square {
            let ep_file = get_file(ep_square);
            state.zobrist_hash ^= ZOBRIST_EP_RNGS[ep_file as usize];
        }

        // Castling rights
        state.irreversible_data.white_long_castle_rights.then(|| {
            state.zobrist_hash ^= ZOBRIST_WHITE_LONG_CASTLE_RNGS;
        });
        state.irreversible_data.white_short_castle_rights.then(|| {
            state.zobrist_hash ^= ZOBRIST_WHITE_SHORT_CASTLE_RNGS;
        });
        state.irreversible_data.black_long_castle_rights.then(|| {
            state.zobrist_hash ^= ZOBRIST_BLACK_LONG_CASTLE_RNGS;
        });
        state.irreversible_data.black_short_castle_rights.then(|| {
            state.zobrist_hash ^= ZOBRIST_BLACK_SHORT_CASTLE_RNGS;
        });

        // Side to move
        if state.active_side == Black {
            state.zobrist_hash ^= ZOBRIST_BLACK_TO_MOVE;
        }

        // Pin masks
        state.generate_pin_masks();

        state
    }

    /// Creates a new `GameState` instance based on the fen string.
    pub fn new_from_fen(fen_string: &str) -> State {
        let mut bb_manager = BBManager::new();
        let mut active_color = White;
        let mut irreversible_data = IrreversibleData::new();
        let mut half_move_clock = 0;

        parse_fen(
            fen_string,
            &mut bb_manager,
            &mut active_color,
            &mut irreversible_data,
            &mut half_move_clock,
        );

        State::new(bb_manager, active_color, irreversible_data, half_move_clock)
    }

    /// Executes a move.
    ///
    /// # Arguments
    ///
    /// * `chess_move` - A `Moove` object representing the move to be made.
    pub fn make_move(&self, moove: Moove) -> State {
        let mut next_state = self.clone();
        next_state.half_move_clock += 1;
        // The new irreversible data.
        let mut next_ir_data = IrreversibleData::new_from_previous_state(&self.irreversible_data);
        // Remove the previous ep file from the hash.
        if let Some(ep_square) = self.irreversible_data.en_passant_square {
            let ep_file = get_file(ep_square);
            next_state.zobrist_hash ^= ZOBRIST_EP_RNGS[ep_file as usize];
        }

        // Get the type of moved piece.
        let moved_piece = self.bb_mngr.get_piece_at_square(moove.get_from()).unwrap();

        // Usually the square something was captured on (if something was captured at all) is the square we moved to...
        let mut capture_square = moove.get_to();
        if moved_piece == Pawn {
            // This is a pawn move, reset the half-move clock.
            next_state.half_move_clock = 0;
            // ... unless this is an en passant capture, we then need to update the capture square.
            next_state.make_move_ep_capture(moove, &mut capture_square);
            // Check if a double pawn push was played and store the en passant file
            next_state.make_move_double_pawn_push(moove, &mut next_ir_data);
        }

        // If something was captured, remove the piece and update irreversible data.
        next_state.make_move_capture(&mut next_ir_data, capture_square);

        // Get the bitboard for the piece that was moved.
        let mut moved_piece_bb = next_state.bb_mngr.get_piece_bb_mut(moved_piece);

        // Clear the square that the piece was moved from.
        next_state.zobrist_hash ^= ZOBRIST_PIECES_RNGS[moove.get_from() as usize]
            [moved_piece as usize][self.active_side as usize];
        moved_piece_bb.clear_square(moove.get_from());

        // Update the moved piece bb if it was a pawn promotion
        let placed_piece = moove.get_promotion_type().unwrap_or(moved_piece);
        moved_piece_bb = next_state.bb_mngr.get_piece_bb_mut(placed_piece);

        // Fill the square it moved to.
        next_state.zobrist_hash ^= ZOBRIST_PIECES_RNGS[moove.get_to() as usize]
            [placed_piece as usize][self.active_side as usize];
        moved_piece_bb.fill_square(moove.get_to());

        // Also update the bitboard for the current side.
        next_state
            .bb_mngr
            .get_side_bb_mut(self.active_side)
            .fill_square(moove.get_to());
        next_state
            .bb_mngr
            .get_side_bb_mut(self.active_side)
            .clear_square(moove.get_from());

        // Some special king handling
        if moved_piece == King {
            next_state.make_move_king(moove, &mut next_ir_data);
        }

        if moved_piece == Rook {
            next_state.make_move_castling_rights_on_rook_move_or_capture(
                &mut next_ir_data,
                moove.get_from(),
                self.active_side,
            );
        }

        // Take care of some basics.
        next_state.zobrist_hash ^= ZOBRIST_BLACK_TO_MOVE;
        next_state.active_side = self.active_side.oppo();
        next_state.irreversible_data = next_ir_data;

        next_state.generate_pin_masks();

        next_state.remove_illegal_en_passant_if_pinned();

        next_state
    }

    fn generate_pin_masks(&mut self) {
        let (straight_pin_mask, diag_pin_mask) =
            self.build_pin_masks(
                self.bb_mngr.get_colored_piece_bb(King, self.active_side).next().unwrap(),
                self.bb_mngr.get_occupied_bb(),
                self.bb_mngr.get_side_bb(self.active_side.oppo()));
        self.straight_pin_mask = straight_pin_mask;
        self.diag_pin_mask = diag_pin_mask;
    }

    fn make_move_ep_capture(&mut self, moove: Moove, capture_square: &mut Square) {
        let ep_square = self.irreversible_data.en_passant_square;

        // if an en passant square exists
        if let Some(ep_square) = ep_square
            // and if we moved to the ep_square
            && ep_square == moove.get_to()
        {
            // update the captured square to the ep_square - offset
            *capture_square = back_by_one(moove.get_to(), self.active_side);
        }
    }

    fn make_move_capture(
        &mut self,
        irreversible_data: &mut IrreversibleData,
        capture_square: Square,
    ) {
        // Get the type of the captured piece if it exists.
        let captured_piece = self.bb_mngr.get_piece_at_square(capture_square);
        // Clear the square on the captured piece's bitboard if it exists.
        if let Some(captured_piece) = captured_piece {
            // Something was captured! Reset the half-move clock.
            self.half_move_clock = 0;
            // Store the captured piece type in the irreversible data.
            irreversible_data.captured_piece = Some(captured_piece);
            // Remove the captured piece from its bitboard.
            let captured_piece_bb = self.bb_mngr.get_piece_bb_mut(captured_piece);
            captured_piece_bb.clear_square(capture_square);
            self.bb_mngr
                .get_side_bb_mut(self.active_side.oppo())
                .clear_square(capture_square);

            self.zobrist_hash ^= ZOBRIST_PIECES_RNGS[capture_square as usize]
                [captured_piece as usize][self.active_side.oppo() as usize];

            // Remove castling rights if the captured piece was a rook on its starting square
            if captured_piece == Rook {
                self.make_move_castling_rights_on_rook_move_or_capture(
                    irreversible_data,
                    capture_square,
                    self.active_side.oppo(),
                )
            }
        }
    }

    fn make_move_double_pawn_push(
        &mut self,
        moove: Moove,
        irreversible_data: &mut IrreversibleData,
    ) {
        if moove.is_double_pawn_push() {
            // the pawn starting square and one forward
            let ep_square = back_by_one(moove.get_to(), self.active_side);
            let ep_file = get_file(ep_square);

            if self.is_ep_legal(ep_square) {
                self.zobrist_hash ^= ZOBRIST_EP_RNGS[ep_file as usize];
                irreversible_data.en_passant_square = Some(ep_square);
            }
        }
    }

    fn make_move_king(&mut self, moove: Moove, irreversible_data: &mut IrreversibleData) {
        // If the king moved we can't castle anymore
        match self.active_side {
            White => {
                if irreversible_data.white_long_castle_rights {
                    self.zobrist_hash ^= ZOBRIST_WHITE_LONG_CASTLE_RNGS;
                }
                if irreversible_data.white_short_castle_rights {
                    self.zobrist_hash ^= ZOBRIST_WHITE_SHORT_CASTLE_RNGS;
                }
            }
            Black => {
                if irreversible_data.black_long_castle_rights {
                    self.zobrist_hash ^= ZOBRIST_BLACK_LONG_CASTLE_RNGS;
                }
                if irreversible_data.black_short_castle_rights {
                    self.zobrist_hash ^= ZOBRIST_BLACK_SHORT_CASTLE_RNGS;
                }
            }
        }
        irreversible_data.remove_long_castle_rights(self.active_side);
        irreversible_data.remove_short_castle_rights(self.active_side);

        // If we castled, we need to move the rook
        if moove.is_castle() {
            let rook_bb = self.bb_mngr.get_piece_bb_mut(Rook);
            let (rook_swap_bb, from_zobrist, to_zobrist) =
                Self::get_rook_swap_bb(moove.get_castle_type(), self.active_side);
            self.zobrist_hash ^= from_zobrist;
            self.zobrist_hash ^= to_zobrist;
            *rook_bb ^= rook_swap_bb;
            let friendly_bb = self.bb_mngr.get_side_bb_mut(self.active_side);
            *friendly_bb ^= rook_swap_bb;
        }
    }

    fn get_rook_swap_bb(castle_type: CastleType, active_color: Side) -> (BitBoard, u64, u64) {
        match castle_type {
            CastleType::Long => match active_color {
                White => (
                    ROOK_SWAP_WHITE_LONG_CASTLE_BB,
                    ZOBRIST_PIECES_RNGS[A1 as usize][Rook as usize][White as usize],
                    ZOBRIST_PIECES_RNGS[D1 as usize][Rook as usize][White as usize],
                ),
                Black => (
                    ROOK_SWAP_BLACK_LONG_CASTLE_BB,
                    ZOBRIST_PIECES_RNGS[A8 as usize][Rook as usize][Black as usize],
                    ZOBRIST_PIECES_RNGS[D8 as usize][Rook as usize][Black as usize],
                ),
            },
            CastleType::Short => match active_color {
                White => (
                    ROOK_SWAP_WHITE_SHORT_CASTLE_BB,
                    ZOBRIST_PIECES_RNGS[H1 as usize][Rook as usize][White as usize],
                    ZOBRIST_PIECES_RNGS[F1 as usize][Rook as usize][White as usize],
                ),
                Black => (
                    ROOK_SWAP_BLACK_SHORT_CASTLE_BB,
                    ZOBRIST_PIECES_RNGS[H8 as usize][Rook as usize][Black as usize],
                    ZOBRIST_PIECES_RNGS[F8 as usize][Rook as usize][Black as usize],
                ),
            },
        }
    }

    fn make_move_castling_rights_on_rook_move_or_capture(
        &mut self,
        irreversible_data: &mut IrreversibleData,
        from_square: Square,
        active_side: Side,
    ) {
        for castling_type in CastleType::get_all_types() {
            let starting_square = Self::get_rook_starting_square(castling_type, active_side);
            if from_square == starting_square {
                // TODO: This could be improved by storing each castling right as 1, 2, 4, 8 and indexing via that.
                let zobrist_number = match castling_type {
                    CastleType::Long => match active_side {
                        Side::White => ZOBRIST_WHITE_LONG_CASTLE_RNGS,
                        Side::Black => ZOBRIST_BLACK_LONG_CASTLE_RNGS,
                    },
                    CastleType::Short => match active_side {
                        Side::White => ZOBRIST_WHITE_SHORT_CASTLE_RNGS,
                        Side::Black => ZOBRIST_BLACK_SHORT_CASTLE_RNGS,
                    },
                };
                if irreversible_data.get_castle_rights(active_side, castling_type) {
                    self.zobrist_hash ^= zobrist_number;
                }
                irreversible_data.remove_castle_rights(active_side, castling_type);
            }
        }
    }

    fn get_rook_starting_square(castle_type: CastleType, color: Side) -> Square {
        match castle_type {
            CastleType::Long => match color {
                Side::White => A1,
                Side::Black => A8,
            },
            CastleType::Short => match color {
                Side::White => H1,
                Side::Black => H8,
            },
        }
    }

    fn is_ep_legal(&self, en_passant_square: Square) -> bool {
        let active_side = self.active_side.oppo();
        let opponent_side = self.active_side;

        let double_push_rank = match active_side {
            Side::White => BLACK_DOUBLE_PUSH_BB,
            Side::Black => WHITE_DOUBLE_PUSH_BB,
        };

        let captured_pawn_square = match active_side {
            Side::White => en_passant_square - SIDE_LENGTH as u8,
            Side::Black => en_passant_square + SIDE_LENGTH as u8,
        };
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        let friendly_king = self.bb_mngr.get_colored_piece_bb(King, active_side);
        let opponent_sliders = self.bb_mngr.get_colored_piece_bb(Queen, opponent_side)
            | self.bb_mngr.get_colored_piece_bb(Rook, opponent_side);

        let friendly_pawns = self.bb_mngr.get_colored_piece_bb(Pawn, active_side);
        let left_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !LEFT_SIDE_BB) >> 1);
        let right_capturing_pawn = friendly_pawns & ((captured_pawn_bb & !RIGHT_SIDE_BB) << 1);

        if left_capturing_pawn.is_empty() && right_capturing_pawn.is_empty() {
            return false;
        }

        if (friendly_king & double_push_rank).is_empty()
            || (opponent_sliders & double_push_rank).is_empty()
        {
            return true;
        }

        let friendly_occupancy = self.bb_mngr.get_side_bb(active_side);
        let opponent_occupancy = self.bb_mngr.get_side_bb(opponent_side) & !captured_pawn_bb;

        let king_square = friendly_king.clone().next().unwrap();

        let would_expose_king_to_slider = |capturing_pawn: BitBoard| {
            if capturing_pawn.is_empty() {
                return false;
            }

            let king_slider_rays = get_slider_moves_at_square::<true>(
                king_square,
                friendly_occupancy & !capturing_pawn,
                opponent_occupancy,
            );

            (king_slider_rays & opponent_sliders).is_not_empty()
        };

        !would_expose_king_to_slider(left_capturing_pawn)
            && !would_expose_king_to_slider(right_capturing_pawn)
    }


    fn build_pin_masks(
        &self,
        king_square: Square,
        occupied_bb: BitBoard,
        enemy_pieces_bb: BitBoard,
    ) -> (BitBoard, BitBoard) {
        let straight_xray_bb = get_slider_xray_moves_at_square::<true>(king_square, occupied_bb);
        let diag_xray_bb = get_slider_xray_moves_at_square::<false>(king_square, occupied_bb);

        let straight_xray_attackers_bb = straight_xray_bb
            & (self.bb_mngr.get_piece_bb(Rook) | self.bb_mngr.get_piece_bb(Queen))
            & enemy_pieces_bb;

        let diag_xray_attackers_bb = diag_xray_bb
            & (self.bb_mngr.get_piece_bb(Bishop) | self.bb_mngr.get_piece_bb(Queen))
            & enemy_pieces_bb;

        (
            self.build_pin_mask(straight_xray_attackers_bb, king_square),
            self.build_pin_mask(diag_xray_attackers_bb, king_square),
        )
    }

    fn build_pin_mask(&self, xray_attackers_bb: BitBoard, king_square: Square) -> BitBoard {
        let mut pin_mask = BitBoard { value: 0 };

        for attacker_square in xray_attackers_bb {
            // The order of indices is important:
            // the attacker square is included, while the king square is not.
            // This keeps capturing the pinning piece legal.
            pin_mask |= BETWEEN_TABLE[attacker_square as usize][king_square as usize];
        }

        pin_mask
    }

    // TODO: THIS BREAKS MY ZOBRIST HASHING. FIX IT.
    fn remove_illegal_en_passant_if_pinned(&mut self) {
        let Some(en_passant_square) = self.irreversible_data.en_passant_square else {
            return;
        };

        let ep_file = get_file(en_passant_square);

        let captured_pawn_square = back_by_one(en_passant_square, self.active_side);
        let captured_pawn_bb = BitBoard::new_from_square(captured_pawn_square);

        if (captured_pawn_bb & self.diag_pin_mask).is_not_empty() {
            self.zobrist_hash ^= ZOBRIST_EP_RNGS[ep_file as usize];
            self.irreversible_data.en_passant_square = None;
        }
    }

}

// A bunch of API helpers
impl State {
    pub fn is_in_check(&self) -> bool {
        is_in_check(self)
    }

    pub fn gen_moves(&mut self) -> Vec<Moove> {
        moves(self, false)
    }

    pub fn gen_attacks(&mut self) -> Vec<Moove> {
        moves(self, true)
    }
}

// zobrist tests - AI generated use with a bucket of salt
fn recomputed_hash(state: &State) -> u64 {
    State::new(
        state.bb_mngr.clone(),
        state.active_side,
        state.irreversible_data.clone(),
        state.half_move_clock,
    )
    .zobrist_hash
}

fn assert_hash_is_recomputable(state: &State) {
    assert_eq!(
        state.zobrist_hash,
        recomputed_hash(state),
        "incremental zobrist hash differs from freshly recomputed hash for state:\n{state:#?}",
    );
}

fn play_uci_sequence(fen: &str, moves: &[&str]) -> State {
    let mut state = State::new_from_fen(fen);
    assert_hash_is_recomputable(&state);

    for uci_move in moves {
        let moove = Moove::moove_from_uci_notation(uci_move);
        state = state.make_move(moove);

        assert_eq!(
            state.zobrist_hash,
            recomputed_hash(&state),
            "zobrist mismatch after move {uci_move} in sequence {moves:?}",
        );
    }

    state
}

#[test]
fn zobrist_matches_recomputed_hash_after_quiet_moves() {
    play_uci_sequence(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &["e2e4", "e7e5", "g1f3", "b8c6", "f1b5", "g8f6"],
    );
}

#[test]
fn zobrist_matches_recomputed_hash_after_captures() {
    play_uci_sequence(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &["e2e4", "d7d5", "e4d5", "d8d5", "b1c3", "d5g2"],
    );
}

#[test]
fn zobrist_matches_recomputed_hash_after_promotion() {
    play_uci_sequence("4k3/P7/8/8/8/8/8/4K3 w - - 0 1", &["a7a8q"]);
}

#[test]
fn zobrist_matches_recomputed_hash_after_promotion_capture() {
    play_uci_sequence("1r2k3/P7/8/8/8/8/8/4K3 w - - 0 1", &["a7b8q"]);
}

#[test]
fn zobrist_matches_recomputed_hash_after_en_passant_capture() {
    play_uci_sequence(
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        &["e2e4", "a7a6", "e4e5", "d7d5", "e5d6"],
    );
}

#[test]
fn zobrist_matches_recomputed_hash_after_short_castling() {
    play_uci_sequence("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", &["e1g1", "e8g8"]);
}

#[test]
fn zobrist_matches_recomputed_hash_after_long_castling() {
    play_uci_sequence("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", &["e1c1", "e8c8"]);
}

#[test]
fn zobrist_matches_recomputed_hash_after_rook_move_removes_castling_rights() {
    play_uci_sequence("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", &["h1h2", "a8a7"]);
}

#[test]
fn zobrist_matches_recomputed_hash_after_rook_capture_removes_castling_rights() {
    play_uci_sequence("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1", &["a1a8"]);
}

#[test]
fn zobrist_hash_changes_with_side_to_move() {
    let white_to_move = State::new_from_fen("8/8/8/8/8/8/8/4K2k w - - 0 1");
    let black_to_move = State::new_from_fen("8/8/8/8/8/8/8/4K2k b - - 0 1");

    assert_hash_is_recomputable(&white_to_move);
    assert_hash_is_recomputable(&black_to_move);

    assert_ne!(
        white_to_move.zobrist_hash, black_to_move.zobrist_hash,
        "side to move should affect the zobrist hash",
    );
}

#[test]
fn zobrist_hash_changes_with_castling_rights() {
    let no_rights = State::new_from_fen("r3k2r/8/8/8/8/8/8/R3K2R w - - 0 1");
    let all_rights = State::new_from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1");

    assert_hash_is_recomputable(&no_rights);
    assert_hash_is_recomputable(&all_rights);

    assert_ne!(
        no_rights.zobrist_hash, all_rights.zobrist_hash,
        "castling rights should affect the zobrist hash",
    );
}

#[test]
fn zobrist_hash_changes_with_en_passant_file() {
    let no_ep = State::new_from_fen("8/8/8/3pP3/8/8/8/4K2k w - - 0 1");
    let ep = State::new_from_fen("8/8/8/3pP3/8/8/8/4K2k w - d6 0 1");

    assert_hash_is_recomputable(&no_ep);
    assert_hash_is_recomputable(&ep);

    assert_ne!(
        no_ep.zobrist_hash, ep.zobrist_hash,
        "en passant file should affect the zobrist hash when stored in the state",
    );
}
