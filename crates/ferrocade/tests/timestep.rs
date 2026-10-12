use ferrocade::timestep::FixedTimestep;

const STEP: f64 = 1.0 / 60.0;

#[test]
fn one_step_per_step_duration() {
    let mut ts = FixedTimestep::new(STEP);
    assert_eq!(ts.advance(STEP), 1);
}

#[test]
fn zero_elapsed_runs_nothing() {
    let mut ts = FixedTimestep::new(STEP);
    assert_eq!(ts.advance(0.0), 0);
}

#[test]
fn fractions_accumulate_across_frames() {
    let mut ts = FixedTimestep::new(STEP);
    assert_eq!(ts.advance(STEP * 0.6), 0);
    assert_eq!(ts.advance(STEP * 0.6), 1);
    assert_eq!(ts.advance(STEP * 0.6), 0);
    assert_eq!(ts.advance(STEP * 0.6), 1);
}

#[test]
fn long_frame_is_capped_and_excess_dropped() {
    let mut ts = FixedTimestep::new(STEP);
    assert_eq!(ts.advance(0.5), 5);
    // The backlog was discarded: a normal frame afterwards runs a normal number of updates.
    assert_eq!(ts.advance(STEP), 1);
}
