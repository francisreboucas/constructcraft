//! Materials: identity data, graphics in views (shading, transparency,
//! surface and cut patterns), render appearance and physical properties.
//! Types refer to materials by name.

use serde::{Deserialize, Serialize};

/// Broad family of a material (filters, defaults and IFC categories).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum MaterialClass {
    #[default]
    Generic,
    Concrete,
    Masonry,
    Mortar,
    Metal,
    Wood,
    Glass,
    Plastic,
    Stone,
    Ceramic,
    Gypsum,
    Insulation,
    Fabric,
    Paint,
    Earth,
    Liquid,
    Gas,
}

impl MaterialClass {
    pub const ALL: [MaterialClass; 17] = [
        MaterialClass::Generic,
        MaterialClass::Concrete,
        MaterialClass::Masonry,
        MaterialClass::Mortar,
        MaterialClass::Metal,
        MaterialClass::Wood,
        MaterialClass::Glass,
        MaterialClass::Plastic,
        MaterialClass::Stone,
        MaterialClass::Ceramic,
        MaterialClass::Gypsum,
        MaterialClass::Insulation,
        MaterialClass::Fabric,
        MaterialClass::Paint,
        MaterialClass::Earth,
        MaterialClass::Liquid,
        MaterialClass::Gas,
    ];

    pub fn key(self) -> &'static str {
        match self {
            MaterialClass::Generic => "matclass-generic",
            MaterialClass::Concrete => "matclass-concrete",
            MaterialClass::Masonry => "matclass-masonry",
            MaterialClass::Mortar => "matclass-mortar",
            MaterialClass::Metal => "matclass-metal",
            MaterialClass::Wood => "matclass-wood",
            MaterialClass::Glass => "matclass-glass",
            MaterialClass::Plastic => "matclass-plastic",
            MaterialClass::Stone => "matclass-stone",
            MaterialClass::Ceramic => "matclass-ceramic",
            MaterialClass::Gypsum => "matclass-gypsum",
            MaterialClass::Insulation => "matclass-insulation",
            MaterialClass::Fabric => "matclass-fabric",
            MaterialClass::Paint => "matclass-paint",
            MaterialClass::Earth => "matclass-earth",
            MaterialClass::Liquid => "matclass-liquid",
            MaterialClass::Gas => "matclass-gas",
        }
    }
}

/// Fill pattern drawn on surfaces (elevations) or cuts (plans, sections).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Pattern {
    #[default]
    None,
    Solid,
    /// Lines at 45°.
    Diagonal,
    /// Lines at 135°.
    DiagonalBack,
    /// Both diagonals.
    Crosshatch,
    Horizontal,
    Vertical,
    Grid,
    /// Running bond courses.
    Brick,
    /// Scattered dots and small triangles.
    Concrete,
    /// Fine dots.
    Sand,
    /// Zig-zag batt insulation.
    Insulation,
    /// Earth: blocks of short strokes, alternately at 45° and 135° (a weave).
    Earth,
    /// One of the project's own patterns (`Project::fill_patterns`).
    Custom(crate::FillPatternId),
}

impl Pattern {
    /// The built-in patterns (custom ones live in the project).
    pub const ALL: [Pattern; 13] = [
        Pattern::None,
        Pattern::Solid,
        Pattern::Diagonal,
        Pattern::DiagonalBack,
        Pattern::Crosshatch,
        Pattern::Horizontal,
        Pattern::Vertical,
        Pattern::Grid,
        Pattern::Brick,
        Pattern::Concrete,
        Pattern::Sand,
        Pattern::Insulation,
        Pattern::Earth,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Pattern::None => "pattern-none",
            Pattern::Solid => "pattern-solid",
            Pattern::Diagonal => "pattern-diagonal",
            Pattern::DiagonalBack => "pattern-diagonal-back",
            Pattern::Crosshatch => "pattern-crosshatch",
            Pattern::Horizontal => "pattern-horizontal",
            Pattern::Vertical => "pattern-vertical",
            Pattern::Grid => "pattern-grid",
            Pattern::Brick => "pattern-brick",
            Pattern::Concrete => "pattern-concrete",
            Pattern::Sand => "pattern-sand",
            Pattern::Insulation => "pattern-insulation",
            Pattern::Earth => "pattern-earth",
            Pattern::Custom(_) => "pattern-custom",
        }
    }
}

