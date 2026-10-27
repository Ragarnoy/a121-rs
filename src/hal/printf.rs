//! Minimal `printf`-style formatting for SDK log messages.
//!
//! The SDK passes its log messages as a format string plus C variadic arguments. This module
//! renders them without newlib's `vsnprintf`. It supports the conversions `d i u o x X c s e E f F
//! g G p %`, the flags `- + space # 0`, width and precision (including `*`), and the length
//! modifiers `hh h l ll j z t`. Formatting stops at the first unsupported conversion, since its
//! argument cannot be skipped; the rest of the format string is written as is.
//!
//! Floats are converted to `f32`, the only float type the SDK logs, and rendered with at most
//! [`MAX_SIGNIFICANT`] significant digits; further digits are written as zeros. Doubles outside
//! the `f32` range print as `inf` or 0.

use core::ffi::{
    c_char, c_double, c_int, c_long, c_longlong, c_uint, c_ulong, c_ulonglong, VaList,
};
use core::fmt::{self, Write};

/// Source of `printf` arguments, in the C default-promoted types.
pub(crate) trait PrintfArgs {
    /// # Safety
    /// The next argument must have been passed as a `c_int`.
    unsafe fn int(&mut self) -> c_int;
    /// # Safety
    /// The next argument must have been passed as a `c_uint` (or `c_int`).
    unsafe fn uint(&mut self) -> c_uint;
    /// # Safety
    /// The next argument must have been passed as a `c_long`.
    unsafe fn long(&mut self) -> c_long;
    /// # Safety
    /// The next argument must have been passed as a `c_ulong`.
    unsafe fn ulong(&mut self) -> c_ulong;
    /// # Safety
    /// The next argument must have been passed as a `c_longlong`.
    unsafe fn longlong(&mut self) -> c_longlong;
    /// # Safety
    /// The next argument must have been passed as a `c_ulonglong`.
    unsafe fn ulonglong(&mut self) -> c_ulonglong;
    /// # Safety
    /// The next argument must have been passed as a `c_double` (or a promoted `float`).
    unsafe fn double(&mut self) -> c_double;
    /// # Safety
    /// The next argument must have been passed as a pointer.
    unsafe fn ptr(&mut self) -> *const c_char;
}

impl PrintfArgs for VaList<'_> {
    unsafe fn int(&mut self) -> c_int {
        unsafe { self.next_arg() }
    }
    unsafe fn uint(&mut self) -> c_uint {
        unsafe { self.next_arg() }
    }
    unsafe fn long(&mut self) -> c_long {
        unsafe { self.next_arg() }
    }
    unsafe fn ulong(&mut self) -> c_ulong {
        unsafe { self.next_arg() }
    }
    unsafe fn longlong(&mut self) -> c_longlong {
        unsafe { self.next_arg() }
    }
    unsafe fn ulonglong(&mut self) -> c_ulonglong {
        unsafe { self.next_arg() }
    }
    unsafe fn double(&mut self) -> c_double {
        unsafe { self.next_arg() }
    }
    unsafe fn ptr(&mut self) -> *const c_char {
        unsafe { self.next_arg() }
    }
}

/// Marker appended when a [`LineBuf`] runs out of space.
const ELLIPSIS: &str = "…";

/// Fixed-size text buffer. Output that does not fit is cut at a character boundary and marked
/// with `…`, and every write from then on fails, so formatting stops early.
pub(crate) struct LineBuf<const N: usize> {
    buf: [u8; N],
    len: usize,
    truncated: bool,
}

impl<const N: usize> LineBuf<N> {
    pub(crate) const fn new() -> Self {
        Self {
            buf: [0; N],
            len: 0,
            truncated: false,
        }
    }

