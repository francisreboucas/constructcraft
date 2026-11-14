//! Where the project stands on Earth: the geographic position of the
//! project origin, its height above sea level, the direction of true north
//! and the parts of its postal address, plus the conversion between
//! latitude/longitude and the project's own coordinates (meters) used to
//! bring map data in.

use glam::DVec2;
use serde::{Deserialize, Serialize};

/// WGS 84 semi-major axis, meters.
const WGS84_A: f64 = 6_378_137.0;
/// WGS 84 first eccentricity squared.
const WGS84_E2: f64 = 6.694_379_990_14e-3;

/// Lowest and highest site elevation accepted, meters (the Dead Sea shore to above Everest).
pub const ELEVATION_RANGE: std::ops::RangeInclusive<f64> = -500.0..=9_000.0;
/// Longest text kept in one address part, bytes.
pub const MAX_TEXT: usize = 512;

/// The geographic location of the project origin (`[0, 0]` at level
/// elevation 0) with its postal address.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SiteLocation {
    /// Latitude in degrees (WGS 84), north positive.
    pub latitude: f64,
    /// Longitude in degrees (WGS 84), east positive.
    pub longitude: f64,
    /// Height of the project's elevation 0 above sea level, meters.
    #[serde(default)]
    pub elevation: f64,
    /// Angle from the project's north (+Y) to true north, degrees counter-clockwise.
    #[serde(default)]
    pub true_north: f64,
    /// The place as the map search named it.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub place: String,
    /// Parts of the postal address.
    #[serde(default)]
    pub address: SiteAddress,
    /// Local time's offset from UTC, hours east (within
    /// [`crate::sun::UTC_OFFSET_RANGE`]); `None`: the offset of the
    /// longitude's time zone (see [`SiteLocation::offset_hours`]).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub utc_offset: Option<f64>,
}

/// Parts of a postal address (any may be empty).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SiteAddress {
    pub street: String,
    pub number: String,
    pub district: String,
    pub city: String,
    pub region: String,
    pub postcode: String,
    pub country: String,
}

impl SiteAddress {
    /// The parts with their keys, in display order.
    pub fn parts(&self) -> [(&'static str, &str); 7] {
        [
            ("street", &self.street),
            ("number", &self.number),
            ("district", &self.district),
            ("city", &self.city),
            ("region", &self.region),
            ("postcode", &self.postcode),
            ("country", &self.country),
        ]
    }

    /// The part called `key`, to change it.
    pub fn part_mut(&mut self, key: &str) -> Option<&mut String> {
        Some(match key {
            "street" => &mut self.street,
            "number" => &mut self.number,
            "district" => &mut self.district,
            "city" => &mut self.city,
            "region" => &mut self.region,
            "postcode" => &mut self.postcode,
            "country" => &mut self.country,
            _ => return None,
        })
    }

    /// One line: "street, number - district, city - region, postcode, country", empty parts left out.
    pub fn one_line(&self) -> String {
        let join = |parts: &[&str], sep: &str| {
            parts.iter().map(|s| s.trim()).filter(|s| !s.is_empty()).collect::<Vec<_>>().join(sep)
        };
        let street = join(&[&self.street, &self.number], ", ");
        let city = join(&[&self.city, &self.region], " - ");
        let first = join(&[&street, &self.district], " - ");
        join(&[&first, &city, &self.postcode, &self.country], ", ")
    }

    pub fn is_empty(&self) -> bool {
        self.parts().iter().all(|(_, v)| v.trim().is_empty())
    }
}

impl SiteLocation {
    /// A location at `latitude`, `longitude` with nothing else set.
    pub fn at(latitude: f64, longitude: f64) -> SiteLocation {
        SiteLocation {
            latitude,
            longitude,
            elevation: 0.0,
            true_north: 0.0,
            place: String::new(),
            address: SiteAddress::default(),
            utc_offset: None,
        }
    }

    /// Local time's offset from UTC, hours east: the one set, else the
    /// longitude's nominal zone (`longitude / 15`, rounded).
    pub fn offset_hours(&self) -> f64 {
        let range = &crate::sun::UTC_OFFSET_RANGE;
        match self.utc_offset {
            Some(o) if o.is_finite() && range.contains(&o) => o,
            _ if self.longitude.is_finite() => (self.longitude / 15.0).round().clamp(*range.start(), *range.end()),
            _ => 0.0,
        }
    }

