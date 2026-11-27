//! Number formatting for drawings, schedules and the interface.
//!
//! Quantities follow the project units ([`cc_core::Units`]), set by the
//! application with [`set_units`]; with the default number style the
//! decimal symbol follows the interface language ([`set_decimal_comma`]).
//! Unit symbols use the active notation ([`set_russian_unit_symbols`]).
//! The `*_with` functions take the project units and decimal symbol
//! explicitly (used by tests and previews).

use cc_core::Units;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};

static DECIMAL_COMMA: AtomicBool = AtomicBool::new(true);
static RUSSIAN_UNIT_SYMBOLS: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// The units quantities are shown in, per thread: the app draws on one
    /// thread, and sessions on other threads (tests, scripts) each keep
    /// their own project's units instead of overwriting one another's.
    static UNITS: RefCell<Units> = RefCell::new(Units::default());
}

/// Use `,` (true) or `.` (false) as the decimal separator of the language.
pub fn set_decimal_comma(comma: bool) {
    DECIMAL_COMMA.store(comma, Ordering::Relaxed);
}

/// Use Russian unit symbols in human-readable quantities.
pub fn set_russian_unit_symbols(enabled: bool) {
    RUSSIAN_UNIT_SYMBOLS.store(enabled, Ordering::Relaxed);
}

pub(crate) fn russian_unit_symbols() -> bool {
    RUSSIAN_UNIT_SYMBOLS.load(Ordering::Relaxed)
}

/// The symbol of a displayed unit in the active language.
pub fn unit_symbol(symbol: &str) -> &str {
    if russian_unit_symbols() { russian_symbol(symbol) } else { symbol }
}

fn russian_symbol(symbol: &str) -> &str {
    match symbol {
        "mm" => "мм",
        "cm" => "см",
        "m" => "м",
        "ft" => "фут",
        "in" => "дюйм",
        "mm²" => "мм²",
        "cm²" => "см²",
        "m²" => "м²",
        "ft²" => "фут²",
        "in²" => "дюйм²",
        "cm³" => "см³",
        "m³" => "м³",
        "ft³" => "фут³",
        "in³" => "дюйм³",
        "yd³" => "ярд³",
        "cm⁴" => "см⁴",
        "in⁴" => "дюйм⁴",
        "L" => "л",
        "gal" => "галлон",
        "kg" => "кг",
        "lb" => "фунт",
        "kg/m" => "кг/м",
        "lb/ft" => "фунт/фут",
        "kg/m²" => "кг/м²",
        "lb/ft²" => "фунт/фут²",
        "kg/m³" => "кг/м³",
        "lb/ft³" => "фунт/фут³",
        "MPa" => "МПа",
        "ksi" => "кип/дюйм²",
        "l/s" => "л/с",
        "gpm" => "галлон/мин",
        "A" => "А",
        "VA" => "ВА",
        _ => symbol,
    }
}

/// Display quantities with `units` from now on.
pub fn set_units(units: &Units) {
    UNITS.with_borrow_mut(|current| {
        if *current != *units {
            *current = units.clone();
        }
    });
}

/// The current units and language decimal symbol.
pub fn current() -> (Units, bool) {
    let units = UNITS.with_borrow(Units::clone);
    (units, DECIMAL_COMMA.load(Ordering::Relaxed))
}

/// `value` with `decimals` places, the style's decimal symbol and grouping.
pub fn number_with(units: &Units, comma: bool, value: f64, decimals: usize) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let s = format!("{:.*}", decimals.min(12), value);
    let (decimal, group) = units.style.separators(comma);
    let (sign, digits) = match s.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", s.as_str()),
    };
    let (int, frac) = digits.split_once('.').unwrap_or((digits, ""));
    let mut grouped = String::with_capacity(int.len() + int.len() / 3);
    for (i, c) in int.chars().enumerate() {
        if i > 0
            && (int.len() - i) % 3 == 0
            && let Some(g) = group
        {
            grouped.push(g);
        }
        grouped.push(c);
    }
    // "-0,00" reads oddly: drop the sign when everything rounds to zero.
    let zero = int.chars().all(|c| c == '0') && frac.chars().all(|c| c == '0');
    let sign = if zero { "" } else { sign };
    if frac.is_empty() { format!("{sign}{grouped}") } else { format!("{sign}{grouped}{decimal}{frac}") }
}

fn with_symbol(text: String, symbol: &str, show: bool) -> String {
    if show { format!("{text} {}", unit_symbol(symbol)) } else { text }
}

/// Inches rounded to 1/`den` (a power of two), as whole inches and the
/// reduced fraction left (numerator, denominator; 0 when none).
fn fraction(inches: f64, den: u32) -> (f64, u32, u32) {
    let den = den.max(1);
    let ticks = (inches * f64::from(den)).round();
    let whole = (ticks / f64::from(den)).floor();
    let (mut num, mut den) = ((ticks - whole * f64::from(den)) as u32, den);
    while num > 0 && num % 2 == 0 && den % 2 == 0 {
        (num, den) = (num / 2, den / 2);
    }
    (whole, num, den)
}

/// The denominator of the fractional inches of a precision setting.
fn denominator(decimals: u8) -> u32 {
    1 << decimals.min(cc_core::units::MAX_DECIMALS)
}

/// Inches with a fraction: `6`, `6 1/2`, `1/2` (`0 1/2` with `zero`; no
/// inch mark).
fn mixed_inches(units: &Units, comma: bool, inches: f64, den: u32, zero: bool) -> String {
    let (whole, num, den) = fraction(inches, den);
    let whole_text = number_with(units, comma, whole, 0);
    match (num, whole == 0.0 && !zero) {
        (0, _) => whole_text,
        (_, true) => format!("{num}/{den}"),
        _ => format!("{whole_text} {num}/{den}"),
    }
}

/// A length in feet and fractional inches (`12'-6 1/2"`) or fractional
/// inches (`150 1/2"`), to 1/2^decimals of an inch.
fn fractional_with(units: &Units, comma: bool, meters: f64) -> String {
    let f = units.length;
    let meters = if meters.is_finite() { meters } else { 0.0 };
    let den = denominator(f.decimals);
    let inches = meters.abs() / cc_core::units::INCH;
    // The sign goes only on lengths that do not round to zero.
    let (whole, num, _) = fraction(inches, den);
    let sign = if meters < 0.0 && (whole > 0.0 || num > 0) { "-" } else { "" };
    if f.unit == cc_core::LengthUnit::FeetInches {
        // Round first, so 11 31/32" at 1/16" becomes 1'-0".
        let rounded = (inches * f64::from(den)).round() / f64::from(den);
        let feet = (rounded / 12.0 + 1e-9).floor();
        let rest = (rounded - feet * 12.0).max(0.0);
        format!("{sign}{}'-{}\"", number_with(units, comma, feet, 0), mixed_inches(units, comma, rest, den, true))
    } else {
        format!("{sign}{}\"", mixed_inches(units, comma, inches, den, false))
    }
}