    pub(crate) fn as_str(&self) -> &str {
        // Only whole `str` prefixes cut at char boundaries are ever copied in
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl<const N: usize> Write for LineBuf<N> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.truncated {
            return Err(fmt::Error);
        }
        if s.len() <= N - self.len {
            self.push(s);
            return Ok(());
        }
        // Make room for the marker, dropping already written text if needed
        let marker = if N >= ELLIPSIS.len() { ELLIPSIS } else { "" };
        let limit = N - marker.len();
        if self.len > limit {
            let mut cut = limit;
            while !self.as_str().is_char_boundary(cut) {
                cut -= 1;
            }
            self.len = cut;
        } else {
            let mut cut = limit - self.len;
            while !s.is_char_boundary(cut) {
                cut -= 1;
            }
            self.push(&s[..cut]);
        }
        self.push(marker);
        self.truncated = true;
        Err(fmt::Error)
    }
}

impl<const N: usize> LineBuf<N> {
    fn push(&mut self, s: &str) {
        self.buf[self.len..self.len + s.len()].copy_from_slice(s.as_bytes());
        self.len += s.len();
    }
}

/// Largest width or precision honoured; larger ones are clamped. Output is cut at 256 bytes
/// anyway, and this bounds the padding loops.
const MAX_FIELD: usize = 1024;

#[derive(Default)]
struct Spec {
    left: bool,
    plus: bool,
    space: bool,
    alt: bool,
    zero: bool,
    width: usize,
    precision: Option<usize>,
}

#[derive(Clone, Copy)]
enum Length {
    Default,
    Char,
    Short,
    Long,
    LongLong,
    Size,
}

/// Renders `fmt` with arguments from `args` into `out`.
///
/// # Safety
///
/// `args` must hold arguments matching the conversions in `fmt`, as for C `printf`, and every
/// `%s` argument must be NULL or a valid string (NUL-terminated, or at least as long as the
/// precision).
pub(crate) unsafe fn format(
    fmt: &[u8],
    args: &mut impl PrintfArgs,
    out: &mut impl Write,
) -> fmt::Result {
    let mut i = 0;
    while i < fmt.len() {
        let literal_start = i;
        while i < fmt.len() && fmt[i] != b'%' {
            i += 1;
        }
        write_lossy(out, &fmt[literal_start..i])?;
        if i == fmt.len() {
            break;
        }

        let spec_start = i;
        i += 1;
        let mut spec = Spec::default();
        while let Some(&flag) = fmt.get(i) {
            match flag {
                b'-' => spec.left = true,
                b'+' => spec.plus = true,
                b' ' => spec.space = true,
                b'#' => spec.alt = true,
                b'0' => spec.zero = true,
                _ => break,
            }
            i += 1;
        }
        if fmt.get(i) == Some(&b'*') {
            let width = unsafe { args.int() };
            spec.left |= width < 0;
            spec.width = (width.unsigned_abs() as usize).min(MAX_FIELD);
            i += 1;
        } else {
            spec.width = parse_number(fmt, &mut i);
        }
        if fmt.get(i) == Some(&b'.') {
            i += 1;
            if fmt.get(i) == Some(&b'*') {
                let precision = unsafe { args.int() };
                // A negative precision is taken as if it were omitted
                spec.precision = usize::try_from(precision).ok().map(|p| p.min(MAX_FIELD));
                i += 1;
            } else {
                spec.precision = Some(parse_number(fmt, &mut i));
            }
        }
        let length = parse_length(fmt, &mut i);

        let Some(&conversion) = fmt.get(i) else {
            // Incomplete specification at the end: print it as is
            write_lossy(out, &fmt[spec_start..])?;
            break;
        };
        i += 1;

        match conversion {
            b'd' | b'i' => {
                let value = unsafe { signed_arg(args, length) };
                let sign = sign_str(value < 0, &spec);
                write_integer(out, &spec, sign, "", value.unsigned_abs(), 10, false)?;
            }
            b'u' | b'o' | b'x' | b'X' => {
                let value = unsafe { unsigned_arg(args, length) };
                let (base, upper) = match conversion {
                    b'o' => (8, false),
                    b'x' => (16, false),
                    b'X' => (16, true),
                    _ => (10, false),
                };
                let prefix = match conversion {
                    b'x' if spec.alt && value != 0 => "0x",
                    b'X' if spec.alt && value != 0 => "0X",
                    _ => "",
                };
                write_integer(out, &spec, "", prefix, value, base, upper)?;
            }
            b'c' => {
                // The argument is promoted to int; C prints it as an unsigned char. A byte that is
                // not ASCII is not valid UTF-8 on its own, so it is replaced, as in `%s`.
                let byte = unsafe { args.int() } as u8;
                let c = if byte.is_ascii() {
                    char::from(byte)
                } else {
                    char::REPLACEMENT_CHARACTER
                };
                let mut encoded = [0; 4];
                write_padded(out, &spec, false, "", "", 0, &*c.encode_utf8(&mut encoded))?;
            }
            b's' => {
                let ptr = unsafe { args.ptr() };
                let bytes = if ptr.is_null() {
                    let null: &[u8] = b"(null)";
                    &null[..spec.precision.unwrap_or(null.len()).min(null.len())]
                } else {
                    unsafe { c_str_bytes(ptr, spec.precision) }
                };
                let padding = spec.width.saturating_sub(bytes.len());
                if !spec.left {
                    write_repeated(out, ' ', padding)?;
                }
                write_lossy(out, bytes)?;
                if spec.left {
                    write_repeated(out, ' ', padding)?;
                }
            }
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' => {
                let value = unsafe { args.double() };
                write_float(out, &spec, value, conversion)?;
            }
            b'p' => {
                let address = unsafe { args.ptr() } as usize as u64;
                write_integer(out, &spec, "", "0x", address, 16, false)?;
            }
            b'%' => out.write_char('%')?,
            b'n' => {
                // Consume the pointer but never write through it
                let _ = unsafe { args.ptr() };
            }
            _ => {
                // The size of its argument is unknown, so later arguments cannot be found
                write_lossy(out, &fmt[spec_start..])?;
                break;
            }
        }
    }
    Ok(())
}

