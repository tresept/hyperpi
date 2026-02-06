use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use std::fs;
use std::io::Write;
use std::time::Duration;
use std::time::Instant;

type BigFloat = FBig<HalfEven, 10>;

fn main() {
    // 計算する桁数
    let digits = 1_040_000;

    // 必要なビット精度を計算
    // 桁数 + 100(誤差補正用)
    let precision = digits + 100;

    // 必要なループ回数を計算
    // Gauss-Legendre法は2次収束なので、だいたい log2(precision) 回繰り返せば十分
    let iterations = ((precision as f64).log2().ceil() as usize).max(10);

    println!("{}桁の計算開始．{}回のループとなります", digits, iterations);
    let (pi, duration) = gauss_legendre_algorithm(precision, iterations);
    println!(
        "{} 桁の円周率計算を完了しました(実行時間: {:?})",
        digits, duration
    );

    // 結果をファイルに保存
    let mut file = fs::File::create("pi.txt").expect("ファイルの作成に失敗しました");
    let pi_str = pi.to_string();
    file.write_all(pi_str.as_bytes())
        .expect("ファイルへの書き込みに失敗しました");
    eprintln!("結果を pi.txt に保存しました");
}

/// Gauss-Legendre法で円周率を計算する関数
/// https://qiita.com/matsuda_tkm/items/418588d3c59cc8d85ec7
/// https://ja.wikipedia.org/wiki/ガウス＝ルジャンドルのアルゴリズム
fn gauss_legendre_algorithm(precision: usize, iterations: usize) -> (BigFloat, Duration) {
    let start = Instant::now();
    // 初期値を設定
    let one = BigFloat::ONE.with_precision(precision).unwrap();
    let two = BigFloat::from(2u8).with_precision(precision).unwrap();
    let four = BigFloat::from(4u8).with_precision(precision).unwrap();

    let mut a = one.clone();
    let mut b = (&one / &two.sqrt()).with_precision(precision).unwrap();
    let mut t = &one / &four;
    let mut p = one.clone();

    eprintln!("初期値の計算が終了しました: {:?}", start.elapsed());

    for i in 0..iterations {
        let a_next = ((&a + &b) / &two).with_precision(precision).unwrap();
        let b_next = (&a * &b).sqrt().with_precision(precision).unwrap();
        let a_diff = &a - &a_next;
        let t_next = &t - &(&p * &a_diff * &a_diff);

        a = a_next;
        b = b_next;
        t = t_next;
        p = (&p * &two).with_precision(precision).unwrap();

        let elapsed = start.elapsed();

        eprintln!(
            "{} 回目のループが終了しました: 経過時間: {:?}",
            i + 1,
            elapsed
        );
    }

    let sum = &a + &b;
    let numerator = &sum * &sum;
    let denominator = &four * &t;

    let duration = start.elapsed();

    (
        (&numerator / &denominator)
            .with_precision(precision)
            .unwrap(),
        duration,
    )
}