/// A length's number in the length unit, without the unit symbol; feet
/// and inches and fractional inches carry their own marks.
pub fn length_number_with(units: &Units, comma: bool, meters: f64) -> String {
    let f = units.length;
    if f.unit.is_fractional() {
        fractional_with(units, comma, meters)
    } else {
        number_with(units, comma, meters * f.unit.per_meter(), f.decimals as usize)
    }
}

/// Length in meters, in the length unit (symbol when the settings ask for it).
pub fn length_with(units: &Units, comma: bool, meters: f64) -> String {
    let f = units.length;
    let text = length_number_with(units, comma, meters);
    with_symbol(text, f.unit.symbol(), f.symbol && !f.unit.is_fractional())
}

/// Length always with its unit symbol (property fields, labels).
pub fn length_unit_with(units: &Units, comma: bool, meters: f64) -> String {
    let f = units.length;
    let text = length_number_with(units, comma, meters);
    with_symbol(text, f.unit.symbol(), !f.unit.is_fractional())
}

/// A small size (a section's side, a diameter, a layer) in inches, without
/// feet, with the inch mark: a fraction at the length precision (`1 1/4"`,
/// `5/16"`, `16"`; sixteenths when lengths are not in fractional inches),
/// or the length's decimals when lengths are in decimal feet or inches
/// (`1.25"`).
pub fn inch_size_with(units: &Units, comma: bool, meters: f64) -> String {
    let meters = if meters.is_finite() { meters } else { 0.0 };
    let inches = meters.abs() / cc_core::units::INCH;
    let f = units.length;
    if f.unit.is_imperial() && !f.unit.is_fractional() {
        return format!("{}\"", number_with(units, comma, meters / cc_core::units::INCH, f.decimals as usize));
    }
    let den = if f.unit.is_fractional() { denominator(f.decimals) } else { 16 };
    // The sign goes only on sizes that do not round to zero.
    let (whole, num, _) = fraction(inches, den);
    let sign = if meters < 0.0 && (whole > 0.0 || num > 0) { "-" } else { "" };
    format!("{sign}{}\"", mixed_inches(units, comma, inches, den, false))
}

/// How [`size_with`] writes a small size in metric projects: each place
/// keeps the form it always had there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricSize {
    /// A length with its unit symbol (see [`length_unit_with`]): `0,20 m`.
    Length,
    /// Millimeters with this many decimals and the unit: `12,5 mm`.
    Mm(usize),
    /// Millimeters with this many decimals, no unit (tags): `200`.
    MmNumber(usize),
    /// Centimeters with this many decimals and the unit: `18,0 cm`.
    Cm(usize),
    /// Centimeters with this many decimals, no unit: `18,0`.
    CmNumber(usize),
    /// Centimeters, whole or with one decimal, no unit (section tags): `15`.
    CmTag,
}

/// A small size (a section's side, a diameter, a layer, a riser): as
/// `metric` says in metric projects, in inches with the inch mark in
/// imperial ones (see [`inch_size_with`]).
pub fn size_with(units: &Units, comma: bool, meters: f64, metric: MetricSize) -> String {
    if units.system() == cc_core::MeasurementSystem::Imperial {
        return inch_size_with(units, comma, meters);
    }
    let meters = if meters.is_finite() { meters } else { 0.0 };
    match metric {
        MetricSize::Length => length_unit_with(units, comma, meters),
        MetricSize::Mm(decimals) => with_symbol(number_with(units, comma, meters * 1000.0, decimals), "mm", true),
        MetricSize::MmNumber(decimals) => number_with(units, comma, meters * 1000.0, decimals),
        MetricSize::Cm(decimals) => with_symbol(number_with(units, comma, meters * 100.0, decimals), "cm", true),
        MetricSize::CmNumber(decimals) => number_with(units, comma, meters * 100.0, decimals),
        MetricSize::CmTag => {
            let v = meters * 100.0;
            if (v - v.round()).abs() < 1e-6 { format!("{v:.0}") } else { format!("{v:.1}") }
        }
    }
}

/// Kilograms in a pound (exact, by definition).
pub const POUND: f64 = 0.453_592_37;
/// Megapascals in a kip per square inch.
const KSI: f64 = 6.894_757_293_168;

/// A physical quantity stored in SI units: shown in them in metric
/// projects and in US customary units in imperial ones. Engine commands
/// and files keep the SI value; only the text changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Quantity {
    /// Kilograms; pounds.
    Mass,
    /// Kilograms per meter; pounds per foot.
    LinearMass,
    /// Kilograms per square meter; pounds per square foot.
    AreaMass,
    /// Kilograms per cubic meter; pounds per cubic foot.
    Density,
    /// Megapascals; kips per square inch.
    Stress,
    /// Liters per second; US gallons per minute.
    Flow,
}

impl Quantity {
    /// Displayed units per SI unit in `system`.
    pub fn per_si(self, system: cc_core::MeasurementSystem) -> f64 {
        use cc_core::units::FOOT;
        if system == cc_core::MeasurementSystem::Metric {
            return 1.0;
        }
        match self {
            Quantity::Mass => 1.0 / POUND,
            Quantity::LinearMass => FOOT / POUND,
            Quantity::AreaMass => FOOT * FOOT / POUND,
            Quantity::Density => FOOT * FOOT * FOOT / POUND,
            Quantity::Stress => 1.0 / KSI,
            // A US gallon is 231 cubic inches.
            Quantity::Flow => 60.0 / (231.0 * cc_core::units::INCH.powi(3) * 1000.0),
        }
    }

    /// The unit's symbol in `system`.
    pub fn symbol(self, system: cc_core::MeasurementSystem) -> &'static str {
        let metric = system == cc_core::MeasurementSystem::Metric;
        match (self, metric) {
            (Quantity::Mass, true) => "kg",
            (Quantity::Mass, false) => "lb",
            (Quantity::LinearMass, true) => "kg/m",
            (Quantity::LinearMass, false) => "lb/ft",
            (Quantity::AreaMass, true) => "kg/m²",
            (Quantity::AreaMass, false) => "lb/ft²",
            (Quantity::Density, true) => "kg/m³",
            (Quantity::Density, false) => "lb/ft³",
            (Quantity::Stress, true) => "MPa",
            (Quantity::Stress, false) => "ksi",
            (Quantity::Flow, true) => "l/s",
            (Quantity::Flow, false) => "gpm",
        }
    }
}

/// `si` (a [`Quantity`] in SI units) in the units' system, with `decimals`
/// places and its symbol: `12,50 kg`, `27.56 lb`.
pub fn quantity_with(units: &Units, comma: bool, q: Quantity, si: f64, decimals: usize) -> String {
    let system = units.system();
    let si = if si.is_finite() { si } else { 0.0 };
    with_symbol(number_with(units, comma, si * q.per_si(system), decimals), q.symbol(system), true)
}