fn parse_number(fmt: &[u8], i: &mut usize) -> usize {
    let mut value: usize = 0;
    while let Some(digit) = fmt.get(*i).filter(|b| b.is_ascii_digit()) {
        value = value
            .saturating_mul(10)
            .saturating_add(usize::from(digit - b'0'));
        *i += 1;
    }
    value.min(MAX_FIELD)
}

fn parse_length(fmt: &[u8], i: &mut usize) -> Length {
    let (length, consumed) = match (fmt.get(*i), fmt.get(*i + 1)) {
        (Some(b'h'), Some(b'h')) => (Length::Char, 2),
        (Some(b'h'), _) => (Length::Short, 1),
        (Some(b'l'), Some(b'l')) => (Length::LongLong, 2),
        (Some(b'l'), _) => (Length::Long, 1),
        (Some(b'j'), _) => (Length::LongLong, 1),
        (Some(b'z' | b't'), _) => (Length::Size, 1),
        // long double is not supported; read it as a double
        (Some(b'L'), _) => (Length::Default, 1),
        _ => (Length::Default, 0),
    };
    *i += consumed;
    length
}

// c_long is 32 bits on the embedded targets but 64 bits on the host, where the conversion is a no-op
#[allow(clippy::useless_conversion)]
unsafe fn signed_arg(args: &mut impl PrintfArgs, length: Length) -> i64 {
    unsafe {
        match length {
            // Promoted to int, then converted back as C does
            Length::Char => i64::from(args.int() as i8),
            Length::Short => i64::from(args.int() as i16),
            Length::Default => i64::from(args.int()),
            // size_t/ptrdiff_t have the width of long on the supported targets
            Length::Long | Length::Size => i64::from(args.long()),
            Length::LongLong => args.longlong(),
        }
    }
}

#[allow(clippy::useless_conversion)]
unsafe fn unsigned_arg(args: &mut impl PrintfArgs, length: Length) -> u64 {
    unsafe {
        match length {
            Length::Char => u64::from(args.uint() as u8),
            Length::Short => u64::from(args.uint() as u16),
            Length::Default => u64::from(args.uint()),
            Length::Long | Length::Size => u64::from(args.ulong()),
            Length::LongLong => args.ulonglong(),
        }
    }
}

/// Reads a C string, stopping at the precision if there is one.
unsafe fn c_str_bytes<'a>(ptr: *const c_char, max: Option<usize>) -> &'a [u8] {
    let mut len = 0;
    while max.is_none_or(|max| len < max) && unsafe { *ptr.add(len) } != 0 {
        len += 1;
    }
    unsafe { core::slice::from_raw_parts(ptr.cast::<u8>(), len) }
}

