use crate::backend::movegen::move_gen::moves;
use crate::backend::game_state::state::State;

pub fn perft(state: &mut State, depth: u8) -> u64 {
    // PERFT: I would love to make this work, but it does not atm.
    // let moves = get_moves(game_state);
    // if depth == 1 {
    //     return moves.len() as u64;
    // }
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
        // If we are in check after making the move -> skip.
        /*
        if is_in_check(&next_state, next_state.active_side.oppo()) {
            continue;
        }
         */

        nodes += perft(&mut next_state, depth - 1);
    }

    nodes
}
