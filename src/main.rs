fn gauss_legendre(n: i32) -> f64 {
    let mut a = 1.0;
    let mut b = 1.0 / f64::sqrt(2.0);
    let mut t = 1.0 / 4.0;
    let mut p = 1.0;

    for _ in 0..n {
        let a_next = (a + b) / 2.0;
        let b_next = f64::sqrt(a * b);
        let t_next = t - p * (a - a_next).powi(2);
        a = a_next;
        b = b_next;
        t = t_next;
        p *= 2.0;
    }

    (a + b).powi(2) / (4.0 * t)
}

fn main() {
    let n = 5; // Number of iterations
    let pi_approx = gauss_legendre(n);
    println!("pi approx: {}", pi_approx);
}