fn sign_str(negative: bool, spec: &Spec) -> &'static str {
    if negative {
        "-"
    } else if spec.plus {
        "+"
    } else if spec.space {
        " "
    } else {
        ""
    }
}

fn write_integer(
    out: &mut impl Write,
    spec: &Spec,
    sign: &str,
    prefix: &str,
    value: u64,
    base: u64,
    upper: bool,
) -> fmt::Result {
    let mut buf = [0u8; 22];
    // With a precision of 0, the value 0 prints no digits
    let digits = if value == 0 && spec.precision == Some(0) {
        ""
    } else {
        digits(value, base, upper, &mut buf)
    };
    let zeros = spec.precision.unwrap_or(0).saturating_sub(digits.len());
    // `#o` guarantees a leading zero
    let prefix = if base == 8 && spec.alt && zeros == 0 && !digits.starts_with('0') {
        "0"
    } else {
        prefix
    };
    // The 0 flag is ignored when a precision is given
    let zero_pad = spec.zero && spec.precision.is_none();
    write_padded(out, spec, zero_pad, sign, prefix, zeros, digits)
}

/// Writes the digits of `value` in `base` into `buf` and returns them.
fn digits(value: u64, base: u64, upper: bool, buf: &mut [u8; 22]) -> &str {
    // u64::MAX in octal is 22 digits
    let mut start = buf.len();
    let mut rest = value;
    loop {
        let digit = (rest % base) as u8;
        start -= 1;
        buf[start] = match digit {
            0..=9 => b'0' + digit,
            _ if upper => b'A' + digit - 10,
            _ => b'a' + digit - 10,
        };
        rest /= base;
        if rest == 0 {
            break;
        }
    }
    core::str::from_utf8(&buf[start..]).unwrap_or("")
}

/// Writes `value` in decimal with at least `min_digits` digits.
fn write_decimal(out: &mut impl Write, value: u64, min_digits: usize) -> fmt::Result {
    let mut buf = [0u8; 22];
    let digits = digits(value, 10, false, &mut buf);
    write_repeated(out, '0', min_digits.saturating_sub(digits.len()))?;
    out.write_str(digits)
}

// Floats are rendered in single precision: the target FPU has no double-precision unit, so f64
// arithmetic would link software routines, and core's float formatting would add about 24 KiB.

/// Significant digits a float is rendered with; later digits are written as zeros.
const MAX_SIGNIFICANT: usize = 7;

const POW10: [u32; 10] = {
    let mut table = [1u32; 10];
    let mut i = 1;
    while i < table.len() {
        table[i] = table[i - 1] * 10;
        i += 1;
    }
    table
};

const POW10_F32: [f32; 39] = {
    // Computed in f64 at compile time, so each entry is rounded once
    let mut table = [1.0f32; 39];
    let mut power = 1.0f64;
    let mut i = 0;
    while i < table.len() {
        table[i] = power as f32;
        power *= 10.0;
        i += 1;
    }
    table
};

/// A non-negative number rounded to `count` significant digits, the first of which is at the
/// decimal position `exp`: `digits * 10^(exp + 1 - count)`.
#[derive(Clone, Copy)]
struct Decimal {
    digits: u32,
    count: usize,
    exp: i32,
}

impl Decimal {
    const ZERO: Self = Self {
        digits: 0,
        count: 1,
        exp: 0,
    };

    /// Digit at the decimal position `pos`: 0 for units, -1 for tenths and so on.
    fn digit(&self, pos: i32) -> char {
        let index = self.exp - pos;
        if index < 0 || index >= self.count as i32 {
            return '0';
        }
        let digit = self.digits / POW10[self.count - 1 - index as usize] % 10;
        char::from(b'0' + digit as u8)
    }

    /// Position of the last non-zero digit, or 0 if there is none.
    fn last_nonzero(&self) -> i32 {
        if self.digits == 0 {
            return 0;
        }
        let mut digits = self.digits;
        let mut pos = self.exp + 1 - self.count as i32;
        while digits.is_multiple_of(10) {
            digits /= 10;
            pos += 1;
        }
        pos
    }
}