    /// Finite coordinates on Earth, a finite elevation in range, a finite
    /// north angle and an offset from UTC in range, if set.
    pub fn is_valid(&self) -> bool {
        self.latitude.is_finite()
            && self.longitude.is_finite()
            && (-90.0..=90.0).contains(&self.latitude)
            && (-180.0..=180.0).contains(&self.longitude)
            && self.elevation.is_finite()
            && ELEVATION_RANGE.contains(&self.elevation)
            && self.true_north.is_finite()
            && self.utc_offset.is_none_or(|o| o.is_finite() && crate::sun::UTC_OFFSET_RANGE.contains(&o))
    }

    /// Fix what can be fixed (north angle into `(-180, 180]`, texts cut to
    /// [`MAX_TEXT`], an offset from UTC out of range dropped); `false` when
    /// the location is beyond repair.
    pub fn repair(&mut self) -> (bool, usize) {
        let mut fixes = 0;
        if self.utc_offset.is_some_and(|o| !(o.is_finite() && crate::sun::UTC_OFFSET_RANGE.contains(&o))) {
            self.utc_offset = None;
            fixes += 1;
        }
        if self.true_north.is_finite() {
            let n = normalize_angle(self.true_north);
            if n != self.true_north {
                self.true_north = n;
                fixes += 1;
            }
        }
        for text in [
            &mut self.place,
            &mut self.address.street,
            &mut self.address.number,
            &mut self.address.district,
            &mut self.address.city,
            &mut self.address.region,
            &mut self.address.postcode,
            &mut self.address.country,
        ] {
            if text.len() > MAX_TEXT {
                truncate(text, MAX_TEXT);
                fixes += 1;
            }
        }
        (self.is_valid(), fixes)
    }

    /// Meters east and north of the site for a point at `latitude`,
    /// `longitude`, on the plane tangent to the ellipsoid at the site
    /// (radii of curvature at its latitude; within a few centimeters over a
    /// kilometer).
    pub fn east_north(&self, latitude: f64, longitude: f64) -> DVec2 {
        let (m, n) = radii(self.latitude);
        let mut dlon = longitude - self.longitude;
        if dlon > 180.0 {
            dlon -= 360.0;
        } else if dlon < -180.0 {
            dlon += 360.0;
        }
        DVec2::new(
            dlon.to_radians() * n * self.latitude.to_radians().cos(),
            (latitude - self.latitude).to_radians() * m,
        )
    }

    /// Latitude and longitude of a point `east_north` meters from the site (the inverse of [`Self::east_north`]).
    pub fn lat_lon(&self, east_north: DVec2) -> (f64, f64) {
        let (m, n) = radii(self.latitude);
        let cos = self.latitude.to_radians().cos().max(1e-9);
        (self.latitude + (east_north.y / m).to_degrees(), self.longitude + (east_north.x / (n * cos)).to_degrees())
    }

    /// The project's direction of true north (unit vector in plan).
    pub fn north(&self) -> DVec2 {
        let a = self.true_north.to_radians();
        DVec2::new(-a.sin(), a.cos())
    }

    /// Project coordinates of a point at `latitude`, `longitude`.
    pub fn to_project(&self, latitude: f64, longitude: f64) -> DVec2 {
        let en = self.east_north(latitude, longitude);
        let north = self.north();
        let east = DVec2::new(north.y, -north.x);
        east * en.x + north * en.y
    }

