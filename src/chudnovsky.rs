use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

use std::time::{Duration, Instant};

type BinFloat = FBig<HalfEven, 2>;

struct Pqr {
    p: IBig,
    q: IBig,
    r: IBig,
}

fn bs(a: i64, b: i64, c3: &IBig) -> Pqr {
    if b - a == 1 {
        let k = IBig::from(a);
        let a_val = IBig::from(13591409);
        let b_val = IBig::from(545140134);

        if a == 0 {
            // 第0項はシンプルに A / 1
            return Pqr {
                p: IBig::from(1),
                q: a_val,
                r: IBig::from(1),
            };
        }

        // 第1項以降のパーツ (P/R が比率を表す)
        // 係数を整理すると、この形が一番整数計算しやすいです
        let p: IBig =
            (IBig::from(6) * &k - 5) * (IBig::from(2) * &k - 1) * (IBig::from(6) * &k - 1) * -1;
        let r = k.pow(3) * c3 / 24; // Chudnovskyの公式を整数用に整理した形
        let q = &p * (a_val + b_val * &k);

        Pqr { p, q, r }
    } else {
        let m = (a + b) / 2;
        let left = bs(a, m, c3);
        let right = bs(m, b, c3);

        // 合体ルール！
        Pqr {
            q: &left.q * &right.r + &left.p * &right.q,
            p: &left.p * &right.p,
            r: &left.r * &right.r,
        }
    }
}

/// Chudnovsky法で円周率を計算
/// * digits: 求めたい10進数の桁数
/// * precision: 内部計算に使う2進数の精度（ビット数）
pub fn calc_chudnovsky(digits: usize, precision: usize) -> BinFloat {
    // 必要な項数
    let n = digits / 14 + 1;

    // let a = IBig::from(13591409);
    // let b = IBig::from(545140134);
    let c = IBig::from(640320);
    let c3 = &c * &c * &c;

    let res = bs(0, n as i64, &c3);

    // 高精度な浮動小数点数に変換
    let sum_num_float = BinFloat::from(res.q).with_precision(precision).value();
    let sum_den_float = BinFloat::from(res.r).with_precision(precision).value();
    let c_float = BinFloat::from(c.clone()).with_precision(precision).value();

    // 高精度な √C を求める
    let c_sqrt = c_float.sqrt().with_precision(precision).value();

    // Pi = (C * √C * sum_den) / (12 * sum_num)
    let twelve = BinFloat::from(12u8).with_precision(precision).value();
    let numerator = (&c_float * &c_sqrt * &sum_den_float)
        .with_precision(precision)
        .value();
    let denominator = (&twelve * &sum_num_float).with_precision(precision).value();

    (&numerator / &denominator)
        .with_precision(precision)
        .value()
}