/// Splits a finite non-negative `value` into a mantissa in `[1, 10)` and a decimal exponent.
fn normalize(value: f32) -> (f32, i32) {
    if value == 0.0 {
        return (0.0, 0);
    }
    let (mut value, mut exp) = (value, 0);
    if value < 1e-30 {
        // Bring subnormals into the range of the table
        value *= 1e15;
        exp = -15;
    }
    let mut e = 0;
    if value >= 1.0 {
        while e < POW10_F32.len() - 1 && value >= POW10_F32[e + 1] {
            e += 1;
        }
        (value / POW10_F32[e], exp + e as i32)
    } else {
        while value * POW10_F32[e] < 1.0 {
            e += 1;
        }
        (value * POW10_F32[e], exp - e as i32)
    }
}

/// Rounds a normalized number to `count` significant digits, at most [`MAX_SIGNIFICANT`].
fn round(mantissa: f32, exp: i32, count: usize) -> Decimal {
    let count = count.clamp(1, MAX_SIGNIFICANT);
    if mantissa == 0.0 {
        return Decimal {
            count,
            ..Decimal::ZERO
        };
    }
    // Below 10^7 every integer is exact in an f32
    let mut digits = (mantissa * POW10_F32[count - 1] + 0.5) as u32;
    let mut exp = exp;
    // Rounding can carry into a new digit, as in 9.99 -> 10.0
    if digits >= POW10[count] {
        digits /= 10;
        exp += 1;
    }
    Decimal { digits, count, exp }
}

/// Rounds a normalized number to `precision` decimals.
fn round_fixed(mantissa: f32, exp: i32, precision: usize) -> Decimal {
    let count = exp + 1 + precision as i32;
    if count > 0 {
        round(mantissa, exp, count as usize)
    } else if count == 0 && mantissa >= 5.0 {
        // Only the rounding reaches the last decimal
        Decimal {
            digits: 1,
            count: 1,
            exp: -(precision as i32),
        }
    } else {
        Decimal::ZERO
    }
}

/// Text written after the sign, prefix and leading zeros of a conversion.
trait Body {
    /// Length in characters, for padding.
    fn char_len(&self) -> usize;
    fn write(&self, out: &mut impl Write) -> fmt::Result;
}

impl Body for &str {
    fn char_len(&self) -> usize {
        self.chars().count()
    }
    fn write(&self, out: &mut impl Write) -> fmt::Result {
        out.write_str(self)
    }
}

/// A float in `%f` or `%e` form.
enum Float {
    Fixed {
        decimal: Decimal,
        precision: usize,
        point: bool,
    },
    Exponent {
        decimal: Decimal,
        precision: usize,
        point: bool,
        upper: bool,
    },
}

impl Body for Float {
    fn char_len(&self) -> usize {
        match *self {
            Float::Fixed {
                decimal,
                precision,
                point,
            } => decimal.exp.max(0) as usize + 1 + usize::from(point) + precision,
            Float::Exponent {
                decimal,
                precision,
                point,
                ..
            } => {
                let exp_digits = if decimal.exp.unsigned_abs() >= 100 {
                    3
                } else {
                    2
                };
                1 + usize::from(point) + precision + 2 + exp_digits
            }
        }
    }

    fn write(&self, out: &mut impl Write) -> fmt::Result {
        match *self {
            Float::Fixed {
                decimal,
                precision,
                point,
            } => {
                for pos in (0..=decimal.exp.max(0)).rev() {
                    out.write_char(decimal.digit(pos))?;
                }
                if point {
                    out.write_char('.')?;
                }
                for pos in 1..=precision as i32 {
                    out.write_char(decimal.digit(-pos))?;
                }
                Ok(())
            }
            Float::Exponent {
                decimal,
                precision,
                point,
                upper,
            } => {
                out.write_char(decimal.digit(decimal.exp))?;
                if point {
                    out.write_char('.')?;
                }
                for offset in 1..=precision as i32 {
                    out.write_char(decimal.digit(decimal.exp - offset))?;
                }
                out.write_char(if upper { 'E' } else { 'e' })?;
                out.write_char(if decimal.exp < 0 { '-' } else { '+' })?;
                write_decimal(out, u64::from(decimal.exp.unsigned_abs()), 2)
            }
        }
    }
}

