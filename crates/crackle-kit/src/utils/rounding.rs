/// Rounds like Python 3 `round(value, ndigits)` using banker's rounding
/// (ties to nearest even).
///
/// - `ndigits == 0`: rounds to an integer-valued `f64` with ties-to-even.
/// - `ndigits > 0`: rounds to the right of the decimal point.
/// - `ndigits < 0`: rounds to the left of the decimal point.
/// - `NaN` and `±inf` are returned unchanged.
///
/// # Examples
///
/// ```ignore
/// assert_eq!(py_round(2.5, 0), 2.0);
/// assert_eq!(py_round(3.5, 0), 4.0);
/// assert_eq!(py_round(2.675, 2), 2.67);
/// assert_eq!(py_round(1250.0, -2), 1200.0);
/// ```
pub(crate) fn py_round(value: f64, ndigits: i32) -> f64 {
    if !value.is_finite() {
        return value;
    }

    if ndigits == 0 {
        return value.round_ties_even();
    }

    if ndigits > 0 {
        if ndigits > 308 {
            return value;
        }

        let repr = format!("{:.*}", ndigits as usize, value);
        if let Ok(parsed) = repr.parse::<f64>() {
            return parsed;
        }

        let scale = 10_f64.powi(ndigits);
        if !scale.is_finite() || scale == 0.0 {
            return value;
        }

        return (value * scale).round_ties_even() / scale;
    }

    let scale = 10_f64.powi(-ndigits);
    if !scale.is_finite() || scale == 0.0 {
        return value;
    }

    let shifted = value / scale;
    if !shifted.is_finite() {
        return value;
    }

    shifted.round_ties_even() * scale
}

