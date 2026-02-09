use dashu::base::SquareRoot;
use dashu::float::{FBig, round::mode::HalfEven};
use dashu::integer::IBig;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bs_first_term() {
        // 第0項のテスト
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let result = bs(0, 1, &c3);

        assert_eq!(result.p, IBig::from(1));
        assert_eq!(result.q, IBig::from(13591409));
        assert_eq!(result.r, IBig::from(1));
    }

    #[test]
    fn test_bs_second_term() {
        // 第1項のテスト
        let c3 = IBig::from(640320) * IBig::from(640320) * IBig::from(640320);
        let result = bs(1, 2, &c3);

        // p = (6*1 - 5) * (2*1 - 1) * (6*1 - 1) * -1 = 1 * 1 * 5 * -1 = -5
        assert_eq!(result.p, IBig::from(-5));
        assert!(result.r > IBig::from(0)); // rは正の値
    }

    #[test]
    fn test_calc_chudnovsky_basic() {
        // 10桁の計算をテスト
        let digits = 10;
        let precision = 128; // 十分な精度

        let pi = calc_chudnovsky(digits, precision);

        // πが妥当な範囲にあることを確認（3 < π < 4）
        let three = BinFloat::from(3i32);
        let four = BinFloat::from(4i32);
        assert!(pi > three);
        assert!(pi < four);
    }

    #[test]
    fn test_calc_chudnovsky_precision() {
        // より高精度でπを計算
        let digits = 50;
        let precision = 256;

        let pi = calc_chudnovsky(digits, precision);

        // πが妥当な範囲にあることを確認（3.14 < π < 3.15）
        let lower = (BinFloat::from(314i32) / BinFloat::from(100i32))
            .with_precision(precision)
            .value();
        let upper = (BinFloat::from(315i32) / BinFloat::from(100i32))
            .with_precision(precision)
            .value();
        assert!(pi > lower);
        assert!(pi < upper);
    }

    #[test]
    fn test_calc_chudnovsky_different_digits() {
        // 異なる桁数で計算が完了することを確認
        let test_cases = vec![5, 10, 20, 30];

        for digits in test_cases {
            let precision = digits * 4; // 十分な精度を確保
            let pi = calc_chudnovsky(digits, precision);

            // 結果が妥当な範囲にあることを確認（3 < π < 3.2）
            let three = BinFloat::from(3i32);
            let three_point_two = (BinFloat::from(32i32) / BinFloat::from(10i32))
                .with_precision(precision)
                .value();
            assert!(pi > three);
            assert!(pi < three_point_two);
        }
    }

    #[test]
    fn test_pqr_structure() {
        // Pqr構造体が正しく作成できることを確認
        let pqr = Pqr {
            p: IBig::from(1),
            q: IBig::from(2),
            r: IBig::from(3),
        };

        assert_eq!(pqr.p, IBig::from(1));
        assert_eq!(pqr.q, IBig::from(2));
        assert_eq!(pqr.r, IBig::from(3));
    }
}