fn write_float(out: &mut impl Write, spec: &Spec, value: f64, conversion: u8) -> fmt::Result {
    let upper = conversion.is_ascii_uppercase();
    // The SDK logs floats, which C promotes to double for variadic calls
    let value = value as f32;
    let sign = sign_str(value.is_sign_negative() && !value.is_nan(), spec);
    if !value.is_finite() {
        let text = match (value.is_nan(), upper) {
            (true, false) => "nan",
            (true, true) => "NAN",
            (false, false) => "inf",
            (false, true) => "INF",
        };
        return write_padded(out, spec, false, sign, "", 0, text);
    }

    let precision = spec.precision.unwrap_or(6);
    let (mantissa, exp) = normalize(value.abs());
    let body = match conversion.to_ascii_lowercase() {
        b'f' => Float::Fixed {
            decimal: round_fixed(mantissa, exp, precision),
            precision,
            point: precision > 0 || spec.alt,
        },
        b'e' => Float::Exponent {
            decimal: round(mantissa, exp, precision + 1),
            precision,
            point: precision > 0 || spec.alt,
            upper,
        },
        _ => general(mantissa, exp, precision, spec.alt, upper),
    };
    write_padded(out, spec, spec.zero, sign, "", 0, body)
}

/// `%g`: `precision` significant digits, in `%e` form for small or large exponents and in `%f`
/// form otherwise. Trailing zeros are removed unless `alt` (`#`) is set.
fn general(mantissa: f32, exp: i32, precision: usize, alt: bool, upper: bool) -> Float {
    let significant = precision.max(1);
    let decimal = round(mantissa, exp, significant);
    let last = decimal.last_nonzero();
    let trim = |precision: usize, wanted: i32| {
        if alt {
            precision
        } else {
            precision.min(wanted.max(0) as usize)
        }
    };
    if decimal.exp < -4 || decimal.exp >= significant as i32 {
        let precision = trim(significant - 1, decimal.exp - last);
        Float::Exponent {
            decimal,
            precision,
            point: precision > 0 || alt,
            upper,
        }
    } else {
        let precision = trim((significant as i32 - 1 - decimal.exp) as usize, -last);
        Float::Fixed {
            decimal,
            precision,
            point: precision > 0 || alt,
        }
    }
}

/// Writes `sign`, `prefix`, `zeros` zeros and `body`, padded to the field width.
fn write_padded(
    out: &mut impl Write,
    spec: &Spec,
    zero_pad: bool,
    sign: &str,
    prefix: &str,
    zeros: usize,
    body: impl Body,
) -> fmt::Result {
    let len = sign.len() + prefix.len() + zeros + body.char_len();
    let padding = spec.width.saturating_sub(len);
    if spec.left {
        out.write_str(sign)?;
        out.write_str(prefix)?;
        write_repeated(out, '0', zeros)?;
        body.write(out)?;
        write_repeated(out, ' ', padding)
    } else if zero_pad {
        out.write_str(sign)?;
        out.write_str(prefix)?;
        write_repeated(out, '0', zeros + padding)?;
        body.write(out)
    } else {
        write_repeated(out, ' ', padding)?;
        out.write_str(sign)?;
        out.write_str(prefix)?;
        write_repeated(out, '0', zeros)?;
        body.write(out)
    }
}

fn write_repeated(out: &mut impl Write, c: char, count: usize) -> fmt::Result {
    for _ in 0..count {
        out.write_char(c)?;
    }
    Ok(())
}