/// A pattern over an optional background pattern, each with its color.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PatternFill {
    pub foreground: Pattern,
    pub foreground_color: [u8; 3],
    pub background: Pattern,
    pub background_color: [u8; 3],
}

impl Default for PatternFill {
    fn default() -> Self {
        Self {
            foreground: Pattern::None,
            foreground_color: [0, 0, 0],
            background: Pattern::None,
            background_color: [255, 255, 255],
        }
    }
}

impl PatternFill {
    pub fn lines(pattern: Pattern) -> Self {
        Self { foreground: pattern, ..Self::default() }
    }

    pub fn is_empty(&self) -> bool {
        self.foreground == Pattern::None && self.background == Pattern::None
    }
}

/// Descriptive and product data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialIdentity {
    pub description: String,
    pub class: MaterialClass,
    pub comments: String,
    pub keywords: String,
    pub manufacturer: String,
    pub model: String,
    /// Unit cost in the project currency.
    pub cost: Option<f64>,
    pub url: String,
    pub keynote: String,
    pub mark: String,
}

/// How the material looks in views.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialGraphics {
    /// Shade with the appearance color instead of the material color.
    pub use_appearance: bool,
    /// 0 (opaque) to 1 (invisible).
    pub transparency: f64,
    /// Pattern on faces seen in elevation and 3D.
    pub surface: PatternFill,
    /// Pattern where the material is cut (plans, sections).
    pub cut: PatternFill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Finish {
    #[default]
    Matte,
    Satin,
    Glossy,
    Polished,
}

impl Finish {
    pub const ALL: [Finish; 4] = [Finish::Matte, Finish::Satin, Finish::Glossy, Finish::Polished];

    pub fn key(self) -> &'static str {
        match self {
            Finish::Matte => "finish-matte",
            Finish::Satin => "finish-satin",
            Finish::Glossy => "finish-glossy",
            Finish::Polished => "finish-polished",
        }
    }

    /// Strength of the specular highlight, 0 to 1.
    pub fn shininess(self) -> f64 {
        match self {
            Finish::Matte => 0.0,
            Finish::Satin => 0.25,
            Finish::Glossy => 0.55,
            Finish::Polished => 0.85,
        }
    }
}

/// Largest texture image file kept, bytes.
pub const MAX_TEXTURE_BYTES: usize = 16 << 20;
/// Most pixels a texture image may have (width × height).
pub const MAX_TEXTURE_PIXELS: u64 = 4096 * 4096;
/// Smallest and largest real-world size of one repeat of a texture, meters.
pub const MIN_TEXTURE_SIZE: f64 = 0.001;
pub const MAX_TEXTURE_SIZE: f64 = 1000.0;

fn one() -> f64 {
    1.0
}

/// An image laid on a material's surfaces in the textures style: one repeat
/// covers `width` × `height` meters, moved by `offset` and turned by
/// `rotation_deg` in the face's plane coordinates (see [`Self::map`]).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialTexture {
    /// The image file (shared: undo steps and copies do not copy the bytes).
    pub image: std::sync::Arc<crate::raster::RasterImage>,
    #[serde(default = "one")]
    pub width: f64,
    #[serde(default = "one")]
    pub height: f64,
    /// Where a repeat starts, meters along the face's plane axes.
    #[serde(default)]
    pub offset: [f64; 2],
    #[serde(default)]
    pub rotation_deg: f64,
}

impl MaterialTexture {
    /// A texture of `image` repeating every `width` × `height` meters.
    pub fn new(image: crate::raster::RasterImage, width: f64, height: f64) -> Self {
        Self { image: std::sync::Arc::new(image), width, height, offset: [0.0, 0.0], rotation_deg: 0.0 }
    }