pub fn area_with(units: &Units, comma: bool, square_meters: f64) -> String {
    let f = units.area;
    let v = square_meters * f.unit.per_square_meter();
    with_symbol(number_with(units, comma, v, f.decimals as usize), f.unit.symbol(), f.symbol)
}

pub fn volume_with(units: &Units, comma: bool, cubic_meters: f64) -> String {
    let f = units.volume;
    let v = cubic_meters * f.unit.per_cubic_meter();
    with_symbol(number_with(units, comma, v, f.decimals as usize), f.unit.symbol(), f.symbol)
}

pub fn angle_with(units: &Units, comma: bool, degrees: f64) -> String {
    let text = number_with(units, comma, degrees, units.angle.decimals as usize);
    if units.angle.symbol { format!("{text}°") } else { text }
}

/// Slope given as rise over run: `8,3 %`, `4,76°`, or the rise per 12 of
/// run (`6:12`, `4.5:12`; trailing zeros dropped).
pub fn slope_with(units: &Units, comma: bool, slope: f64) -> String {
    let f = units.slope;
    let decimals = f.decimals as usize;
    let (text, symbol) = match f.unit {
        cc_core::SlopeUnit::Percent => (number_with(units, comma, slope * 100.0, decimals), " %"),
        cc_core::SlopeUnit::Degrees => (number_with(units, comma, slope.atan().to_degrees(), decimals), "°"),
        cc_core::SlopeUnit::RisePer12 => {
            let slope = if slope.is_finite() { slope } else { 0.0 };
            (trimmed_with(units, comma, slope * 12.0, decimals), ":12")
        }
    };
    if f.symbol { format!("{text}{symbol}") } else { text }
}

/// Parse a slope as rise over run: a number in the slope unit (percent,
/// degrees, or the rise per 12 of run), or with its unit typed: `8.3 %`,
/// `5°`, a ratio `6:12` or `6/12` (`1 1/2:12` too). `None` for anything
/// else, and for 90° or more.
pub fn parse_slope_with(units: &Units, text: &str) -> Option<f64> {
    let t = text.trim();
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, t),
    };
    let degrees = |d: f64| (d.abs() < 90.0).then(|| d.to_radians().tan());
    let slope = if let Some(n) = t.strip_suffix('%') {
        parse_number(n)? / 100.0
    } else if let Some(n) = t.strip_suffix('°').or_else(|| t.strip_suffix("deg")) {
        degrees(parse_number(n)?)?
    } else if let Some((rise, run)) = t.split_once(':') {
        let (rise, run) = (parse_mixed(rise)?, parse_mixed(run)?);
        if run == 0.0 {
            return None;
        }
        rise / run
    } else if t.contains('/') && !t.contains(char::is_whitespace) {
        parse_mixed(t)?
    } else {
        let v = parse_mixed(t)?;
        match units.slope.unit {
            cc_core::SlopeUnit::Percent => v / 100.0,
            cc_core::SlopeUnit::Degrees => degrees(v)?,
            cc_core::SlopeUnit::RisePer12 => v / 12.0,
        }
    };
    // One sign, before the number.
    if slope < 0.0 {
        return None;
    }
    let slope = if negative { -slope } else { slope };
    // Adding zero turns a negative zero into zero.
    slope.is_finite().then_some(slope + 0.0)
}

pub fn money_with(units: &Units, comma: bool, amount: f64) -> String {
    let text = number_with(units, comma, amount, units.currency_decimals as usize);
    if units.currency.is_empty() { text } else { format!("{} {text}", units.currency) }
}

/// The scale 1:`n` as a project with `units` writes it (`1/4" = 1'-0"` in
/// imperial projects, else `1:n`; see `cc_core::scale::label`).
pub fn scale_with(units: &Units, n: u32) -> String {
    cc_core::scale::label(units.system(), n)
}

/// Millimeters in an inch, for sizes on paper.
const PAPER_INCH: f64 = 25.4;
/// How far from a 1/64" step a paper size may be and still be written as
/// that fraction, millimeters.
const PAPER_FRACTION_TOLERANCE: f64 = 0.005;

/// `value` with at most `decimals` places, trailing zeros dropped.
fn trimmed_with(units: &Units, comma: bool, value: f64, decimals: usize) -> String {
    let text = number_with(units, comma, value, decimals);
    let (decimal, _) = units.style.separators(comma);
    if !text.contains(decimal) {
        return text;
    }
    let text = text.trim_end_matches('0');
    text.strip_suffix(decimal).unwrap_or(text).to_string()
}

/// A size on paper (text heights, bubbles, gaps, sheet sides), stored in
/// millimeters, with its unit: `2,5 mm` in metric projects; in imperial
/// ones inches as a fraction to 1/64 (`3/32"`, `1 1/2"`, `17"`), or with
/// three decimals when the size is not on a 1/64 step (`0.098"`).
pub fn paper_with(units: &Units, comma: bool, mm: f64) -> String {
    let mm = if mm.is_finite() { mm } else { 0.0 };
    if units.system() == cc_core::MeasurementSystem::Metric {
        return with_symbol(trimmed_with(units, comma, mm, 2), "mm", true);
    }
    let inches = mm.abs() / PAPER_INCH;
    let steps = (inches * 64.0).round();
    if (steps / 64.0 - inches).abs() * PAPER_INCH <= PAPER_FRACTION_TOLERANCE {
        let (whole, num, _) = fraction(inches, 64);
        let sign = if mm < 0.0 && (whole > 0.0 || num > 0) { "-" } else { "" };
        format!("{sign}{}\"", mixed_inches(units, comma, inches, 64, false))
    } else {
        format!("{}\"", number_with(units, comma, mm / PAPER_INCH, 3))
    }
}

/// A pen weight on paper, stored in millimeters: `0,25 mm` in metric
/// projects, decimal inches in imperial ones (`0.010"`).
pub fn paper_weight_with(units: &Units, comma: bool, mm: f64) -> String {
    let mm = if mm.is_finite() { mm } else { 0.0 };
    match units.system() {
        cc_core::MeasurementSystem::Metric => with_symbol(number_with(units, comma, mm, 2), "mm", true),
        cc_core::MeasurementSystem::Imperial => format!("{}\"", number_with(units, comma, mm / PAPER_INCH, 3)),
    }
}

