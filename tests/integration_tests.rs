use mouse::State;
use mouse::backend::perft::perft;
use perft_fixtures::perft_fixtures::{FAST_PERFT, LONG_PERFT, NORMAL_PERFT, PerftFixture, LONGER_PERFT};

#[test]
fn test_perft_fast() {
    test_perft_fixtures(&FAST_PERFT);
}

#[test]
fn test_perft_normal() {
    test_perft_fixtures(&NORMAL_PERFT);
}

#[test]
// This test takes quite a while. Thus, it's disabled by default.
// Can be run using 'cargo test -- --ignored'.
fn test_perft_long() {
    test_perft_fixtures(&LONG_PERFT);
}

#[test]
fn test_perft_longer() {
    test_perft_fixtures(&LONGER_PERFT);
}

fn test_perft_fixtures(perft_fixtures: &[PerftFixture]) {
    for perft_fixture in perft_fixtures {
        test_single_perft_fixture(perft_fixture);
    }
}

fn test_single_perft_fixture(perft_fixture: &PerftFixture) {
    let fen = perft_fixture.perft_setup.fen;
    let depth = perft_fixture.depth;
    let expected_nodes = perft_fixture.expected_nodes;
    let name = perft_fixture.perft_setup.name.to_owned();

    println!("Testing {:?} with {:?} nodes", name, expected_nodes);

    let mut state = State::new_from_fen(fen);
    let nodes = perft(&mut state, depth);

    assert_eq!(nodes, expected_nodes, "Testing {:?}", name);
}

// --------------------------------------------- //
// TESTING
// --------------------------------------------- //
#[test]
fn test_perft_01() {
    let mut state = State::new_from_fen("8/1n2k3/5n1n/2n5/4N3/2N5/1N2KN2/8 w - - 0 1");
    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 472915);

    // This currently takes too long to run.
    // let nodes = perft(&state, 5);
    // assert_eq!(nodes, 11949411);
}

#[test]
fn test_perft_02() {
    let mut state = State::new_from_fen("4k3/pppppppp/8/8/8/8/PPPPPPPP/4K3 w - - 0 1");
    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 98766);
}

#[test]
fn test_perft_03() {
    let mut state = State::new_from_fen("4k3/ppp5/7p/8/8/8/PPP5/4K3 w - - 0 1");

    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 17684);
}

#[test]
fn test_perft_04() {
    let mut state = State::new_from_fen("4k3/ppp5/7p/8/8/8/PPP5/4K3 w - - 0 1");

    let nodes = perft(&mut state, 5);
    assert_eq!(nodes, 197056);
}

#[test]
fn test_perft_05() {
    let mut state = State::new_from_fen("7k/3p4/8/2P5/8/8/8/7K b - - 0 1");

    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 896);

    let nodes = perft(&mut state, 5);
    assert_eq!(nodes, 6583);
}

#[test]
fn test_perft_06() {
    let mut state = State::new_from_fen("7k/8/8/8/8/2K5/2P5/8 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 7);
}

#[test]
fn test_perft_07() {
    let mut state = State::new_from_fen("8/3P1k2/8/8/8/8/8/7K b - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 7);

    let nodes = perft(&mut state, 2);
    assert_eq!(nodes, 49);

    // Missing slider logic atm
    // let nodes = perft(&game_state, 3);
    // assert_eq!(nodes, 289);
}

#[test]
fn test_perft_08() {
    let mut state = State::new_from_fen("8/1ppP1k2/1n6/3P2P1/8/8/8/7K b - - 0 1");

    let nodes = perft(&mut state, 2);
    assert_eq!(nodes, 117);
}

#[test]
fn test_perft_09() {
    let mut state = State::new_from_fen("7k/P7/8/8/8/8/8/7K w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 7);
}

#[test]
fn test_perft_10() {
    let mut state = State::new_from_fen("6k1/8/8/8/8/8/8/Q1R1B2K b - - 0 1");

    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 15773);
}

#[test]
fn test_perft_11() {
    let mut state = State::new_from_fen("6k1/6Q1/8/8/8/8/8/2R1B2K b - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 1);
}

#[test]
fn test_perft_12() {
    let mut state = State::new_from_fen("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1");

    let nodes = perft(&mut state, 5);
    assert_eq!(nodes, 4865609);
}

#[test]
fn test_perft_13() {
    // test if basic castling moves get generated
    let mut state = State::new_from_fen("6k1/8/8/8/8/8/8/R3K2R w KQ - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 26);
}

#[test]
fn test_perft_14() {
    // test if basic castling moves get generated - with checkers
    let mut state = State::new_from_fen("2r2rk1/8/8/8/8/8/8/R3K2R w KQ - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 22);
}

#[test]
fn test_perft_15() {
    // test if basic castling moves get generated - with blockers
    let mut state = State::new_from_fen("2r2rk1/8/8/8/8/8/8/R1B1K1NR w KQ - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 28);
}

#[test]
fn test_perft_16() {
    // test if basic castling moves get made and unmade correctly
    let mut state = State::new_from_fen("6k1/8/8/8/8/8/8/R3K2R w KQ - 0 1");

    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 10438);
}
#[test]
fn test_perft_17() {
    // test if basic castling moves get made and unmade correctly
    let mut state = State::new_from_fen("r5k1/8/8/8/8/8/8/R3K2R w KQ - 0 1");

    let nodes = perft(&mut state, 4);
    assert_eq!(nodes, 112371);
}

#[test]
fn test_perft_18() {
    let mut state = State::new_from_fen("4k3/8/8/p7/1P6/8/8/4K3 b - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 7);
}

#[test]
fn test_perft_19() {
    // Simple test for double checks :)
    let mut state = State::new_from_fen("k3rR2/8/8/5n2/8/4K3/8/8 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 5);
}

// Number 20 was an illegal fen 1k4q1/8/8/3pP3/8/1K6/8/8 w - D6 0 1
// It confused me, it's gone now.

#[test]
fn test_perft_21() {
    let mut state = State::new_from_fen("4k3/3p4/8/8/B7/8/8/K7 b - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 4);
}

#[test]
fn test_perft_22() {
    let mut state = State::new_from_fen("1q4qk/8/8/8/1PQ5/1K6/8/8 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 11);
}

#[test]
fn test_perft_23() {
    let mut state = State::new_from_fen("1q4qk/8/8/1p6/2P5/1K6/8/8 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 6);
}

#[test]
fn test_perft_25() {
    let mut state = State::new_from_fen("4k3/8/8/q7/8/2N5/8/4K3 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 5);
}

#[test]
fn test_perft_26() {
    let mut state = State::new_from_fen("4k3/8/8/8/8/5q2/5R2/r2Q1K2 w - - 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 7);
}

#[test]
fn test_perft_27() {
    let mut state = State::new_from_fen("8/8/8/1Ppp3r/RK3p1k/8/4P1P1/8 w - c6 0 1");

    let nodes = perft(&mut state, 1);
    assert_eq!(nodes, 6);
}
