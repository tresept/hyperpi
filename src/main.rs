use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use std::time::Instant;

fn main() {
    // 計算する桁数
    let digits = 0;

    // 必要なビット精度を計算
    // 桁数 * log2(10) + 10(誤差補正用)
    let precision = (digits as f64 * 10.0_f64.log2() + 10.0) as usize;

    let start = Instant::now();
    let _ = calculate_pi(precision);
    let duration = start.elapsed();

    println!("Calculated {} digits of pi in {:?}", digits, duration);

    // 結果の一部を小数点型で表示
    // let pi_decimal = pi.to_decimal().value();
    // let pi_str = pi_decimal.to_string();
    // let display_len = pi_str.len().min(102); // 3.14... plus some digits
    // println!(
    //     "Pi (first {} chars) = {}",
    //     display_len,
    //     &pi_str[..display_len]
    // );
}

/// Gauss-Legendre法で円周率を計算する関数
/// https://qiita.com/matsuda_tkm/items/418588d3c59cc8d85ec7
/// https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
fn calculate_pi(precision: usize) -> FBig<HalfEven, 2> {
    // 初期値を設定
    let one = FBig::<HalfEven, 2>::ONE.with_precision(precision).value();
    let two = FBig::<HalfEven, 2>::from(2u8)
        .with_precision(precision)
        .value();
    let four = FBig::<HalfEven, 2>::from(4u8)
        .with_precision(precision)
        .value();

    let mut a = one.clone();
    let mut b = (&one / &two.sqrt()).with_precision(precision).value();
    let mut t = &one / &four;
    let mut p = one.clone();

    // 2次収束なので だいたいlog2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as u32).max(10);

    for _ in 0..iterations {
        let a_next = ((&a + &b) / &two).with_precision(precision).value();
        let b_next = (&a * &b).sqrt().with_precision(precision).value();
        let a_diff = &a - &a_next;
        let t_next = &t - &(&p * &a_diff * &a_diff);

        a = a_next;
        b = b_next;
        t = t_next;
        p = (&p * &two).with_precision(precision).value();
    }

    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;

    (&numerator / &denominator)
        .with_precision(precision)
        .value()
}