/// Convenience wrapper for `py_round(value, 0)`.
///
/// Uses Python 3 style banker's rounding (ties to nearest even) at the integer
/// digit.
pub(crate) fn py_round0(value: f64) -> f64 {
    py_round(value, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    use proptest::prelude::*;

    const PY_ORACLE_SCRIPT: &str = r#"
import math
import struct
import sys

for raw in sys.stdin:
    raw = raw.rstrip("\n")
    if not raw:
        continue
    bits_s, nd_s = raw.split("\t")
    bits = int(bits_s)
    nd = int(nd_s)
    x = struct.unpack(">d", bits.to_bytes(8, "big", signed=False))[0]
    r = round(x, nd)
    if math.isnan(r):
        print("nan")
    elif math.isinf(r):
        print("inf" if r > 0 else "-inf")
    else:
        rb = struct.unpack(">Q", struct.pack(">d", r))[0]
        print(str(rb))
"#;

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum OracleValue {
        Bits(u64),
        NaN,
        PosInf,
        NegInf,
    }

    fn should_allow_one_ulp(value: f64, ndigits: i32) -> bool {
        const INTEGER_EXACT_LIMIT: f64 = 9_007_199_254_740_992.0;
        ndigits < 0 && value.abs() >= INTEGER_EXACT_LIMIT
    }

    fn run_python_oracle(cases: &[(f64, i32)]) -> Vec<OracleValue> {
        let mut child = Command::new("python3")
            .arg("-c")
            .arg(PY_ORACLE_SCRIPT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("python3 interpreter is required for oracle tests");

        {
            let stdin = child
                .stdin
                .as_mut()
                .expect("failed to open python stdin");

            for (value, ndigits) in cases {
                let line = format!("{}\t{}\n", value.to_bits(), ndigits);
                stdin
                    .write_all(line.as_bytes())
                    .expect("failed to write oracle case to python stdin");
            }
        }

        let output = child
            .wait_with_output()
            .expect("failed to read python oracle output");

        assert!(
            output.status.success(),
            "python oracle failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8(output.stdout).expect("python output must be valid UTF-8");
        let lines: Vec<&str> = stdout.lines().collect();

        assert_eq!(
            lines.len(),
            cases.len(),
            "python oracle output length mismatch"
        );

        lines
            .iter()
            .map(|line| match *line {
                "nan" => OracleValue::NaN,
                "inf" => OracleValue::PosInf,
                "-inf" => OracleValue::NegInf,
                _ => OracleValue::Bits(
                    line.parse::<u64>()
                        .expect("python oracle emitted invalid numeric bits"),
                ),
            })
            .collect()
    }

    fn assert_matches_python(cases: &[(f64, i32)]) {
        let expected = run_python_oracle(cases);

        for ((value, ndigits), oracle) in cases.iter().zip(expected.iter()) {
            let actual = py_round(*value, *ndigits);

            match oracle {
                OracleValue::NaN => {
                    assert!(
                        actual.is_nan(),
                        "expected NaN from python for value={value:?}, ndigits={ndigits}, got {actual:?}"
                    );
                }
                OracleValue::PosInf => {
                    assert_eq!(
                        actual,
                        f64::INFINITY,
                        "expected +inf from python for value={value:?}, ndigits={ndigits}"
                    );
                }
                OracleValue::NegInf => {
                    assert_eq!(
                        actual,
                        f64::NEG_INFINITY,
                        "expected -inf from python for value={value:?}, ndigits={ndigits}"
                    );
                }
                OracleValue::Bits(bits) => {
                    let actual_bits = actual.to_bits();
                    if actual_bits != *bits {
                        let one_ulp_ok = should_allow_one_ulp(*value, *ndigits)
                            && actual.is_finite()
                            && f64::from_bits(*bits).is_finite()
                            && (actual_bits.abs_diff(*bits) == 1);

                        assert!(
                            one_ulp_ok,
                            "python mismatch for value={value:?}, ndigits={ndigits}, actual={actual:?}, actual_bits={actual_bits}, expected_bits={bits}"
                        );
                    }
                }
            }
        }
    }

    fn deterministic_cases() -> Vec<(f64, i32)> {
        let mut cases = Vec::new();

        for ndigits in -6..=6 {
            for k in (-1500..=1500).step_by(37) {
                cases.push((k as f64 / 8.0, ndigits));
            }
        }

        for ndigits in 0..=6 {
            let scale = 10_f64.powi(ndigits);
            for k in (-500..=500).step_by(17) {
                let tie = (k as f64 + 0.5) / scale;
                cases.push((tie, ndigits));
                cases.push((-tie, ndigits));

                let tie_bits = tie.to_bits();
                if tie_bits > 0 {
                    cases.push((f64::from_bits(tie_bits - 1), ndigits));
                }
                if tie_bits < u64::MAX {
                    cases.push((f64::from_bits(tie_bits + 1), ndigits));
                }
            }
        }

        let specials = [
            0.0,
            -0.0,
            2.675,
            2.685,
            1.25,
            1.35,
            -1.25,
            -1.35,
            f64::MIN_POSITIVE,
            -f64::MIN_POSITIVE,
            1.0e-308,
            -1.0e-308,
            1.0e308,
            -1.0e308,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NAN,
        ];

        for value in specials {
            for ndigits in -8..=8 {
                cases.push((value, ndigits));
            }
        }

        cases
    }

    #[test]
    fn ties_to_even_at_zero_digits() {
        assert_eq!(py_round0(2.5), 2.0);
        assert_eq!(py_round0(3.5), 4.0);
        assert_eq!(py_round0(-2.5), -2.0);
        assert_eq!(py_round0(-3.5), -4.0);
    }

    #[test]
    fn decimal_precision_matches_python_behavior() {
        assert_eq!(py_round(2.675, 2), 2.67);
        assert_eq!(py_round(2.685, 2), 2.69);
        assert_eq!(py_round(1.25, 1), 1.2);
        assert_eq!(py_round(1.35, 1), 1.4);
    }

    #[test]
    fn negative_ndigits_matches_python_behavior() {
        assert_eq!(py_round(1250.0, -2), 1200.0);
        assert_eq!(py_round(1350.0, -2), 1400.0);
        assert_eq!(py_round(-1250.0, -2), -1200.0);
        assert_eq!(py_round(-1350.0, -2), -1400.0);
    }

    #[test]
    fn non_finite_values_passthrough() {
        assert!(py_round(f64::NAN, 2).is_nan());
        assert_eq!(py_round(f64::INFINITY, 2), f64::INFINITY);
        assert_eq!(py_round(f64::NEG_INFINITY, -3), f64::NEG_INFINITY);
    }

    #[test]
    fn python_oracle_many_values_deterministic() {
        let cases = deterministic_cases();
        assert_matches_python(&cases);
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(1024))]

        #[test]
        fn python_oracle_proptest_many_values(
            value in any::<f64>().prop_filter("finite only", |v| v.is_finite()),
            ndigits in -10i32..=10i32,
        ) {
            let single = [(value, ndigits)];
            assert_matches_python(&single);
        }
    }
}