    /// Whether the image is one a texture may hold (valid, within the size limits).
    pub fn image_ok(&self) -> bool {
        self.image.is_valid()
            && self.image.data.len() <= MAX_TEXTURE_BYTES
            && u64::from(self.image.width) * u64::from(self.image.height) <= MAX_TEXTURE_PIXELS
    }

    /// Replace sizes, offsets and angles that are not finite or out of
    /// range; returns the number of fixes (the image is checked by the caller).
    pub fn repair(&mut self) -> usize {
        let mut fixes = 0;
        for v in [&mut self.width, &mut self.height] {
            if !(v.is_finite() && (MIN_TEXTURE_SIZE..=MAX_TEXTURE_SIZE).contains(v)) {
                *v = if v.is_finite() { v.clamp(MIN_TEXTURE_SIZE, MAX_TEXTURE_SIZE) } else { 1.0 };
                fixes += 1;
            }
        }
        for v in self.offset.iter_mut().chain([&mut self.rotation_deg]) {
            if !(v.is_finite() && v.abs() <= 1e6) {
                *v = 0.0;
                fixes += 1;
            }
        }
        fixes
    }

    /// Texture coordinates (in repeats, `v` up the image) of a point with
    /// plane coordinates `uv` in meters: moved by the offset, turned by the
    /// rotation and scaled by the size.
    pub fn map(&self, uv: [f64; 2]) -> [f64; 2] {
        let (x, y) = (uv[0] - self.offset[0], uv[1] - self.offset[1]);
        let (s, c) = (-self.rotation_deg).to_radians().sin_cos();
        let (x, y) = (x * c - y * s, x * s + y * c);
        let w = if self.width.abs() > 1e-9 { self.width } else { 1.0 };
        let h = if self.height.abs() > 1e-9 { self.height } else { 1.0 };
        [x / w, y / h]
    }
}

/// Render appearance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialAppearance {
    pub color: [u8; 3],
    pub finish: Finish,
    pub description: String,
    /// An image mapped on the faces in the textures style (the color shows
    /// where it cannot be drawn).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub texture: Option<MaterialTexture>,
}

impl Default for MaterialAppearance {
    fn default() -> Self {
        Self { color: [200, 200, 200], finish: Finish::Matte, description: String::new(), texture: None }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum Behavior {
    #[default]
    Isotropic,
    Orthotropic,
}

impl Behavior {
    pub const ALL: [Behavior; 2] = [Behavior::Isotropic, Behavior::Orthotropic];

    pub fn key(self) -> &'static str {
        match self {
            Behavior::Isotropic => "behavior-isotropic",
            Behavior::Orthotropic => "behavior-orthotropic",
        }
    }
}

/// Physical properties (SI units); `None` when unknown.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MaterialPhysical {
    pub behavior: Behavior,
    /// kg/m³.
    pub density: Option<f64>,
    /// W/(m·K).
    pub thermal_conductivity: Option<f64>,
    /// J/(kg·K).
    pub specific_heat: Option<f64>,
    /// 1/K.
    pub thermal_expansion: Option<f64>,
    /// MPa.
    pub youngs_modulus: Option<f64>,
    /// Poisson's ratio.
    pub poisson: Option<f64>,
    /// MPa.
    pub compressive_strength: Option<f64>,
    /// MPa.
    pub tensile_strength: Option<f64>,
}