/// Parse a size on paper, in millimeters: a bare number is millimeters in
/// metric projects and inches in imperial ones. A suffix (`mm`, `cm`, `in`,
/// their Russian spellings, or `"`) overrides the unit. Fractions and mixed
/// numbers are read (`3/32`, `1 1/2"`); `None` for anything else.
pub fn parse_paper_with(units: &Units, text: &str) -> Option<f64> {
    let t = text.trim();
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, t),
    };
    let lower = t.to_lowercase();
    let suffixes = [
        ("mm", 1.0),
        ("cm", 10.0),
        ("inches", PAPER_INCH),
        ("inch", PAPER_INCH),
        ("in", PAPER_INCH),
        (russian_symbol("mm"), 1.0),
        (russian_symbol("cm"), 10.0),
        (russian_symbol("in"), PAPER_INCH),
        ("\"", PAPER_INCH),
        ("''", PAPER_INCH),
    ];
    let bare = match units.system() {
        cc_core::MeasurementSystem::Metric => 1.0,
        cc_core::MeasurementSystem::Imperial => PAPER_INCH,
    };
    let (number, per) = suffixes
        .iter()
        .find_map(|(suffix, per)| lower.strip_suffix(suffix).map(|n| (n.trim_end(), *per)))
        .unwrap_or((lower.as_str(), bare));
    // One sign, before the number.
    if number.contains('-') && !number.contains('/') {
        return None;
    }
    let mm = parse_mixed(number)? * per;
    let mm = if negative { -mm } else { mm };
    // Adding zero turns a negative zero into zero.
    mm.is_finite().then_some(mm + 0.0)
}

/// Parse a number typed with either decimal symbol and optional grouping.
pub fn parse_number(text: &str) -> Option<f64> {
    let t: String = text.trim().chars().filter(|c| !c.is_whitespace() && *c != '\u{202f}').collect();
    let t = match (t.rfind(','), t.rfind('.')) {
        // Both present: the last one is the decimal symbol.
        (Some(c), Some(p)) if c > p => t.replace('.', "").replace(',', "."),
        (Some(_), Some(_)) => t.replace(',', ""),
        (Some(_), None) => t.replace(',', "."),
        _ => t,
    };
    t.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A number that may be mixed with a fraction: `6`, `6,5`, `1/2`, `6 1/2`
/// or `6-1/2`.
fn parse_mixed(text: &str) -> Option<f64> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    // Spaces separate a whole number from its fraction, never digit groups.
    if !t.contains('/') {
        return if t.contains(char::is_whitespace) { None } else { parse_number(t) };
    }
    let t = t.replace('-', " ");
    let mut parts = t.split_whitespace();
    let (first, second) = (parts.next()?, parts.next());
    if parts.next().is_some() {
        return None;
    }
    let ratio = |s: &str| -> Option<f64> {
        let (n, d) = s.split_once('/')?;
        let (n, d) = (parse_number(n)?, parse_number(d)?);
        (d != 0.0).then(|| n / d).filter(|v| v.is_finite())
    };
    match second {
        Some(frac) => Some(parse_number(first)? + ratio(frac)?),
        None => ratio(first),
    }
}

/// Feet and inches: `12'6"`, `12'-6 1/2"`, `12' 6.5"`, `12'`, `6 1/2"`,
/// `12ft 6in`, and, read in feet and inches, `12 6` or `12 6 1/2`. The
/// outer `None` when the text is not written in feet or inches; the inner
/// one when it is, but cannot be read.
fn parse_imperial(units: &Units, text: &str) -> Option<Option<f64>> {
    let t = text.trim();
    let (negative, t) = match t.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, t),
    };
    let t = t.replace("feet", "'").replace("foot", "'").replace("ft", "'").replace(russian_symbol("ft"), "'");
    let t = t
        .replace("inches", "\"")
        .replace("inch", "\"")
        .replace("in", "\"")
        .replace(russian_symbol("in"), "\"")
        .replace("''", "\"");
    let unit = units.length.unit;
    let inches = if let Some((feet, rest)) = t.split_once('\'') {
        let rest = rest.trim().trim_start_matches('-').trim();
        let rest = rest.strip_suffix('"').unwrap_or(rest);
        let inches = if rest.trim().is_empty() { Some(0.0) } else { parse_mixed(rest) };
        parse_mixed(feet).zip(inches).map(|(f, i)| f * 12.0 + i)
    } else if let Some(inches) = t.trim().strip_suffix('"') {
        parse_mixed(inches)
    } else if unit == cc_core::LengthUnit::FeetInches {
        // Bare numbers: feet, then inches (with a fraction).
        let mut parts = t.split_whitespace();
        let feet = parts.next().and_then(parse_mixed);
        let rest: Vec<&str> = parts.collect();
        let inches = if rest.is_empty() { Some(0.0) } else { parse_mixed(&rest.join(" ")) };
        feet.zip(inches).map(|(f, i)| f * 12.0 + i)
    } else if unit == cc_core::LengthUnit::FractionalInch || t.contains('/') {
        parse_mixed(&t).map(|v| v / unit.per_meter() / cc_core::units::INCH)
    } else {
        return None;
    };
    let m = inches.map(|i| i * cc_core::units::INCH).filter(|m| m.is_finite());
    Some(m.map(|m| if negative { -m } else { m }))
}

/// Parse a length typed in the length unit (a trailing Latin or Russian
/// unit symbol, or feet and inch marks, override it).
pub fn parse_length_with(units: &Units, text: &str) -> Option<f64> {
    let t = text.trim();
    for unit in [cc_core::LengthUnit::Millimeter, cc_core::LengthUnit::Centimeter, cc_core::LengthUnit::Meter] {
        let symbol = unit.symbol();
        // A Russian inch also ends in "м". The preceding text must be a number.
        if let Some(v) =
            t.strip_suffix(symbol).or_else(|| t.strip_suffix(russian_symbol(symbol))).and_then(parse_number)
        {
            return Some(v / unit.per_meter());
        }
    }
    if let Some(m) = parse_imperial(units, t) {
        return m;
    }
    parse_number(t).map(|v| v / units.length.unit.per_meter())
}

/// Parse a small size (see [`size_with`]) typed in a project of `units`:
/// in imperial projects a bare number is inches (`3/4`, `1 1/2`, `6`);
/// otherwise, and with feet or a unit typed, as [`parse_length_with`].
pub fn parse_size_with(units: &Units, text: &str) -> Option<f64> {
    let t = text.trim();
    if units.system() == cc_core::MeasurementSystem::Imperial {
        let (negative, rest) = match t.strip_prefix('-') {
            Some(rest) => (true, rest.trim_start()),
            None => (false, t),
        };
        // One sign, before the number.
        if negative && rest.starts_with('-') {
            return None;
        }
        let bare = !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || " ./,-".contains(c));
        if bare && !rest.contains('-') {
            let m = parse_mixed(rest)? * cc_core::units::INCH;
            let m = if negative { -m } else { m };
            return m.is_finite().then_some(m + 0.0);
        }
    }
    parse_length_with(units, t)
}

// --- Current settings ---

/// Plain number with the current decimal symbol and grouping.
pub fn number(value: f64, decimals: usize) -> String {
    let (u, c) = current();
    number_with(&u, c, value, decimals)
}