    /// Latitude and longitude of a point in project coordinates.
    pub fn from_project(&self, p: DVec2) -> (f64, f64) {
        let north = self.north();
        let east = DVec2::new(north.y, -north.x);
        self.lat_lon(DVec2::new(p.dot(east), p.dot(north)))
    }
}

/// Meridian and prime-vertical radii of curvature at `latitude` degrees.
fn radii(latitude: f64) -> (f64, f64) {
    let s = latitude.to_radians().sin();
    let w = (1.0 - WGS84_E2 * s * s).sqrt();
    (WGS84_A * (1.0 - WGS84_E2) / (w * w * w), WGS84_A / w)
}

/// `a` degrees in `(-180, 180]`.
pub fn normalize_angle(a: f64) -> f64 {
    let r = a.rem_euclid(360.0);
    if r > 180.0 { r - 360.0 } else { r }
}

fn truncate(s: &mut String, max: usize) {
    let mut end = max.min(s.len());
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    s.truncate(end);
}

/// Degrees as IFC's compound plane angle: degrees, minutes, seconds and
/// millionths of a second, all with the sign of the angle.
pub fn to_dms(deg: f64) -> [i64; 4] {
    let sign = if deg < 0.0 { -1 } else { 1 };
    let micro = (deg.abs() * 3_600_000_000.0).round() as i64;
    let d = micro / 3_600_000_000;
    let m = micro / 60_000_000 % 60;
    let s = micro / 1_000_000 % 60;
    let u = micro % 1_000_000;
    [sign * d, sign * m, sign * s, sign * u]
}

/// Degrees from IFC's compound plane angle (three or four parts; the sign of any part makes it negative).
pub fn from_dms(parts: &[i64]) -> Option<f64> {
    if !(3..=4).contains(&parts.len()) {
        return None;
    }
    let negative = parts.iter().any(|p| *p < 0);
    let get = |i: usize| parts.get(i).map_or(0.0, |p| p.unsigned_abs() as f64);
    let deg = get(0) + get(1) / 60.0 + get(2) / 3600.0 + get(3) / 3_600_000_000.0;
    deg.is_finite().then_some(if negative { -deg } else { deg })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn near(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn one_degree_of_latitude_and_longitude() {
        // At the equator a degree of latitude is 110 574 m and of longitude 111 320 m.
        let site = SiteLocation::at(0.0, 0.0);
        let p = site.east_north(1.0, 0.0);
        assert!(near(p.y, 110_574.0, 2.0), "{p}");
        let p = site.east_north(0.0, 1.0);
        assert!(near(p.x, 111_320.0, 2.0), "{p}");
    }

    #[test]
    fn round_trip_with_true_north() {
        let mut site = SiteLocation::at(-23.5505, -46.6333);
        site.true_north = 30.0;
        let p = site.to_project(-23.551, -46.632);
        let (lat, lon) = site.from_project(p);
        assert!(near(lat, -23.551, 1e-9) && near(lon, -46.632, 1e-9));
        // True north rotated 30° counter-clockwise: a point due north lies up and to the left.
        let p = site.to_project(-23.5495, -46.6333);
        assert!(p.x < 0.0 && p.y > 0.0);
        assert!(near(p.angle_to(DVec2::Y).to_degrees(), -30.0, 1e-6));
    }

    #[test]
    fn longitude_wraps_across_the_antimeridian() {
        let site = SiteLocation::at(0.0, 179.9999);
        assert!(site.east_north(0.0, -179.9999).x > 0.0);
    }

    #[test]
    fn dms_round_trip() {
        for deg in [-23.550_520, 46.633_309, 0.0, -0.5, 89.999_999] {
            let dms = to_dms(deg);
            assert!(near(from_dms(&dms).unwrap_or(f64::NAN), deg, 1e-9), "{deg} {dms:?}");
        }
        assert_eq!(to_dms(-23.5), [-23, -30, 0, 0]);
        assert_eq!(from_dms(&[1, 2]), None);
    }

    #[test]
    fn repair_rejects_off_earth_and_normalizes_north() {
        let mut site = SiteLocation::at(10.0, 20.0);
        site.true_north = 370.0;
        site.place = "é".repeat(400);
        let (ok, fixes) = site.repair();
        assert!(ok);
        assert_eq!(fixes, 2);
        assert!(near(site.true_north, 10.0, 1e-9));
        assert!(site.place.len() <= MAX_TEXT);
        let mut bad = SiteLocation::at(f64::NAN, 0.0);
        assert!(!bad.repair().0);
        let mut bad = SiteLocation::at(0.0, 200.0);
        assert!(!bad.repair().0);
    }

    #[test]
    fn address_line() {
        let a = SiteAddress {
            street: "Rua A".into(),
            number: "10".into(),
            district: "Centro".into(),
            city: "Fortaleza".into(),
            region: "CE".into(),
            postcode: "60000-000".into(),
            country: "Brasil".into(),
        };
        assert_eq!(a.one_line(), "Rua A, 10 - Centro, Fortaleza - CE, 60000-000, Brasil");
        assert_eq!(SiteAddress { city: "X".into(), ..Default::default() }.one_line(), "X");
    }
}