impl MaterialPhysical {
    /// Values as (field key, value) pairs, in display order.
    pub fn values_mut(&mut self) -> [(&'static str, &mut Option<f64>); 8] {
        [
            ("phys-density", &mut self.density),
            ("phys-conductivity", &mut self.thermal_conductivity),
            ("phys-specific-heat", &mut self.specific_heat),
            ("phys-expansion", &mut self.thermal_expansion),
            ("phys-youngs", &mut self.youngs_modulus),
            ("phys-poisson", &mut self.poisson),
            ("phys-compressive", &mut self.compressive_strength),
            ("phys-tensile", &mut self.tensile_strength),
        ]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Material {
    pub name: String,
    /// sRGB shading color.
    pub color: [u8; 3],
    #[serde(default)]
    pub identity: MaterialIdentity,
    #[serde(default)]
    pub graphics: MaterialGraphics,
    #[serde(default)]
    pub appearance: MaterialAppearance,
    #[serde(default)]
    pub physical: MaterialPhysical,
}

impl Material {
    /// A plain material: the appearance takes the shading color.
    pub fn new(name: impl Into<String>, color: [u8; 3]) -> Self {
        Self {
            name: name.into(),
            color,
            identity: Default::default(),
            graphics: Default::default(),
            appearance: MaterialAppearance { color, ..Default::default() },
            physical: Default::default(),
        }
    }

    /// Color used to shade faces in views.
    pub fn shading_color(&self) -> [u8; 3] {
        if self.graphics.use_appearance { self.appearance.color } else { self.color }
    }

    /// Replace non-finite or out-of-range numbers; returns the number of fixes.
    pub fn repair(&mut self) -> usize {
        let mut fixes = 0;
        let t = &mut self.graphics.transparency;
        if !t.is_finite() || !(0.0..=1.0).contains(t) {
            *t = if t.is_finite() { t.clamp(0.0, 1.0) } else { 0.0 };
            fixes += 1;
        }
        if self.identity.cost.is_some_and(|c| !c.is_finite()) {
            self.identity.cost = None;
            fixes += 1;
        }
        for (_, v) in self.physical.values_mut() {
            if v.is_some_and(|x| !x.is_finite()) {
                *v = None;
                fixes += 1;
            }
        }
        if let Some(texture) = &mut self.appearance.texture {
            if texture.image_ok() {
                fixes += texture.repair();
            } else {
                self.appearance.texture = None;
                fixes += 1;
            }
        }
        fixes
    }
}

/// Template data for a default material: class, cut pattern and physical values.
pub fn template_properties(m: &mut Material, class: MaterialClass) {
    let p = &mut m.physical;
    let mut set = |density: f64, conductivity: f64, heat: f64| {
        p.density = Some(density);
        p.thermal_conductivity = Some(conductivity);
        p.specific_heat = Some(heat);
    };
    let cut = match class {
        MaterialClass::Concrete => {
            set(2400.0, 1.75, 1000.0);
            m.physical.youngs_modulus = Some(25_000.0);
            m.physical.compressive_strength = Some(25.0);
            Pattern::Concrete
        }
        MaterialClass::Masonry => {
            set(1600.0, 0.70, 920.0);
            Pattern::Diagonal
        }
        MaterialClass::Mortar => {
            set(1800.0, 1.15, 1000.0);
            Pattern::Sand
        }
        MaterialClass::Wood => {
            set(600.0, 0.15, 1340.0);
            Pattern::DiagonalBack
        }
        MaterialClass::Glass => {
            set(2500.0, 1.0, 840.0);
            m.graphics.transparency = 0.6;
            Pattern::None
        }
        MaterialClass::Gypsum => {
            set(1000.0, 0.35, 840.0);
            Pattern::Sand
        }
        MaterialClass::Ceramic => {
            set(2000.0, 1.05, 920.0);
            Pattern::Crosshatch
        }
        MaterialClass::Fabric => {
            set(300.0, 0.06, 1300.0);
            Pattern::None
        }
        MaterialClass::Insulation => {
            set(30.0, 0.035, 1030.0);
            Pattern::Insulation
        }
        MaterialClass::Metal => {
            set(7850.0, 50.0, 490.0);
            Pattern::None
        }
        MaterialClass::Earth => {
            set(1800.0, 1.5, 1000.0);
            Pattern::Earth
        }
        _ => Pattern::None,
    };
    m.identity.class = class;
    m.graphics.cut = PatternFill::lines(cut);
}

/// NBR 6118:2023 §8.2.8: initial tangent modulus `Eci` and secant modulus `Ecs`
/// in MPa for a concrete class of characteristic compressive strength `fck_mpa`
/// (with coarse aggregate factor `alpha_e = 1.0`, granite/gneiss).
pub fn concrete_modulus_nbr6118(fck_mpa: f64) -> (f64, f64) {
    let eci =
        if fck_mpa <= 50.0 { 5600.0 * fck_mpa.max(0.0).sqrt() } else { 21_500.0 * (fck_mpa / 10.0 + 1.25).cbrt() };
    let alpha_i = (0.8 + 0.2 * fck_mpa / 80.0).min(1.0);
    let ecs = alpha_i * eci;
    (eci, ecs)
}

/// Structural steel grades (NBR 7007, NBR 7008 and ASTM): `(designation, fy_mpa, fu_mpa)`.
pub const BRAZIL_STEELS: &[(&str, f64, f64)] = &[
    ("MR250 (NBR 7007)", 250.0, 400.0),
    ("AR350 (NBR 7007)", 350.0, 450.0),
    ("AR415 (NBR 7007)", 415.0, 520.0),
    ("ASTM A36", 250.0, 400.0),
    ("ASTM A572 Gr50", 345.0, 450.0),
    ("ASTM A992", 345.0, 450.0),
    ("ASTM A588", 345.0, 485.0),
    ("ZAR 230 (NBR 7008)", 230.0, 310.0),
    ("ZAR 280 (NBR 7008)", 280.0, 380.0),
    ("ZAR 345 (NBR 7008)", 345.0, 430.0),
];

/// Reinforcing steel grades (NBR 7480): `(designation, fyk_mpa)`.
pub const BRAZIL_REBAR_STEELS: &[(&str, f64)] = &[("CA-25", 250.0), ("CA-50", 500.0), ("CA-60", 600.0)];

/// Concrete classes of NBR 8953 / NBR 6118:2023 (fck in MPa).
pub const BRAZIL_CONCRETE_CLASSES: &[u32] = &[20, 25, 30, 35, 40, 45, 50, 55, 60, 70, 80, 90];

/// Returns the catalog of Brazilian structural materials (NBR 8800, NBR 7007,
/// NBR 7008, ASTM, NBR 7480 and NBR 6118:2023) with mechanical properties.
pub fn brazil_structural_materials() -> Vec<Material> {
    let mut list = Vec::new();
    for &(grade, fy, _fu) in BRAZIL_STEELS {
        let mut m = Material::new(format!("Steel {grade}"), [157, 161, 165]);
        template_properties(&mut m, MaterialClass::Metal);
        m.identity.description = format!("Structural steel {grade} (fy = {fy} MPa)");
        m.physical.density = Some(7850.0);
        m.physical.youngs_modulus = Some(200_000.0);
        m.physical.poisson = Some(0.3);
        m.physical.thermal_expansion = Some(1.2e-5);
        m.physical.tensile_strength = Some(fy);
        m.physical.compressive_strength = Some(fy);
        list.push(m);
    }
    for &(grade, fyk) in BRAZIL_REBAR_STEELS {
        let mut m = Material::new(format!("Rebar {grade} (NBR 7480)"), [107, 110, 114]);
        template_properties(&mut m, MaterialClass::Metal);
        m.identity.description = format!("Reinforcing steel {grade} (fyk = {fyk} MPa)");
        m.physical.density = Some(7850.0);
        m.physical.youngs_modulus = Some(210_000.0);
        m.physical.poisson = Some(0.3);
        m.physical.thermal_expansion = Some(1.0e-5);
        m.physical.tensile_strength = Some(fyk);
        m.physical.compressive_strength = Some(fyk);
        list.push(m);
    }
    for &c in BRAZIL_CONCRETE_CLASSES {
        let fck = f64::from(c);
        let (_, ecs) = concrete_modulus_nbr6118(fck);
        let ecs_rounded = (ecs / 100.0).round() * 100.0;
        let mut m = Material::new(format!("Concrete C{c} (NBR 6118)"), [168, 168, 168]);
        template_properties(&mut m, MaterialClass::Concrete);
        m.identity.description = format!("Concrete class C{c} per NBR 8953 / NBR 6118 (fck = {c} MPa)");
        m.physical.density = Some(2500.0);
        m.physical.youngs_modulus = Some(ecs_rounded);
        m.physical.poisson = Some(0.2);
        m.physical.thermal_expansion = Some(1.0e-5);
        m.physical.compressive_strength = Some(fck);
        list.push(m);
    }
    list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_files_load_and_repair_fixes_numbers() {
        // A version 3 material has only a name and a color.
        let m: Material = ron::from_str("(name: \"Concreto\", color: (170, 170, 165))").unwrap();
        assert_eq!(m.appearance.color, [200, 200, 200]);
        assert_eq!(m.graphics.transparency, 0.0);
        let mut m = Material::new("X", [1, 2, 3]);
        assert_eq!(m.appearance.color, [1, 2, 3]);
        m.graphics.transparency = f64::NAN;
        m.physical.density = Some(f64::INFINITY);
        m.identity.cost = Some(f64::NAN);
        assert_eq!(m.repair(), 3);
        assert_eq!(m.graphics.transparency, 0.0);
        assert_eq!(m.physical.density, None);
    }

    #[test]
    fn textures_map_and_repair() {
        let png = b"\x89PNG\r\n\x1a\n".to_vec();
        let image =
            crate::raster::RasterImage { format: crate::raster::RasterFormat::Png, width: 2, height: 2, data: png };
        let mut t = MaterialTexture::new(image, 2.0, 0.5);
        assert_eq!(t.map([1.0, 1.0]), [0.5, 2.0]);
        t.offset = [1.0, 0.0];
        t.rotation_deg = 90.0;
        let [u, v] = t.map([1.0, 1.0]);
        // (0, 1) turned back a quarter: (1, 0), then scaled.
        assert!((u - 0.5).abs() < 1e-9 && v.abs() < 1e-9, "{u} {v}");
        let mut m = Material::new("X", [1, 2, 3]);
        t.width = f64::NAN;
        t.offset[1] = f64::INFINITY;
        m.appearance.texture = Some(t.clone());
        assert_eq!(m.repair(), 2);
        assert_eq!(m.appearance.texture.as_ref().map(|t| t.width), Some(1.0));
        // An image that is not what it says is dropped.
        let mut bad = (*t.image).clone();
        bad.data = b"not a picture".to_vec();
        m.appearance.texture = Some(MaterialTexture { image: std::sync::Arc::new(bad), ..t });
        assert_eq!(m.repair(), 1);
        assert!(m.appearance.texture.is_none());
    }

    #[test]
    fn shading_can_follow_the_appearance() {
        let mut m = Material::new("X", [10, 20, 30]);
        m.appearance.color = [200, 0, 0];
        assert_eq!(m.shading_color(), [10, 20, 30]);
        m.graphics.use_appearance = true;
        assert_eq!(m.shading_color(), [200, 0, 0]);
    }

    #[test]
    fn brazil_structural_materials_have_valid_mechanical_properties() {
        let mats = brazil_structural_materials();
        assert_eq!(mats.len(), BRAZIL_STEELS.len() + BRAZIL_REBAR_STEELS.len() + BRAZIL_CONCRETE_CLASSES.len());
        let mr250 = mats.iter().find(|m| m.name == "Steel MR250 (NBR 7007)").unwrap();
        assert_eq!(mr250.identity.class, MaterialClass::Metal);
        assert_eq!(mr250.physical.density, Some(7850.0));
        assert_eq!(mr250.physical.youngs_modulus, Some(200_000.0));
        assert_eq!(mr250.physical.tensile_strength, Some(250.0));
        let ca50 = mats.iter().find(|m| m.name == "Rebar CA-50 (NBR 7480)").unwrap();
        assert_eq!(ca50.physical.youngs_modulus, Some(210_000.0));
        assert_eq!(ca50.physical.tensile_strength, Some(500.0));
        let c30 = mats.iter().find(|m| m.name == "Concrete C30 (NBR 6118)").unwrap();
        assert_eq!(c30.identity.class, MaterialClass::Concrete);
        assert_eq!(c30.physical.compressive_strength, Some(30.0));
        assert_eq!(c30.physical.density, Some(2500.0));
        let (eci, ecs) = concrete_modulus_nbr6118(30.0);
        assert!(eci > 30_000.0 && ecs > 26_000.0);
        let (_, ecs80) = concrete_modulus_nbr6118(80.0);
        assert!(ecs80 > 40_000.0);
    }
}