/// Length in the project's length unit (dimensions, tags, schedules).
pub fn meters(value: f64) -> String {
    let (u, c) = current();
    length_with(&u, c, value)
}

/// Length with its unit symbol.
pub fn length(value: f64) -> String {
    let (u, c) = current();
    length_unit_with(&u, c, value)
}

/// Whether lengths are shown in US customary units.
pub fn imperial() -> bool {
    current().0.length.unit.is_imperial()
}

/// A bar's length (or a leg's, a radius, a spacing) as bending schedules
/// and bar tags write it: whole centimeters without the unit in metric
/// projects, the project's length format in imperial ones.
pub fn bar_length(meters: f64) -> String {
    if imperial() { self::meters(meters) } else { number(meters * 100.0, 0) }
}

/// A number with up to `decimals` places, trailing zeros dropped (12.5, 10).
pub fn trimmed(value: f64, decimals: usize) -> String {
    let (u, c) = current();
    trimmed_with(&u, c, value, decimals)
}

/// The units and decimal symbol bar diameters are written with: a decimal
/// point when the project asks for one (`Units::diameter_point`, #437).
fn bar_current() -> (Units, bool) {
    let (mut units, comma) = current();
    if units.diameter_point {
        units.style = cc_core::NumberStyle::PointDecimal;
    }
    (units, comma)
}

/// A bar diameter as bar tags write it: `Ø12.5` (millimeters), `Ø1/2"` in imperial projects.
pub fn bar_diameter(meters: f64) -> String {
    if imperial() {
        return format!("Ø{}", inch_size(meters));
    }
    let (units, comma) = bar_current();
    format!("Ø{}", trimmed_with(&units, comma, meters * 1000.0, 1))
}

/// A bar diameter as schedules write it: `12,5 mm` (or `12.5 mm` with
/// `Units::diameter_point`), `1/2"` in imperial projects.
pub fn bar_size(meters: f64) -> String {
    let (units, comma) = bar_current();
    size_with(&units, comma, meters, MetricSize::Mm(1))
}

/// The scale 1:`n` in the current units (see [`scale_with`]).
pub fn scale(n: u32) -> String {
    scale_with(&current().0, n)
}

/// A small size in inches with the inch mark (see [`inch_size_with`]).
pub fn inch_size(meters: f64) -> String {
    let (u, c) = current();
    inch_size_with(&u, c, meters)
}

/// A small size in the current units (see [`size_with`]).
pub fn size(meters: f64, metric: MetricSize) -> String {
    let (u, c) = current();
    size_with(&u, c, meters, metric)
}

/// A physical quantity in the current units (see [`quantity_with`]).
pub fn quantity(q: Quantity, si: f64, decimals: usize) -> String {
    let (u, c) = current();
    quantity_with(&u, c, q, si, decimals)
}

/// The measurement system of the current units.
pub fn system() -> cc_core::MeasurementSystem {
    current().0.system()
}

/// Parse a small size in the current units (see [`parse_size_with`]).
pub fn parse_size(text: &str) -> Option<f64> {
    parse_size_with(&current().0, text)
}

/// Parse a slope in the current units (see [`parse_slope_with`]).
pub fn parse_slope(text: &str) -> Option<f64> {
    parse_slope_with(&current().0, text)
}

pub fn area(value: f64) -> String {
    let (u, c) = current();
    area_with(&u, c, value)
}

pub fn volume(value: f64) -> String {
    let (u, c) = current();
    volume_with(&u, c, value)
}

pub fn angle(degrees: f64) -> String {
    let (u, c) = current();
    angle_with(&u, c, degrees)
}

pub fn slope(value: f64) -> String {
    let (u, c) = current();
    slope_with(&u, c, value)
}

pub fn money(value: f64) -> String {
    let (u, c) = current();
    money_with(&u, c, value)
}

pub fn parse_length(text: &str) -> Option<f64> {
    parse_length_with(&current().0, text)
}

/// A size on paper in the current units (see [`paper_with`]).
pub fn paper(mm: f64) -> String {
    let (u, c) = current();
    paper_with(&u, c, mm)
}

/// A pen weight in the current units (see [`paper_weight_with`]).
pub fn paper_weight(mm: f64) -> String {
    let (u, c) = current();
    paper_weight_with(&u, c, mm)
}

/// Parse a size on paper in the current units (see [`parse_paper_with`]).
pub fn parse_paper(text: &str) -> Option<f64> {
    parse_paper_with(&current().0, text)
}

