//! 欄位值規則：R2、R5、R6、R9、R17。
//!
//! v2 的 Tag 沒有 `writable`／`write_permission`，所以沒有 R3、R4、R23。

use std::cmp::Ordering;

use crate::diag::Diagnostic;
use crate::schema::{MachineModel, Scalar, Tag, Variable};
use crate::types::{DataType, ScalarType};

pub(super) fn check(m: &MachineModel, out: &mut Vec<Diagnostic>) {
    for t in &m.tags {
        check_tag(t, out);
    }
    for v in &m.variables {
        check_variable(v, out);
    }
}

fn parse_type(ty: &str, loc: &str, out: &mut Vec<Diagnostic>) -> Option<DataType> {
    match DataType::parse(ty) {
        Ok(t) => Some(t),
        Err(e) => {
            out.push(Diagnostic::error("R2", loc, format!("type `{ty}`：{e}")));
            None
        }
    }
}

fn check_tag(t: &Tag, out: &mut Vec<Diagnostic>) {
    let loc = format!("tags[{}]", t.id);

    if t.address.is_empty() {
        out.push(Diagnostic::error("R17", &loc, "tag 必須有 address"));
    }
    let ty = parse_type(&t.ty, &loc, out);

    if let (Some(min), Some(max)) = (t.min, t.max) {
        if min.partial_cmp(&max) != Some(Ordering::Less) {
            out.push(Diagnostic::error(
                "R5",
                &loc,
                format!("min ({min}) 必須小於 max ({max})"),
            ));
        }
    }

    #[allow(clippy::float_cmp)]
    if t.scale == Some(0.0) {
        out.push(Diagnostic::error("R9", &loc, "scale 不得為 0"));
    }

    if let (Some(default), Some(ty)) = (&t.default, ty) {
        check_default(&ty, default, t.min, t.max, &loc, out);
    }
}

fn check_variable(v: &Variable, out: &mut Vec<Diagnostic>) {
    let loc = format!("variables[{}]", v.id);
    let ty = parse_type(&v.ty, &loc, out);
    if let (Some(default), Some(ty)) = (&v.default, ty) {
        check_default(&ty, default, None, None, &loc, out);
    }
}

#[allow(clippy::cast_precision_loss)]
fn numeric_value(d: &Scalar) -> Option<f64> {
    match d {
        Scalar::Int(i) => Some(*i as f64),
        Scalar::Float(f) => Some(*f),
        _ => None,
    }
}

fn check_default(
    ty: &DataType,
    default: &Scalar,
    min: Option<f64>,
    max: Option<f64>,
    loc: &str,
    out: &mut Vec<Diagnostic>,
) {
    if ty.array {
        out.push(Diagnostic::error("R6", loc, "陣列型別不支援 default"));
        return;
    }
    let compatible = match (ty.scalar, default) {
        (ScalarType::Bool, Scalar::Bool(_))
        | (ScalarType::Float32 | ScalarType::Float64, Scalar::Int(_) | Scalar::Float(_)) => true,
        (ScalarType::Text(n), Scalar::Text(s)) => s.len() <= usize::from(n),
        (_, Scalar::Int(i)) => ty
            .int_bounds()
            .is_some_and(|(lo, hi)| (lo..=hi).contains(i)),
        _ => false,
    };
    if !compatible {
        out.push(Diagnostic::error(
            "R6",
            loc,
            format!(
                "default {default:?} 與型別 `{}` 不相容或超出型別範圍",
                describe(ty)
            ),
        ));
        return;
    }
    if let Some(value) = numeric_value(default) {
        let below = min.is_some_and(|lo| value < lo);
        let above = max.is_some_and(|hi| value > hi);
        if below || above {
            out.push(Diagnostic::error(
                "R6",
                loc,
                format!("default {value} 不在 min／max 範圍內"),
            ));
        }
    }
}

fn describe(ty: &DataType) -> String {
    format!("{:?}", ty.scalar)
}
