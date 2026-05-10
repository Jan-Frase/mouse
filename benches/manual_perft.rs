use mouse::State;
use mouse::backend::perft::perft;
use perft_fixtures::perft_fixtures::{LONG_PERFT, PerftFixture};
use std::time::Instant;

pub fn manual_perft() {
    let mut total_nodes = 0;

    let now = Instant::now();
    for perft_fixture in LONG_PERFT {
        let nodes = run_nps_perft(perft_fixture);
        total_nodes += nodes;
    }

    let elapsed = now.elapsed();
    let total_nodes_per_second = total_nodes as f64 / elapsed.as_secs_f64();

    println!();
    println!("Total nodes: {:?}", total_nodes);
    println!("Total time: {:.1}s", elapsed.as_secs_f64());
    println!("Total nodes per second: {:.0}", total_nodes_per_second);
}

// --------------------------------------------- //
// PERFT with nps
// --------------------------------------------- //

fn run_nps_perft(perft_fixture: PerftFixture) -> u64 {
    let fen = perft_fixture.perft_setup.fen;
    let mut state = State::new_from_fen(fen);

    // Start timer to calculate nodes per second.
    let now = Instant::now();

    let nodes = perft(&mut state, perft_fixture.depth);

    let elapsed = now.elapsed();
    let nodes_per_second = nodes as f64 / elapsed.as_secs_f64();
    println!(
        "Name: {:?}, Nps: {:.0}.",
        perft_fixture.perft_setup.name, nodes_per_second
    );

    nodes
}

pub fn main() {
    manual_perft();
}