/// Writes bytes as UTF-8, replacing invalid sequences with U+FFFD.
fn write_lossy(out: &mut impl Write, bytes: &[u8]) -> fmt::Result {
    for chunk in bytes.utf8_chunks() {
        out.write_str(chunk.valid())?;
        if !chunk.invalid().is_empty() {
            out.write_char(char::REPLACEMENT_CHARACTER)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::ffi::CStr;

    /// Formats through a real C variadic call, as the SDK does.
    /// Returns whether formatting succeeded, that is, whether the output was not truncated.
    unsafe extern "C" fn render(out: *mut LineBuf<256>, fmt: *const c_char, mut args: ...) -> bool {
        let fmt = unsafe { CStr::from_ptr(fmt) }.to_bytes();
        unsafe { format(fmt, &mut args, &mut *out) }.is_ok()
    }

    macro_rules! check {
        ($expected:expr, $fmt:literal $(, $arg:expr)* $(,)?) => {{
            let mut out = LineBuf::<256>::new();
            assert!(unsafe { render(&mut out, $fmt.as_ptr() $(, $arg)*) });
            assert_eq!(out.as_str(), $expected, "format {:?}", $fmt);
        }};
    }

    #[test]
    fn literals_and_percent() {
        check!("plain text", c"plain text");
        check!("100%", c"100%%");
        check!("%q", c"%q");
        check!("end %", c"end %");
    }

    #[test]
    fn unsupported_conversion_stops_formatting() {
        check!(
            "1 %a %d %s",
            c"%d %a %d %s",
            1 as c_int,
            2.0f64,
            3 as c_int,
            4 as c_int
        );
        check!("%*q %d", c"%*q %d", 5 as c_int, 6 as c_int);
    }

    #[test]
    fn huge_widths_are_clamped() {
        let check_truncated = |out: &LineBuf<256>| {
            assert_eq!(out.as_str().len(), 256);
            assert!(out.as_str().ends_with(ELLIPSIS));
        };
        let mut out = LineBuf::<256>::new();
        assert!(!unsafe { render(&mut out, c"%*d".as_ptr(), c_int::MIN, 1 as c_int) });
        check_truncated(&out);
        let mut out = LineBuf::<256>::new();
        assert!(!unsafe { render(&mut out, c"%4000000000d".as_ptr(), 1 as c_int) });
        check_truncated(&out);
        let mut out = LineBuf::<256>::new();
        assert!(!unsafe { render(&mut out, c"%.*f".as_ptr(), c_int::MAX, 1.0f64) });
        check_truncated(&out);
    }

    #[test]
    fn sdk_integer_conversions() {
        check!("000042 x", c"%06lu %s", 42 as c_ulong, c"x".as_ptr());
        check!(
            "beef 0005",
            c"%04hx %04hx",
            0xBEEF as c_uint,
            0x1_0005 as c_uint
        );
        check!("4464 -5", c"%hi %li", 70_000 as c_int, -5 as c_long);
        check!(
            "0000dead 00ff",
            c"%08lx %04lx",
            0xDEAD as c_ulong,
            0xFF as c_ulong
        );
        check!("7 17", c"%u %lo", 7 as c_uint, 15 as c_ulong);
        check!("ok", c"%c%c", b'o' as c_int, b'k' as c_int);
        check!("\u{FFFD}|  a", c"%c|%3c", 0xB0 as c_int, b'a' as c_int);
    }

    #[test]
    fn integer_flags_width_and_precision() {
        check!(
            "1|    2|3    |00004|+5| 6",
            c"%d|%5d|%-5d|%05d|%+d|% d",
            1 as c_int,
            2 as c_int,
            3 as c_int,
            4 as c_int,
            5 as c_int,
            6 as c_int,
        );
        check!(
            "ff FF 0xff 0XFF 10 010 0",
            c"%x %X %#x %#X %o %#o %#x",
            255 as c_uint,
            255 as c_uint,
            255 as c_uint,
            255 as c_uint,
            8 as c_uint,
            8 as c_uint,
            0 as c_uint,
        );
        check!(
            "0 0 00010",
            c"%#o %#.0o %#.5o",
            0 as c_uint,
            0 as c_uint,
            8 as c_uint
        );
        check!("|007|  -07|", c"|%.3d|%5.2d|", 7 as c_int, -7 as c_int);
        check!("[]", c"[%.0d]", 0 as c_int);
        check!(
            "   7|7   |",
            c"%*d|%-*d|",
            4 as c_int,
            7 as c_int,
            4 as c_int,
            7 as c_int
        );
        check!("-9223372036854775808", c"%lld", c_longlong::MIN);
        check!("18446744073709551615", c"%llu", c_ulonglong::MAX);
        check!("42", c"%zu", 42usize);
    }

    #[test]
    fn strings() {
        check!("(null)", c"%s", core::ptr::null::<c_char>());
        check!(
            "ab|   ab|ab   |",
            c"%.2s|%5s|%-5s|",
            c"abc".as_ptr(),
            c"ab".as_ptr(),
            c"ab".as_ptr()
        );
        check!("x", c"%.*s", 1 as c_int, c"xyz".as_ptr());
        check!("caf\u{FFFD}", c"%s", c"caf\xff".as_ptr());
    }

    #[test]
    fn sdk_float_conversions() {
        check!("1.250000e+01", c"%e", 12.5f64);
        check!("0.000000e+00", c"%e", 0.0f64);
        check!("1.000000E-05", c"%E", 1e-5f64);
        check!(
            "1.500000 +1.500000  1.500000",
            c"%F %+F % F",
            1.5f64,
            1.5f64,
            1.5f64
        );
        check!(
            "1.23 -2.0 0001.2",
            c"%.2f %.1f %06.1f",
            1.23456f64,
            -2.0f64,
            1.23f64
        );
        check!(
            "nan INF  -inf",
            c"%f %F %5f",
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY
        );
    }

    #[test]
    fn float_rounding_and_range() {
        check!(
            "10.00 1.00e+01 0.1",
            c"%.2f %.2e %.1f",
            9.9999f64,
            9.9999f64,
            0.05f64
        );
        check!(
            "1.5e+38 2.5e-38 1.0e-40",
            c"%.1e %.1e %.1e",
            1.5e38f64,
            2.5e-38f64,
            1e-40f64
        );
        // Outside the f32 range
        check!("inf 0.0", c"%.0e %.1f", 1e300f64, 1e-300f64);
        check!("-0.000000", c"%f", -0.0f64);
        check!("0.01 0.00 1", c"%.2f %.2f %.0f", 0.006f64, 0.004f64, 0.6f64);
    }

    #[test]
    fn float_digits_beyond_f32_precision_are_zeros() {
        check!("100000000000000000000.000000", c"%f", 1e20f64);
        check!("2000.0000000000000000", c"%.16f", 2000.0f64);
        check!("0.10000000000000000000", c"%.20f", 0.1f64);
        check!("1.2345670000e+04", c"%.10e", 12345.67f64);
    }

    #[test]
    fn general_float() {
        check!(
            "100 0.0001 1.23457e+08",
            c"%g %g %g",
            100.0f64,
            0.0001f64,
            123_456_789.0f64
        );
        check!("1e-05 1.5 100.000", c"%g %G %#g", 1e-5f64, 1.5f64, 100.0f64);
        check!(
            "0 1E+10 0.000123 1.0",
            c"%g %G %.3g %#.2g",
            0.0f64,
            1e10f64,
            0.000123f64,
            1.0f64
        );
        check!("0.0001234", c"%.16g", 0.0001234f64);
    }

    #[test]
    fn line_buffer_truncates_at_char_boundary() {
        let mut buf = LineBuf::<8>::new();
        assert!(buf.write_str("abcdefghij").is_err());
        assert_eq!(buf.as_str(), "abcde…");
        assert!(buf.write_str("more").is_err());
        assert_eq!(buf.as_str(), "abcde…");

        let mut buf = LineBuf::<8>::new();
        assert!(buf.write_str("abcdé!!!").is_err());
        assert_eq!(buf.as_str(), "abcd…");

        // A message that fits exactly is kept whole
        let mut buf = LineBuf::<8>::new();
        buf.write_str("abcdefgh").unwrap();
        assert_eq!(buf.as_str(), "abcdefgh");

        // Already written text is dropped to make room for the marker
        let mut buf = LineBuf::<8>::new();
        buf.write_str("abcdefg").unwrap();
        assert!(buf.write_str("hi").is_err());
        assert_eq!(buf.as_str(), "abcde…");
    }
}