/// Level elevation with an explicit sign, e.g. `+3,00`.
pub fn elevation(value: f64) -> String {
    let sign = if value >= 0.0 { "+" } else { "-" };
    format!("{sign}{}", meters(value.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use cc_core::{LengthUnit, NumberStyle, SlopeUnit};

    /// Bar diameters follow the number style unless the project asks for
    /// a decimal point, which leaves every other number alone (#437).
    #[test]
    fn bar_diameters_may_take_a_point() {
        set_decimal_comma(true);
        let mut u = Units { style: NumberStyle::CommaDecimal, ..Units::default() };
        set_units(&u);
        assert_eq!(bar_diameter(0.0125), "Ø12,5");
        assert_eq!(bar_size(0.0063), "6,3 mm");
        u.diameter_point = true;
        set_units(&u);
        assert_eq!(bar_diameter(0.0125), "Ø12.5");
        assert_eq!(bar_diameter(0.010), "Ø10");
        assert_eq!(bar_size(0.0063), "6.3 mm");
        assert_eq!(length(0.8), "0,80 m");
        assert_eq!(size(0.0125, MetricSize::Mm(1)), "12,5 mm");
        // Imperial bars keep their fractions.
        set_units(&Units { diameter_point: true, ..Units::imperial() });
        assert_eq!(bar_diameter(0.5 * cc_core::units::INCH), "Ø1/2\"");
    }

    #[test]
    fn defaults_match_the_language() {
        let u = Units::default();
        assert_eq!(length_with(&u, true, 0.8), "0,80");
        assert_eq!(length_with(&u, false, 0.8), "0.80");
        assert_eq!(area_with(&u, true, 28.37), "28,37 m²");
        assert_eq!(volume_with(&u, true, 1.5), "1,500 m³");
        assert_eq!(number_with(&u, true, -0.001, 2), "0,00");
        assert_eq!(number_with(&u, true, 1234567.891, 2), "1234567,89");
    }

    #[test]
    fn units_symbols_and_grouping() {
        let mut u = Units::default();
        u.length.unit = LengthUnit::Millimeter;
        u.length.decimals = 0;
        u.style = NumberStyle::CommaDecimalPointGroups;
        assert_eq!(length_with(&u, false, 5.6), "5.600");
        assert_eq!(length_unit_with(&u, false, 10.9), "10.900 mm");
        u.length.symbol = true;
        assert_eq!(length_with(&u, false, 0.15), "150 mm");
        u.style = NumberStyle::PointDecimalCommaGroups;
        assert_eq!(number_with(&u, true, -1234567.5, 1), "-1,234,567.5");
        u.style = NumberStyle::CommaDecimalSpaceGroups;
        assert_eq!(number_with(&u, true, 1234.5, 1), "1\u{202f}234,5");
        u.slope.unit = SlopeUnit::Degrees;
        u.slope.decimals = 0;
        assert_eq!(slope_with(&u, true, 1.0), "45°");
        u.currency = "R$".into();
        assert_eq!(money_with(&u, true, 1500.0), "R$ 1\u{202f}500,00");
        assert_eq!(angle_with(&u, true, 90.0), "90,00°");
    }

    #[test]
    fn parsing_accepts_both_symbols_and_units() {
        assert_eq!(parse_number("3,5"), Some(3.5));
        assert_eq!(parse_number("1.234,5"), Some(1234.5));
        assert_eq!(parse_number("1,234.5"), Some(1234.5));
        assert_eq!(parse_number(" 1 234 "), Some(1234.0));
        assert_eq!(parse_number("abc"), None);
        let mut u = Units::default();
        u.length.unit = LengthUnit::Millimeter;
        assert_eq!(parse_length_with(&u, "150"), Some(0.15));
        assert_eq!(parse_length_with(&u, "2,5 m"), Some(2.5));
        assert_eq!(parse_length_with(&u, "30cm"), Some(0.3));
    }

    #[test]
    fn parsing_accepts_russian_unit_symbols() {
        let units = Units::imperial();
        for (latin, cyrillic) in [
            ("3 m", "3 м"),
            ("250 mm", "250 мм"),
            ("250mm", "250мм"),
            ("-2,5 cm", "-2,5 см"),
            ("2 ft", "2 фут"),
            ("6 1/2 in", "6 1/2 дюйм"),
            ("12ft 6in", "12 фут 6 дюйм"),
        ] {
            let expected = parse_length_with(&units, latin).unwrap();
            assert_eq!(parse_length_with(&units, cyrillic), Some(expected), "{cyrillic}");
            assert_eq!(parse_size_with(&units, cyrillic), Some(expected), "{cyrillic}");
        }
        for (latin, cyrillic) in [("2 mm", "2 мм"), ("2 cm", "2 СМ"), ("1/2 in", "1/2 дюйм")] {
            let expected = parse_paper_with(&units, latin).unwrap();
            assert_eq!(parse_paper_with(&units, cyrillic), Some(expected), "{cyrillic}");
        }
        assert_eq!(parse_length_with(&units, "мм"), None);
    }

    #[test]
    fn feet_and_fractional_inches() {
        let mut u = Units::imperial();
        let ft = |f: f64, i: f64| f * 0.3048 + i * 0.0254;
        assert_eq!(length_with(&u, false, ft(12.0, 6.5)), "12'-6 1/2\"");
        assert_eq!(length_with(&u, false, ft(0.0, 0.0)), "0'-0\"");
        assert_eq!(length_with(&u, false, ft(3.0, 0.0)), "3'-0\"");
        assert_eq!(length_with(&u, false, ft(0.0, 0.25)), "0'-0 1/4\"");
        assert_eq!(length_with(&u, false, -ft(1.0, 2.0)), "-1'-2\"");
        // Rounding carries into the feet: 11 31/32" to the nearest 1/16" is 1'-0".
        assert_eq!(length_with(&u, false, ft(0.0, 11.0 + 31.0 / 32.0)), "1'-0\"");
        assert_eq!(length_with(&u, false, ft(1234.0, 0.0)), "1,234'-0\"");
        // The precision: 1/2^decimals.
        u.length.decimals = 1;
        assert_eq!(length_with(&u, false, ft(0.0, 6.3)), "0'-6 1/2\"");
        u.length.decimals = 6;
        assert_eq!(length_with(&u, false, ft(0.0, 1.0 / 64.0)), "0'-0 1/64\"");
        u.length.unit = cc_core::LengthUnit::FractionalInch;
        u.length.decimals = 3;
        assert_eq!(length_with(&u, false, ft(12.0, 6.125)), "150 1/8\"");
        assert_eq!(length_unit_with(&u, false, ft(0.0, 0.5)), "1/2\"");
        u.length.unit = cc_core::LengthUnit::Foot;
        u.length.decimals = 2;
        u.length.symbol = true;
        assert_eq!(length_with(&u, false, ft(12.0, 6.0)), "12.50 ft");
        u.length.unit = cc_core::LengthUnit::Inch;
        assert_eq!(length_unit_with(&u, false, ft(1.0, 0.5)), "12.50 in");
        u.area.unit = cc_core::AreaUnit::SquareFoot;
        assert_eq!(area_with(&u, false, ft(10.0, 0.0) * ft(10.0, 0.0)), "100.00 ft²");
        u.volume.unit = cc_core::VolumeUnit::CubicYard;
        u.volume.decimals = 2;
        assert_eq!(volume_with(&u, false, 27.0 * ft(1.0, 0.0).powi(3)), "1.00 yd³");
        // Decimal inches in decimal units.
        assert_eq!(inch_size_with(&u, false, ft(0.0, 7.5)), "7.50\"");
        assert_eq!(inch_size_with(&Units::imperial(), false, ft(0.0, 7.5)), "7 1/2\"");
        assert_eq!(inch_size_with(&Units::imperial(), false, ft(0.0, 8.0)), "8\"");
    }

    #[test]
    fn parsing_feet_and_inches() {
        let u = Units::imperial();
        let ft = |f: f64, i: f64| Some(f * 0.3048 + i * 0.0254);
        let close = |a: Option<f64>, b: Option<f64>| match (a, b) {
            (Some(a), Some(b)) => (a - b).abs() < 1e-9,
            _ => false,
        };
        for (text, want) in [
            ("12'6\"", ft(12.0, 6.0)),
            ("12'-6 1/2\"", ft(12.0, 6.5)),
            ("12' 6.5\"", ft(12.0, 6.5)),
            ("12' 6,5\"", ft(12.0, 6.5)),
            ("12'", ft(12.0, 0.0)),
            ("6 1/2\"", ft(0.0, 6.5)),
            ("6-1/2\"", ft(0.0, 6.5)),
            ("3/4\"", ft(0.0, 0.75)),
            ("12ft 6in", ft(12.0, 6.0)),
            ("12 ft", ft(12.0, 0.0)),
            ("6 in", ft(0.0, 6.0)),
            ("-1'-2\"", ft(-1.0, -2.0)),
            // Bare numbers in feet and inches: feet, then inches.
            ("10", ft(10.0, 0.0)),
            ("10 6", ft(10.0, 6.0)),
            ("10 6 1/2", ft(10.0, 6.5)),
            ("2.5", ft(2.5, 0.0)),
            // Metric still works.
            ("2,5 m", Some(2.5)),
            ("30 cm", Some(0.3)),
        ] {
            assert!(close(parse_length_with(&u, text), want), "{text}: {:?}", parse_length_with(&u, text));
        }
        for bad in ["", "'", "1/0\"", "abc", "1 2 3 4", "12'abc"] {
            assert_eq!(parse_length_with(&u, bad), None, "{bad}");
        }
        let mut u = u;
        u.length.unit = cc_core::LengthUnit::FractionalInch;
        assert!(close(parse_length_with(&u, "6 1/2"), ft(0.0, 6.5)));
        u.length.unit = cc_core::LengthUnit::Inch;
        assert!(close(parse_length_with(&u, "6.5"), ft(0.0, 6.5)));
        u.length.unit = cc_core::LengthUnit::Foot;
        assert!(close(parse_length_with(&u, "1 1/2"), ft(1.5, 0.0)));
    }

    #[test]
    fn paper_sizes_in_both_systems() {
        let metric = Units::default();
        assert_eq!(paper_with(&metric, true, 2.5), "2,5 mm");
        assert_eq!(paper_with(&metric, false, 2.5), "2.5 mm");
        assert_eq!(paper_with(&metric, true, 420.0), "420 mm");
        assert_eq!(paper_with(&metric, true, 0.126), "0,13 mm");
        assert_eq!(paper_with(&metric, true, f64::NAN), "0 mm");
        assert_eq!(paper_weight_with(&metric, true, 0.25), "0,25 mm");
        assert_eq!(paper_weight_with(&metric, false, 0.1), "0.10 mm");
        let imperial = Units::imperial();
        let inch = 25.4;
        assert_eq!(paper_with(&imperial, false, 3.0 / 32.0 * inch), "3/32\"");
        assert_eq!(paper_with(&imperial, false, 0.125 * inch), "1/8\"");
        assert_eq!(paper_with(&imperial, false, 1.5 * inch), "1 1/2\"");
        assert_eq!(paper_with(&imperial, false, 17.0 * inch), "17\"");
        assert_eq!(paper_with(&imperial, false, 1.0 / 64.0 * inch), "1/64\"");
        assert_eq!(paper_with(&imperial, false, 0.0), "0\"");
        // Off the 1/64 steps: decimal inches.
        assert_eq!(paper_with(&imperial, false, 2.5), "0.098\"");
        // Within 0.005 mm of a step still reads as the fraction.
        assert_eq!(paper_with(&imperial, false, 0.125 * inch + 0.004), "1/8\"");
        assert_eq!(paper_weight_with(&imperial, false, 0.254), "0.010\"");
        assert_eq!(paper_weight_with(&imperial, false, 0.25), "0.010\"");
        assert_eq!(paper_weight_with(&imperial, false, 0.5), "0.020\"");
    }

    #[test]
    fn parsing_paper_sizes() {
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
        let metric = Units::default();
        let imperial = Units::imperial();
        let inch = 25.4;
        for (units, text, want) in [
            (&metric, "2,5", 2.5),
            (&metric, "2.5 mm", 2.5),
            (&metric, "1 cm", 10.0),
            (&metric, "1/8\"", 0.125 * inch),
            (&metric, "0.5 in", 0.5 * inch),
            (&metric, "1/2", 0.5),
            (&imperial, "1/8", 0.125 * inch),
            (&imperial, "3/32\"", 3.0 / 32.0 * inch),
            (&imperial, "1 1/2\"", 1.5 * inch),
            (&imperial, "1-1/2\"", 1.5 * inch),
            (&imperial, "0.098", 0.098 * inch),
            (&imperial, "0,010\"", 0.01 * inch),
            (&imperial, "2.5 mm", 2.5),
            (&imperial, "2.5mm", 2.5),
            (&imperial, "1 IN", inch),
            (&imperial, "-1/2\"", -0.5 * inch),
            (&imperial, "-1 1/2\"", -1.5 * inch),
        ] {
            assert!(close(parse_paper_with(units, text), want), "{text}: {:?}", parse_paper_with(units, text));
        }
        let zero = parse_paper_with(&imperial, "-0");
        assert!(zero.is_some_and(|z| z == 0.0 && z.is_sign_positive()), "{zero:?}");
        for bad in ["", "abc", "mm", "\"", "1/0", "1 2 3", "--1", "1e999", "inf", "NaN", "1/2/3", "- -1", "1 -mm"] {
            assert_eq!(parse_paper_with(&imperial, bad), None, "{bad}");
            assert_eq!(parse_paper_with(&metric, bad), None, "{bad}");
        }
    }

    #[test]
    fn small_sizes_in_both_systems() {
        let metric = Units::default();
        let imperial = Units::imperial();
        let inch = cc_core::units::INCH;
        for (metric_size, want) in [
            (MetricSize::Length, "0,20 m"),
            (MetricSize::Mm(1), "200,0 mm"),
            (MetricSize::MmNumber(0), "200"),
            (MetricSize::Cm(1), "20,0 cm"),
            (MetricSize::CmNumber(0), "20"),
            (MetricSize::CmTag, "20"),
        ] {
            assert_eq!(size_with(&metric, true, 0.2, metric_size), want);
            // Imperial projects: inches with the mark, whatever the metric form.
            assert_eq!(size_with(&imperial, false, 8.0 * inch, metric_size), "8\"");
        }
        assert_eq!(size_with(&metric, true, 0.125, MetricSize::CmTag), "12.5");
        assert_eq!(size_with(&metric, false, f64::NAN, MetricSize::Mm(0)), "0 mm");
        assert_eq!(inch_size_with(&imperial, false, 1.25 * inch), "1 1/4\"");
        assert_eq!(inch_size_with(&imperial, false, 5.0 / 16.0 * inch), "5/16\"");
        assert_eq!(inch_size_with(&imperial, false, 16.0 * inch), "16\"");
        assert_eq!(inch_size_with(&imperial, false, 0.0), "0\"");
        assert_eq!(inch_size_with(&imperial, false, -0.5 * inch), "-1/2\"");
        assert_eq!(inch_size_with(&imperial, false, f64::INFINITY), "0\"");
        // To the length precision: 1/16" by default, 1/8" at three.
        assert_eq!(inch_size_with(&imperial, false, 0.008), "5/16\"");
        let mut eighths = imperial.clone();
        eighths.length.decimals = 3;
        assert_eq!(inch_size_with(&eighths, false, 0.008), "3/8\"");
        // Metric lengths: sixteenths.
        assert_eq!(inch_size_with(&metric, false, 0.75 * inch), "3/4\"");
    }

    #[test]
    fn parsing_small_sizes() {
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
        let imperial = Units::imperial();
        let metric = Units::default();
        let inch = cc_core::units::INCH;
        for (units, text, want) in [
            (&imperial, "3/4", 0.75 * inch),
            (&imperial, "1 1/2", 1.5 * inch),
            (&imperial, "6", 6.0 * inch),
            (&imperial, "6.5", 6.5 * inch),
            (&imperial, "1 1/4\"", 1.25 * inch),
            (&imperial, "1'-2\"", 14.0 * inch),
            (&imperial, "1'", 12.0 * inch),
            (&imperial, "25 mm", 0.025),
            (&imperial, "-1/2", -0.5 * inch),
            (&metric, "0,15", 0.15),
            (&metric, "15 cm", 0.15),
        ] {
            assert!(close(parse_size_with(units, text), want), "{text}: {:?}", parse_size_with(units, text));
        }
        for bad in ["", "abc", "1/0", "1 2 3", "--1"] {
            assert_eq!(parse_size_with(&imperial, bad), None, "{bad}");
        }
        // What a project shows reads back.
        for sixteenths in 0..=(16 * 30) {
            let m = f64::from(sixteenths) / 16.0 * inch;
            let text = inch_size_with(&imperial, false, m);
            assert!(close(parse_size_with(&imperial, &text), m), "{text}");
        }
    }

    #[test]
    fn masses_and_stresses_in_both_systems() {
        let metric = Units::default();
        let imperial = Units::imperial();
        assert_eq!(quantity_with(&metric, true, Quantity::Mass, 3.7, 2), "3,70 kg");
        assert_eq!(quantity_with(&imperial, false, Quantity::Mass, POUND * 10.0, 2), "10.00 lb");
        assert_eq!(quantity_with(&metric, true, Quantity::LinearMass, 96.0, 2), "96,00 kg/m");
        // 1 lb/ft is 1.488 kg/m.
        assert_eq!(quantity_with(&imperial, false, Quantity::LinearMass, 1.488_163_9, 2), "1.00 lb/ft");
        assert_eq!(quantity_with(&imperial, false, Quantity::AreaMass, 4.882_428, 1), "1.0 lb/ft²");
        assert_eq!(quantity_with(&metric, true, Quantity::AreaMass, 45.0, 0), "45 kg/m²");
        // Steel: 7850 kg/m³ is 490 lb/ft³; 200 GPa is 29,008 ksi.
        assert_eq!(quantity_with(&imperial, false, Quantity::Density, 7850.0, 0), "490 lb/ft³");
        assert_eq!(quantity_with(&imperial, false, Quantity::Stress, 200_000.0, 0), "29,008 ksi");
        assert_eq!(quantity_with(&metric, true, Quantity::Stress, 25.0, 0), "25 MPa");
        assert_eq!(quantity_with(&imperial, false, Quantity::Mass, f64::NAN, 1), "0.0 lb");
        // 1 l/s is 15.85 US gallons per minute.
        assert_eq!(quantity_with(&imperial, false, Quantity::Flow, 1.0, 2), "15.85 gpm");
        assert_eq!(quantity_with(&metric, true, Quantity::Flow, 0.25, 2), "0,25 l/s");
        let all = [Quantity::Mass, Quantity::LinearMass, Quantity::AreaMass, Quantity::Density, Quantity::Stress];
        for q in all.into_iter().chain([Quantity::Flow]) {
            assert_eq!(q.per_si(cc_core::MeasurementSystem::Metric), 1.0);
            assert!(q.per_si(cc_core::MeasurementSystem::Imperial) > 0.0);
        }
    }

    #[test]
    fn slopes_as_rise_per_twelve() {
        let imperial = Units::imperial();
        assert_eq!(imperial.slope.unit, SlopeUnit::RisePer12);
        assert_eq!(slope_with(&imperial, false, 0.5), "6:12");
        assert_eq!(slope_with(&imperial, false, 4.5 / 12.0), "4.5:12");
        let comma = Units { style: NumberStyle::Language, ..imperial.clone() };
        assert_eq!(slope_with(&comma, true, 4.5 / 12.0), "4,5:12");
        assert_eq!(slope_with(&imperial, false, 0.0), "0:12");
        assert_eq!(slope_with(&imperial, false, f64::NAN), "0:12");
        let mut bare = imperial.clone();
        bare.slope.symbol = false;
        assert_eq!(slope_with(&bare, false, 0.5), "6");
        // Metric projects are unchanged.
        assert_eq!(slope_with(&Units::default(), true, 0.0833), "8,3 %");
    }

    #[test]
    fn parsing_slopes() {
        let close = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-9);
        let imperial = Units::imperial();
        let metric = Units::default();
        let mut degrees = Units::default();
        degrees.slope.unit = SlopeUnit::Degrees;
        for (units, text, want) in [
            (&imperial, "6:12", 0.5),
            (&imperial, "6/12", 0.5),
            (&imperial, "6", 0.5),
            (&imperial, "4 1/2", 0.375),
            (&imperial, "1 1/2:12", 0.125),
            (&imperial, "4,5:12", 0.375),
            (&imperial, "8.33 %", 0.0833),
            (&imperial, "45°", 1.0),
            (&imperial, "-1/4:12", -1.0 / 48.0),
            (&metric, "8,3", 0.083),
            (&metric, "8.3%", 0.083),
            (&metric, "6:12", 0.5),
            (&degrees, "45", 1.0),
            (&degrees, "45 deg", 1.0),
        ] {
            assert!(close(parse_slope_with(units, text), want), "{text}: {:?}", parse_slope_with(units, text));
        }
        for bad in ["", "abc", ":12", "6:", "6:0", "1/0", "90°", "-90°", "6:-12", "inf", "NaN %", "1:2:3"] {
            assert_eq!(parse_slope_with(&imperial, bad), None, "{bad}");
        }
        assert_eq!(parse_slope_with(&degrees, "90"), None);
        // What a project shows reads back.
        for units in [&imperial, &metric, &degrees] {
            for rise in 0..=24 {
                let slope = f64::from(rise) / 12.0;
                let text = slope_with(units, false, slope);
                let back = parse_slope_with(units, &text).unwrap_or(f64::NAN);
                assert_eq!(slope_with(units, false, back), text);
                if units.slope.unit == SlopeUnit::RisePer12 {
                    assert!((back - slope).abs() < 1e-9, "{text}: {back}");
                }
            }
        }
    }

    #[test]
    fn paper_sizes_round_trip() {
        let imperial = Units::imperial();
        let metric = Units::default();
        for step in 0..=(64 * 50) {
            let mm = f64::from(step) / 64.0 * 25.4;
            for (units, comma) in [(&imperial, false), (&imperial, true), (&metric, true), (&metric, false)] {
                let text = paper_with(units, comma, mm);
                let back = parse_paper_with(units, &text);
                // Metric keeps two decimals: within 0.005 mm.
                let metric = units.system() == cc_core::MeasurementSystem::Metric;
                let tolerance = if metric { 0.005 + 1e-9 } else { 1e-9 };
                assert!(back.is_some_and(|b| (b - mm).abs() <= tolerance), "{text}: {back:?} vs {mm}");
            }
        }
    }
}
