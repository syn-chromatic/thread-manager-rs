pub fn estimate_pi_leibniz(terms: usize) -> f64 {
    let mut pi: f64 = 0.0;

    for idx in 0..terms {
        let divisor: f64 = 2.0 * idx as f64 + 1.0;
        let term: f64 = if idx % 2 == 0 {
            1.0 / divisor
        } else {
            -1.0 / divisor
        };

        pi += term;
    }

    4.0 * pi
}
