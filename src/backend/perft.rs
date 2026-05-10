use crate::backend::movegen::move_gen::moves;
use crate::backend::game_state::state::State;

pub fn perft(state: &mut State, depth: u8) -> u64 {
    /*
    if depth == 0 {
        return 1;
    }
     */

    let moves = moves(state);
    if depth == 1 {
        return moves.len() as u64;
    }
    
    let mut nodes = 0;
    for chess_move in moves {
        let mut next_state = state.make_move(chess_move);
        nodes += perft(&mut next_state, depth - 1);
    }

    nodes
}
