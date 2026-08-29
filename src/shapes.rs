//! Las 23 formas de Paint, los 9 pinceles y los dos rasterizadores.
//!
//! Truco central: **casi toda forma es una lista de puntos en coordenadas
//! normalizadas** (0..1 dentro de su caja). Eso da tres cosas gratis:
//!
//! 1. Agregar una forma es agregar una fila a una tabla.
//! 2. La misma lista de puntos dibuja el icono en la galería *y* la forma en el
//!    lienzo, así que no hay ni un archivo de iconos en el proyecto.
//! 3. Sólo hay dos rasterizadores —contorno y relleno— en vez de 23.

use crate::canvas::Canvas;
use ecolor::Color32;

pub type Pt = (f32, f32);

// ---------------------------------------------------------------- las formas

/// En el orden exacto de la galería de Paint de Windows 10.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Line,
    Curve,
    Oval,
    Rectangle,
    RoundedRect,
    Polygon,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    Star4,
    Star5,
    Star6,
    CalloutRounded,
    CalloutOval,
    CalloutCloud,
    Heart,
    Lightning,
    Heptagon,
    Octagon,
    Trapezoid,
    Parallelogram,
    Cross,
    L,
    Chevron,
    ArrowNE,
    ArrowLeftRight,
    ArrowUpDown,
    ArrowFour,
    Star8,
    Star12,
    Sun,
    Burst,
    Moon,
    Drop,
    Gear,
    Shield,
    House,
    Bookmark,
    Tag,
    Semicircle,
    Sector,
    Nonagon,
    Decagon,
    Dodecagon,
    Kite,
    Rhomboid,
    TrapezoidR,
    ArrowBent,
    ArrowCurvedU,
    ArrowPentagon,
    CalloutSquare,
    Cross2,
    Frame,
    Chevron2,
    Zigzag,
    Pill,
    Egg,
    Leaf,
    Star3,
    Star7,
    Star10,
    Star24,
    BurstSharp,
    Cube,
    Plaque,
    Bevel,
    Diamond2,
}

pub const ALL_SHAPES: [Shape; 73] = [
    Shape::Line,
    Shape::Curve,
    Shape::Oval,
    Shape::Rectangle,
    Shape::RoundedRect,
    Shape::Polygon,
    Shape::Triangle,
    Shape::RightTriangle,
    Shape::Diamond,
    Shape::Pentagon,
    Shape::Hexagon,
    Shape::ArrowRight,
    Shape::ArrowLeft,
    Shape::ArrowUp,
    Shape::ArrowDown,
    Shape::Star4,
    Shape::Star5,
    Shape::Star6,
    Shape::CalloutRounded,
    Shape::CalloutOval,
    Shape::CalloutCloud,
    Shape::Heart,
    Shape::Lightning,
    Shape::Heptagon,
    Shape::Octagon,
    Shape::Trapezoid,
    Shape::Parallelogram,
    Shape::Cross,
    Shape::L,
    Shape::Chevron,
    Shape::ArrowNE,
    Shape::ArrowLeftRight,
    Shape::ArrowUpDown,
    Shape::ArrowFour,
    Shape::Star8,
    Shape::Star12,
    Shape::Sun,
    Shape::Burst,
    Shape::Moon,
    Shape::Drop,
    Shape::Gear,
    Shape::Shield,
    Shape::House,
    Shape::Bookmark,
    Shape::Tag,
    Shape::Semicircle,
    Shape::Sector,
    Shape::Nonagon,
    Shape::Decagon,
    Shape::Dodecagon,
    Shape::Kite,
    Shape::Rhomboid,
    Shape::TrapezoidR,
    Shape::ArrowBent,
    Shape::ArrowCurvedU,
    Shape::ArrowPentagon,
    Shape::CalloutSquare,
    Shape::Cross2,
    Shape::Frame,
    Shape::Chevron2,
    Shape::Zigzag,
    Shape::Pill,
    Shape::Egg,
    Shape::Leaf,
    Shape::Star3,
    Shape::Star7,
    Shape::Star10,
    Shape::Star24,
    Shape::BurstSharp,
    Shape::Cube,
    Shape::Plaque,
    Shape::Bevel,
    Shape::Diamond2,
];

impl Shape {
    /// El nombre que ve el usuario, ya traducido.
    ///
    /// La traducción va **acá adentro** y no en cada sitio que lo dibuja: son
    /// trece llamadas repartidas en dos archivos, y con envolverlas una por una
    /// la que se agregue mañana se olvida. `lang` es una tabla de texto de la
    /// biblioteca estándar, así que el motor sigue sin saber que egui existe.
    pub fn label(self) -> &'static str {
        crate::lang::t(match self {
            Self::Line => "Línea",
            Self::Curve => "Curva",
            Self::Oval => "Óvalo",
            Self::Rectangle => "Rectángulo",
            Self::RoundedRect => "Rectángulo redondeado",
            Self::Polygon => "Polígono",
            Self::Triangle => "Triángulo",
            Self::RightTriangle => "Triángulo rectángulo",
            Self::Diamond => "Rombo",
            Self::Pentagon => "Pentágono",
            Self::Hexagon => "Hexágono",
            Self::ArrowRight => "Flecha derecha",
            Self::ArrowLeft => "Flecha izquierda",
            Self::ArrowUp => "Flecha arriba",
            Self::ArrowDown => "Flecha abajo",
            Self::Star4 => "Estrella de 4 puntas",
            Self::Star5 => "Estrella de 5 puntas",
            Self::Star6 => "Estrella de 6 puntas",
            Self::CalloutRounded => "Llamada rectangular redondeada",
            Self::CalloutOval => "Llamada ovalada",
            Self::CalloutCloud => "Llamada de nube",
            Self::Heart => "Corazón",
            Self::Lightning => "Rayo",
            Self::Heptagon => "Heptágono",
            Self::Octagon => "Octágono",
            Self::Trapezoid => "Trapecio",
            Self::Parallelogram => "Paralelogramo",
            Self::Cross => "Cruz",
            Self::L => "Ele",
            Self::Chevron => "Galón",
            Self::ArrowNE => "Flecha diagonal",
            Self::ArrowLeftRight => "Flecha doble horizontal",
            Self::ArrowUpDown => "Flecha doble vertical",
            Self::ArrowFour => "Flecha de cuatro puntas",
            Self::Star8 => "Estrella de 8 puntas",
            Self::Star12 => "Estrella de 12 puntas",
            Self::Sun => "Sol",
            Self::Burst => "Explosión",
            Self::Moon => "Luna",
            Self::Drop => "Gota",
            Self::Gear => "Engranaje",
            Self::Shield => "Escudo",
            Self::House => "Casa",
            Self::Bookmark => "Marcador",
            Self::Tag => "Etiqueta",
            Self::Semicircle => "Semicírculo",
            Self::Sector => "Sector circular",
            Self::Nonagon => "Eneágono",
            Self::Decagon => "Decágono",
            Self::Dodecagon => "Dodecágono",
            Self::Kite => "Cometa",
            Self::Rhomboid => "Romboide",
            Self::TrapezoidR => "Trapecio acostado",
            Self::ArrowBent => "Flecha doblada",
            Self::ArrowCurvedU => "Flecha en U",
            Self::ArrowPentagon => "Flecha pentagonal",
            Self::CalloutSquare => "Llamada rectangular",
            Self::Cross2 => "Cruz delgada",
            Self::Frame => "Marco",
            Self::Chevron2 => "Galón delgado",
            Self::Zigzag => "Zigzag",
            Self::Pill => "Cápsula",
            Self::Egg => "Huevo",
            Self::Leaf => "Hoja",
            Self::Star3 => "Estrella de 3 puntas",
            Self::Star7 => "Estrella de 7 puntas",
            Self::Star10 => "Estrella de 10 puntas",
            Self::Star24 => "Sol de 24 rayos",
            Self::BurstSharp => "Explosión aguda",
            Self::Cube => "Cubo",
            Self::Plaque => "Placa",
            Self::Bevel => "Bisel",
            Self::Diamond2 => "Rombo alargado",
        })
    }

    /// Línea y curva son abiertas: no se rellenan.
    pub fn is_closed(self) -> bool {
        !matches!(self, Self::Line | Self::Curve)
    }

    /// La forma en coordenadas unitarias (0..1 dentro de su caja).
    /// Todo vive en este espacio; `points` hace el único mapeo a píxeles.
    fn unit(self) -> Vec<Pt> {
        let table = |t: &[Pt]| t.to_vec();
        match self {
            // Diagonal de la caja: así arrastrar da la línea que uno espera.
            Self::Line | Self::Curve => vec![(0.0, 0.0), (1.0, 1.0)],
            Self::Rectangle | Self::Polygon => table(&UNIT_RECT),
            Self::Oval => ellipse(32),
            Self::RoundedRect => rounded_rect(0.18),
            Self::Triangle => table(&UNIT_TRIANGLE),
            Self::RightTriangle => table(&UNIT_RIGHT_TRIANGLE),
            Self::Diamond => table(&UNIT_DIAMOND),
            Self::Pentagon => regular(5),
            Self::Hexagon => regular(6),
            Self::ArrowRight => table(&UNIT_ARROW_R),
            Self::ArrowLeft => UNIT_ARROW_R.iter().map(|p| (1.0 - p.0, p.1)).collect(),
            Self::ArrowUp => UNIT_ARROW_R.iter().map(|p| (p.1, 1.0 - p.0)).collect(),
            Self::ArrowDown => UNIT_ARROW_R.iter().map(|p| (p.1, p.0)).collect(),
            Self::Star4 => star(4, 0.38),
            Self::Star5 => star(5, 0.382),
            Self::Star6 => star(6, 0.577),
            Self::CalloutRounded => table(&UNIT_CALLOUT_RECT),
            Self::CalloutOval => callout_oval(),
            Self::CalloutCloud => table(&UNIT_CLOUD),
            Self::Heart => heart(),
            Self::Lightning => table(&UNIT_LIGHTNING),
            Self::Heptagon => regular(7),
            Self::Octagon => regular(8),
            Self::Trapezoid => table(&UNIT_TRAPEZOID),
            Self::Parallelogram => table(&UNIT_PARALLELOGRAM),
            Self::Cross => table(&UNIT_CROSS),
            Self::L => table(&UNIT_L),
            Self::Chevron => table(&UNIT_CHEVRON),
            Self::ArrowNE => table(&UNIT_ARROW_NE),
            Self::ArrowLeftRight => table(&UNIT_ARROW_LR),
            // La vertical es la horizontal con los ejes cambiados.
            Self::ArrowUpDown => UNIT_ARROW_LR.iter().map(|p| (p.1, p.0)).collect(),
            Self::ArrowFour => table(&UNIT_ARROW_4),
            Self::Star8 => star(8, 0.52),
            Self::Star12 => star(12, 0.68),
            Self::Sun => star(16, 0.66),
            Self::Burst => star(10, 0.44),
            Self::Moon => moon(),
            Self::Drop => drop(),
            Self::Gear => gear(9, 0.34, 0.5),
            Self::Shield => table(&UNIT_SHIELD),
            Self::House => table(&UNIT_HOUSE),
            Self::Bookmark => table(&UNIT_BOOKMARK),
            Self::Tag => table(&UNIT_TAG),
            Self::Semicircle => semicircle(),
            Self::Sector => sector(),
            Self::Nonagon => regular(9),
            Self::Decagon => regular(10),
            Self::Dodecagon => regular(12),
            Self::Kite => table(&UNIT_KITE),
            Self::Rhomboid => table(&UNIT_RHOMBOID),
            Self::TrapezoidR => table(&UNIT_TRAPEZOID_R),
            Self::ArrowBent => table(&UNIT_ARROW_BENT),
            Self::ArrowCurvedU => table(&UNIT_ARROW_U),
            Self::ArrowPentagon => table(&UNIT_ARROW_PENTA),
            Self::CalloutSquare => table(&UNIT_CALLOUT_SQ),
            Self::Cross2 => table(&UNIT_CROSS_THIN),
            Self::Frame => table(&UNIT_FRAME),
            Self::Chevron2 => table(&UNIT_CHEVRON_THIN),
            Self::Zigzag => table(&UNIT_ZIGZAG),
            Self::Pill => pill(),
            Self::Egg => egg(),
            Self::Leaf => leaf(),
            Self::Star3 => star(3, 0.30),
            Self::Star7 => star(7, 0.55),
            Self::Star10 => star(10, 0.62),
            Self::Star24 => star(24, 0.80),
            Self::BurstSharp => star(8, 0.30),
            Self::Cube => table(&UNIT_CUBE),
            Self::Plaque => table(&UNIT_PLAQUE),
            Self::Bevel => table(&UNIT_BEVEL),
            Self::Diamond2 => table(&UNIT_DIAMOND_LONG),
        }
    }

    /// Los puntos de la forma dentro de la caja `(x0,y0)-(x1,y1)`, en píxeles.
    /// La misma llamada sirve para el icono de la galería y para el lienzo.
    pub fn points(self, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Pt> {
        let (w, h) = (x1 - x0, y1 - y0);
        self.unit()
            .into_iter()
            .map(|p| (x0 + p.0 * w, y0 + p.1 * h))
            .collect()
    }
}

// Tablas en coordenadas normalizadas (0..1). Agregar una forma es agregar una.
const UNIT_RECT: [Pt; 4] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
const UNIT_TRIANGLE: [Pt; 3] = [(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)];
const UNIT_RIGHT_TRIANGLE: [Pt; 3] = [(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
const UNIT_DIAMOND: [Pt; 4] = [(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)];

const UNIT_ARROW_R: [Pt; 7] = [
    (0.0, 0.3),
    (0.6, 0.3),
    (0.6, 0.05),
    (1.0, 0.5),
    (0.6, 0.95),
    (0.6, 0.7),
    (0.0, 0.7),
];

const UNIT_CALLOUT_RECT: [Pt; 11] = [
    (0.06, 0.0),
    (0.94, 0.0),
    (1.0, 0.06),
    (1.0, 0.64),
    (0.94, 0.70),
    (0.42, 0.70),
    (0.22, 1.0),
    (0.26, 0.70),
    (0.06, 0.70),
    (0.0, 0.64),
    (0.0, 0.06),
];

/// Nube recorrida en orden, con la cola intercalada donde corresponde.
/// Antes se generaba con seis lóbulos ordenados por ángulo, pero la mitad de
/// los puntos caían *dentro* de la silueta y el contorno los visitaba todos:
/// salía un sol, no una nube. Una tabla es más corta y se ve.
const UNIT_CLOUD: [Pt; 27] = [
    (0.12, 0.62),
    (0.06, 0.52),
    (0.09, 0.40),
    (0.19, 0.33),
    (0.21, 0.21),
    (0.32, 0.14),
    (0.43, 0.18),
    (0.49, 0.09),
    (0.61, 0.08),
    (0.68, 0.16),
    (0.78, 0.13),
    (0.88, 0.22),
    (0.87, 0.34),
    (0.95, 0.41),
    (0.96, 0.53),
    (0.88, 0.60),
    (0.85, 0.69),
    (0.74, 0.73),
    (0.65, 0.69),
    (0.55, 0.75),
    (0.44, 0.74),
    (0.38, 0.68),
    // la cola
    (0.31, 0.73),
    (0.20, 1.00),
    (0.28, 0.69),
    (0.19, 0.68),
    (0.14, 0.66),
];

const UNIT_LIGHTNING: [Pt; 7] = [
    (0.52, 0.0),
    (0.16, 0.55),
    (0.42, 0.55),
    (0.28, 1.0),
    (0.84, 0.38),
    (0.54, 0.38),
    (0.78, 0.0),
];

const UNIT_TRAPEZOID: [Pt; 4] = [(0.25, 0.0), (0.75, 0.0), (1.0, 1.0), (0.0, 1.0)];
const UNIT_PARALLELOGRAM: [Pt; 4] = [(0.25, 0.0), (1.0, 0.0), (0.75, 1.0), (0.0, 1.0)];

const UNIT_CROSS: [Pt; 12] = [
    (0.34, 0.0),
    (0.66, 0.0),
    (0.66, 0.34),
    (1.0, 0.34),
    (1.0, 0.66),
    (0.66, 0.66),
    (0.66, 1.0),
    (0.34, 1.0),
    (0.34, 0.66),
    (0.0, 0.66),
    (0.0, 0.34),
    (0.34, 0.34),
];

const UNIT_L: [Pt; 6] = [
    (0.0, 0.0),
    (0.36, 0.0),
    (0.36, 0.64),
    (1.0, 0.64),
    (1.0, 1.0),
    (0.0, 1.0),
];

const UNIT_CHEVRON: [Pt; 6] = [
    (0.0, 0.0),
    (0.45, 0.0),
    (1.0, 0.5),
    (0.45, 1.0),
    (0.0, 1.0),
    (0.55, 0.5),
];

/// Flecha a 45°. No es la de la derecha rotada: rotar una caja cuadrada 45°
/// la saca de su propia caja, así que la diagonal se dibuja directo.
const UNIT_ARROW_NE: [Pt; 7] = [
    (0.0, 0.86),
    (0.52, 0.34),
    (0.38, 0.20),
    (0.86, 0.14),
    (0.80, 0.62),
    (0.66, 0.48),
    (0.14, 1.0),
];

const UNIT_ARROW_LR: [Pt; 10] = [
    (0.0, 0.5),
    (0.25, 0.15),
    (0.25, 0.35),
    (0.75, 0.35),
    (0.75, 0.15),
    (1.0, 0.5),
    (0.75, 0.85),
    (0.75, 0.65),
    (0.25, 0.65),
    (0.25, 0.85),
];

const UNIT_ARROW_4: [Pt; 24] = [
    (0.5, 0.0),
    (0.72, 0.22),
    (0.6, 0.22),
    (0.6, 0.4),
    (0.78, 0.4),
    (0.78, 0.28),
    (1.0, 0.5),
    (0.78, 0.72),
    (0.78, 0.6),
    (0.6, 0.6),
    (0.6, 0.78),
    (0.72, 0.78),
    (0.5, 1.0),
    (0.28, 0.78),
    (0.4, 0.78),
    (0.4, 0.6),
    (0.22, 0.6),
    (0.22, 0.72),
    (0.0, 0.5),
    (0.22, 0.28),
    (0.22, 0.4),
    (0.4, 0.4),
    (0.4, 0.22),
    (0.28, 0.22),
];

const UNIT_SHIELD: [Pt; 10] = [
    (0.03, 0.05),
    (0.5, 0.0),
    (0.97, 0.05),
    (0.96, 0.40),
    (0.90, 0.66),
    (0.76, 0.86),
    (0.5, 1.0),
    (0.24, 0.86),
    (0.10, 0.66),
    (0.04, 0.40),
];

const UNIT_HOUSE: [Pt; 7] = [
    (0.5, 0.0),
    (1.0, 0.42),
    (0.84, 0.42),
    (0.84, 1.0),
    (0.16, 1.0),
    (0.16, 0.42),
    (0.0, 0.42),
];

const UNIT_BOOKMARK: [Pt; 5] = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.5, 0.74), (0.0, 1.0)];

const UNIT_TAG: [Pt; 5] = [
    (0.0, 0.5),
    (0.28, 0.08),
    (1.0, 0.08),
    (1.0, 0.92),
    (0.28, 0.92),
];

/// Engranaje de `n` dientes. El ángulo crece siempre y sólo cambia el radio,
/// así que sale un polígono en estrella: no puede cruzarse a sí mismo.
fn gear(n: usize, r_in: f32, r_out: f32) -> Vec<Pt> {
    let step = std::f32::consts::TAU / n as f32;
    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        let s = i as f32 * step;
        for (f, r) in [(0.0, r_in), (0.18, r_out), (0.50, r_out), (0.68, r_in)] {
            let a = s + step * f;
            out.push((0.5 + r * a.cos(), 0.5 + r * a.sin()));
        }
    }
    out
}

/// Luna: media circunferencia por fuera y otra más angosta de vuelta. Los dos
/// arcos comparten los polos, así que el de adentro no repite esos vértices —
/// dos puntos idénticos seguidos dejan un segmento de largo cero.
fn moon() -> Vec<Pt> {
    use std::f32::consts::PI;
    const N: usize = 20;
    let mut out = Vec::with_capacity(2 * N);
    for i in 0..=N {
        let a = PI / 2.0 + i as f32 / N as f32 * PI;
        out.push((0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()));
    }
    for i in 1..N {
        let a = PI * 1.5 - i as f32 / N as f32 * PI;
        out.push((0.5 + 0.28 * a.cos(), 0.5 + 0.5 * a.sin()));
    }
    out
}

/// Gota: la punta arriba y un arco de 280° abajo. Los dos tramos rectos que
/// suben a la punta son las tangentes aproximadas del círculo.
fn drop() -> Vec<Pt> {
    const N: usize = 22;
    let (a0, a1) = ((-50.0_f32).to_radians(), 230.0_f32.to_radians());
    let mut out = vec![(0.5, 0.0)];
    for i in 0..=N {
        let a = a0 + (a1 - a0) * i as f32 / N as f32;
        out.push((0.5 + 0.34 * a.cos(), 0.66 + 0.34 * a.sin()));
    }
    out
}

/// Semicírculo: cúpula arriba y base plana. El radio vertical es 1.0, no 0.5,
/// para que llene la caja en vez de quedar en la mitad de arriba.
fn semicircle() -> Vec<Pt> {
    use std::f32::consts::PI;
    const N: usize = 20;
    (0..=N)
        .map(|i| {
            let a = PI + i as f32 / N as f32 * PI;
            (0.5 + 0.5 * a.cos(), 1.0 + a.sin())
        })
        .collect()
}

/// Porción de círculo: el centro y un arco de 280°.
fn sector() -> Vec<Pt> {
    const N: usize = 24;
    let (a0, a1) = (40.0_f32.to_radians(), 320.0_f32.to_radians());
    let mut out = vec![(0.5, 0.5)];
    for i in 0..=N {
        let a = a0 + (a1 - a0) * i as f32 / N as f32;
        out.push((0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin()));
    }
    out
}

const UNIT_KITE: [Pt; 4] = [(0.5, 0.0), (0.92, 0.36), (0.5, 1.0), (0.08, 0.36)];
const UNIT_RHOMBOID: [Pt; 4] = [(0.0, 0.22), (0.75, 0.0), (1.0, 0.78), (0.25, 1.0)];
const UNIT_TRAPEZOID_R: [Pt; 4] = [(0.0, 0.0), (1.0, 0.22), (1.0, 0.78), (0.0, 1.0)];

const UNIT_ARROW_BENT: [Pt; 10] = [
    (0.0, 0.62),
    (0.0, 0.38),
    (0.58, 0.38),
    (0.58, 0.14),
    (1.0, 0.44),
    (0.58, 0.74),
    (0.58, 0.50),
    (0.22, 0.50),
    (0.22, 1.0),
    (0.0, 1.0),
];

const UNIT_ARROW_U: [Pt; 11] = [
    (0.06, 1.0),
    (0.06, 0.46),
    (0.30, 0.46),
    (0.30, 0.72),
    (0.62, 0.72),
    (0.62, 0.46),
    (0.44, 0.46),
    (0.72, 0.10),
    (1.0, 0.46),
    (0.82, 0.46),
    (0.82, 1.0),
];

const UNIT_ARROW_PENTA: [Pt; 5] = [
    (0.0, 0.16),
    (0.66, 0.16),
    (1.0, 0.5),
    (0.66, 0.84),
    (0.0, 0.84),
];

const UNIT_CALLOUT_SQ: [Pt; 7] = [
    (0.0, 0.0),
    (1.0, 0.0),
    (1.0, 0.70),
    (0.44, 0.70),
    (0.20, 1.0),
    (0.24, 0.70),
    (0.0, 0.70),
];

const UNIT_CROSS_THIN: [Pt; 12] = [
    (0.42, 0.0),
    (0.58, 0.0),
    (0.58, 0.42),
    (1.0, 0.42),
    (1.0, 0.58),
    (0.58, 0.58),
    (0.58, 1.0),
    (0.42, 1.0),
    (0.42, 0.58),
    (0.0, 0.58),
    (0.0, 0.42),
    (0.42, 0.42),
];

/// Marco: el borde exterior y el hueco recorridos en el mismo trazo, unidos por
/// el lado de arriba. Un polígono no tiene agujeros, así que el hueco es parte
/// del contorno.
const UNIT_FRAME: [Pt; 8] = [
    (0.0, 0.0),
    (1.0, 0.0),
    (0.86, 0.16),
    (0.14, 0.16),
    (0.14, 0.84),
    (0.86, 0.84),
    (1.0, 1.0),
    (0.0, 1.0),
];

const UNIT_CHEVRON_THIN: [Pt; 6] = [
    (0.0, 0.0),
    (0.30, 0.0),
    (0.62, 0.5),
    (0.30, 1.0),
    (0.0, 1.0),
    (0.32, 0.5),
];

const UNIT_ZIGZAG: [Pt; 10] = [
    (0.0, 0.28),
    (0.25, 0.0),
    (0.50, 0.28),
    (0.75, 0.0),
    (1.0, 0.28),
    (1.0, 0.56),
    (0.75, 0.28),
    (0.50, 0.56),
    (0.25, 0.28),
    (0.0, 0.56),
];

const UNIT_CUBE: [Pt; 6] = [
    (0.0, 0.28),
    (0.30, 0.0),
    (1.0, 0.0),
    (1.0, 0.72),
    (0.70, 1.0),
    (0.0, 1.0),
];

const UNIT_PLAQUE: [Pt; 8] = [
    (0.14, 0.0),
    (0.86, 0.0),
    (1.0, 0.14),
    (1.0, 0.86),
    (0.86, 1.0),
    (0.14, 1.0),
    (0.0, 0.86),
    (0.0, 0.14),
];

const UNIT_BEVEL: [Pt; 8] = [
    (0.18, 0.0),
    (0.82, 0.0),
    (1.0, 0.18),
    (1.0, 0.82),
    (0.82, 1.0),
    (0.18, 1.0),
    (0.0, 0.82),
    (0.0, 0.18),
];

const UNIT_DIAMOND_LONG: [Pt; 6] = [
    (0.5, 0.0),
    (1.0, 0.30),
    (1.0, 0.70),
    (0.5, 1.0),
    (0.0, 0.70),
    (0.0, 0.30),
];

/// Un arco de elipse, de `a0` a `a1` en `n` pasos. Los extremos entran los dos:
/// quien lo encadene con otro tramo tiene que sacar el que se repite.
fn arc(cx: f32, cy: f32, rx: f32, ry: f32, a0: f32, a1: f32, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|i| {
            let a = a0 + (a1 - a0) * i as f32 / n as f32;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

/// Cápsula: dos medias circunferencias y los lados rectos que salen solos al
/// unirlas.
fn pill() -> Vec<Pt> {
    use std::f32::consts::FRAC_PI_2;
    let mut out = arc(0.75, 0.5, 0.25, 0.5, -FRAC_PI_2, FRAC_PI_2, 14);
    out.extend(arc(0.25, 0.5, 0.25, 0.5, FRAC_PI_2, FRAC_PI_2 * 3.0, 14));
    out
}

/// Huevo: la punta arriba y un óvalo abajo. Es la gota con la punta más roma.
fn egg() -> Vec<Pt> {
    let mut out = vec![(0.5, 0.0)];
    out.extend(arc(
        0.5,
        0.60,
        0.40,
        0.40,
        (-62.0_f32).to_radians(),
        242.0_f32.to_radians(),
        26,
    ));
    out
}

/// Hoja: dos arcos opuestos, cada uno centrado en una esquina de la caja. El
/// último punto de cada tramo es el primero del otro, así que se descarta —dos
/// vértices idénticos seguidos dejan un segmento de largo cero.
fn leaf() -> Vec<Pt> {
    use std::f32::consts::{FRAC_PI_2, PI};
    let mut a = arc(0.0, 1.0, 1.0, 1.0, -FRAC_PI_2, 0.0, 18);
    a.pop();
    let mut b = arc(1.0, 0.0, 1.0, 1.0, FRAC_PI_2, PI, 18);
    b.pop();
    a.extend(b);
    a
}

fn ellipse(n: usize) -> Vec<Pt> {
    (0..n)
        .map(|i| {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            (0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin())
        })
        .collect()
}

/// Polígono regular de `n` lados, con una punta hacia arriba.
fn regular(n: usize) -> Vec<Pt> {
    (0..n)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 / n as f32 * std::f32::consts::TAU;
            (0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin())
        })
        .collect()
}

/// Estrella de `n` puntas. `inner` es el radio del valle como fracción del de
/// la punta: 0.382 es el de la estrella de cinco puntas clásica.
fn star(n: usize, inner: f32) -> Vec<Pt> {
    (0..n * 2)
        .map(|i| {
            let k = if i % 2 == 0 { 0.5 } else { 0.5 * inner };
            let a =
                -std::f32::consts::FRAC_PI_2 + i as f32 / (n * 2) as f32 * std::f32::consts::TAU;
            (0.5 + k * a.cos(), 0.5 + k * a.sin())
        })
        .collect()
}

fn rounded_rect(r: f32) -> Vec<Pt> {
    let mut out = Vec::with_capacity(4 * 7);
    // esquinas: centro y ángulo inicial
    let corners = [
        ((1.0 - r, 1.0 - r), 0.0_f32),
        ((r, 1.0 - r), std::f32::consts::FRAC_PI_2),
        ((r, r), std::f32::consts::PI),
        ((1.0 - r, r), std::f32::consts::PI * 1.5),
    ];
    for ((cx, cy), a0) in corners {
        for i in 0..=6 {
            let a = a0 + i as f32 / 6.0 * std::f32::consts::FRAC_PI_2;
            out.push((cx + r * a.cos(), cy + r * a.sin()));
        }
    }
    out
}

fn callout_oval() -> Vec<Pt> {
    let mut out: Vec<Pt> = (0..28)
        .map(|i| {
            let a = i as f32 / 28.0 * std::f32::consts::TAU;
            (0.5 + 0.5 * a.cos(), 0.35 + 0.35 * a.sin())
        })
        .collect();
    // La cola va *intercalada* en el recorrido, no agregada al final: en un
    // polígono el orden es la forma, y empujarla al final cruzaría dos cuerdas
    // por el medio del globo.
    out.splice(9..9, [(0.44, 0.70), (0.22, 1.0), (0.30, 0.68)]);
    out
}

fn heart() -> Vec<Pt> {
    (0..40)
        .map(|i| {
            let t = i as f32 / 40.0 * std::f32::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y =
                13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            // Muestreando 40 puntos: x ∈ [-16, 16], y ∈ [-17.0, 11.9]. La punta
            // de abajo cae justo en t = π, así que con divisores redondos se
            // salía de la caja un 6% y el test lo cazaba.
            (0.5 + x / 32.0, (11.9 - y) / 28.9)
        })
        .collect()
}

/// Bézier cuadrática, para la herramienta Curva.
pub fn quadratic(a: Pt, ctrl: Pt, b: Pt, n: usize) -> Vec<Pt> {
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            let u = 1.0 - t;
            (
                u * u * a.0 + 2.0 * u * t * ctrl.0 + t * t * b.0,
                u * u * a.1 + 2.0 * u * t * ctrl.1 + t * t * b.1,
            )
        })
        .collect()
}

// ------------------------------------------------------------ rasterizadores

/// Estampa un disco lleno. Es el nib de todo: lápiz, contornos, pinceles.
pub fn stamp(c: &mut Canvas, x: i32, y: i32, width: f32, col: Color32) {
    if width <= 1.0 {
        c.set(x, y, col);
        return;
    }
    let r = width / 2.0;
    let ri = r.ceil() as i32;
    let r2 = r * r;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if (dx * dx + dy * dy) as f32 <= r2 {
                c.set(x + dx, y + dy, col);
            }
        }
    }
}

/// Estampa un cuadrado. El borrador de Paint es cuadrado, no redondo.
pub fn stamp_square(c: &mut Canvas, x: i32, y: i32, width: f32, col: Color32) {
    let r = (width / 2.0).round() as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            c.set(x + dx, y + dy, col);
        }
    }
}

pub fn line(c: &mut Canvas, a: Pt, b: Pt, width: f32, col: Color32) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0);
    for i in 0..=steps as i32 {
        let t = i as f32 / steps;
        stamp(
            c,
            (a.0 + dx * t).round() as i32,
            (a.1 + dy * t).round() as i32,
            width,
            col,
        );
    }
}

pub fn stroke_polyline(c: &mut Canvas, pts: &[Pt], closed: bool, width: f32, col: Color32) {
    if pts.len() < 2 {
        if let Some(p) = pts.first() {
            stamp(c, p.0.round() as i32, p.1.round() as i32, width, col);
        }
        return;
    }
    for i in 0..pts.len() - 1 {
        line(c, pts[i], pts[i + 1], width, col);
    }
    if closed {
        line(c, pts[pts.len() - 1], pts[0], width, col);
    }
}

/// Relleno de polígono por líneas de barrido, regla par-impar.
pub fn fill_polygon(c: &mut Canvas, pts: &[Pt], col: Color32) {
    if pts.len() < 3 {
        return;
    }
    // Acotado al lienzo, no a la forma: arrastrar una figura muy afuera daría
    // un rango de miles de millones de filas y la aplicación se cuelga.
    let ymin = pts
        .iter()
        .fold(f32::MAX, |m, p| m.min(p.1))
        .floor()
        .max(0.0) as i32;
    let ymax = pts
        .iter()
        .fold(f32::MIN, |m, p| m.max(p.1))
        .ceil()
        .min(c.h as f32) as i32;
    let cw = c.w as i32;
    let mut xs: Vec<f32> = Vec::with_capacity(8);

    for y in ymin..=ymax {
        let yc = y as f32 + 0.5;
        xs.clear();
        for i in 0..pts.len() {
            let (x1, y1) = pts[i];
            let (x2, y2) = pts[(i + 1) % pts.len()];
            if (y1 <= yc) != (y2 <= yc) {
                let t = (yc - y1) / (y2 - y1);
                xs.push(x1 + t * (x2 - x1));
            }
        }
        xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        for pair in xs.chunks(2) {
            if let [a, b] = pair {
                let a = (a.round() as i32).max(0);
                let b = (b.round() as i32).min(cw);
                for x in a..b {
                    c.set(x, y, col);
                }
            }
        }
    }
}

// ------------------------------------------------------------------ pinceles

/// Los 9 pinceles de Paint, en el orden de la galería.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Brush {
    Round,
    Calligraphy1,
    Calligraphy2,
    Airbrush,
    Oil,
    Crayon,
    Marker,
    NaturalPencil,
    Watercolor,
}

pub const ALL_BRUSHES: [Brush; 9] = [
    Brush::Round,
    Brush::Calligraphy1,
    Brush::Calligraphy2,
    Brush::Airbrush,
    Brush::Oil,
    Brush::Crayon,
    Brush::Marker,
    Brush::NaturalPencil,
    Brush::Watercolor,
];

impl Brush {
    /// El nombre que ve el usuario, ya traducido.
    ///
    /// La traducción va **acá adentro** y no en cada sitio que lo dibuja: son
    /// trece llamadas repartidas en dos archivos, y con envolverlas una por una
    /// la que se agregue mañana se olvida. `lang` es una tabla de texto de la
    /// biblioteca estándar, así que el motor sigue sin saber que egui existe.
    pub fn label(self) -> &'static str {
        crate::lang::t(match self {
            Self::Round => "Pincel",
            Self::Calligraphy1 => "Caligrafía 1",
            Self::Calligraphy2 => "Caligrafía 2",
            Self::Airbrush => "Aerógrafo",
            Self::Oil => "Óleo",
            Self::Crayon => "Crayón",
            Self::Marker => "Marcador",
            Self::NaturalPencil => "Lápiz natural",
            Self::Watercolor => "Acuarela",
        })
    }
}

/// Generador pseudoaleatorio xorshift. Cinco líneas en vez de una dependencia.
pub struct Rng(u32);

/// Hace falta para `std::mem::take`, que es como se saca el generador de `Doc`
/// mientras se le presta el lienzo al pincel.
///
/// **No usar `#[derive(Default)]`**: daría `Rng(0)`, y cero es el estado muerto
/// de un xorshift — se queda en cero para siempre y los pinceles texturados
/// pierden el grano sin que nada avise.
impl Default for Rng {
    fn default() -> Self {
        Self::new()
    }
}

impl Rng {
    pub fn new() -> Self {
        Self(0x2545_F491)
    }

    #[inline]
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1 << 24) as f32
    }
}

/// Una pincelada de `a` a `b`. Cada pincel es una variación del mismo bucle:
/// cambia la forma del nib y la opacidad.
pub fn brush_stroke(
    c: &mut Canvas,
    brush: Brush,
    a: Pt,
    b: Pt,
    width: f32,
    col: Color32,
    rng: &mut Rng,
) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let steps = dx.abs().max(dy.abs()).ceil().max(1.0);
    let half = (width / 2.0).max(0.5);

    for i in 0..=steps as i32 {
        let t = i as f32 / steps;
        let x = (a.0 + dx * t).round() as i32;
        let y = (a.1 + dy * t).round() as i32;

        match brush {
            Brush::Round => stamp(c, x, y, width, col),

            // Nib de plumín: una línea inclinada, no un disco.
            Brush::Calligraphy1 | Brush::Calligraphy2 => {
                let sign = if brush == Brush::Calligraphy1 { 1 } else { -1 };
                let n = half.ceil() as i32;
                for k in -n..=n {
                    c.set(x + k, y + k * sign, col);
                    c.set(x + k, y + k * sign + 1, col);
                }
            }

            // Rocía puntos sueltos; cuanto más lento el trazo, más se acumula.
            Brush::Airbrush => {
                let r = width * 1.4;
                let n = (r * 0.8).max(3.0) as i32;
                for _ in 0..n {
                    let ang = rng.unit() * std::f32::consts::TAU;
                    let d = rng.unit().sqrt() * r;
                    c.set(x + (ang.cos() * d) as i32, y + (ang.sin() * d) as i32, col);
                }
            }

            // Grueso y texturado: disco lleno con mordidas irregulares.
            Brush::Oil => {
                let r = (width / 2.0).ceil() as i32;
                let r2 = (width / 2.0) * (width / 2.0);
                for ddy in -r..=r {
                    for ddx in -r..=r {
                        if (ddx * ddx + ddy * ddy) as f32 <= r2 && rng.unit() > 0.12 {
                            c.blend(x + ddx, y + ddy, col, 0.85);
                        }
                    }
                }
            }

            // Granulado y translúcido.
            Brush::Crayon => {
                let r = (width / 2.0).ceil() as i32;
                let r2 = (width / 2.0) * (width / 2.0);
                for ddy in -r..=r {
                    for ddx in -r..=r {
                        if (ddx * ddx + ddy * ddy) as f32 <= r2 && rng.unit() > 0.45 {
                            c.blend(x + ddx, y + ddy, col, 0.55);
                        }
                    }
                }
            }

            // Capa translúcida pareja: se multiplica al pasar de nuevo.
            Brush::Marker => {
                let r = (width / 2.0).ceil() as i32;
                let r2 = (width / 2.0) * (width / 2.0);
                for ddy in -r..=r {
                    for ddx in -r..=r {
                        if (ddx * ddx + ddy * ddy) as f32 <= r2 {
                            c.blend(x + ddx, y + ddy, col, 0.35);
                        }
                    }
                }
            }

            // Trazo liviano, como bosquejar.
            Brush::NaturalPencil => {
                let r = (width / 3.0).ceil().max(1.0) as i32;
                for ddy in -r..=r {
                    for ddx in -r..=r {
                        if rng.unit() > 0.55 {
                            c.blend(x + ddx, y + ddy, col, 0.4);
                        }
                    }
                }
            }

            // Suave y aguado: más opaco en el centro que en el borde.
            Brush::Watercolor => {
                let rf = width * 0.75;
                let r = rf.ceil() as i32;
                for ddy in -r..=r {
                    for ddx in -r..=r {
                        let d = ((ddx * ddx + ddy * ddy) as f32).sqrt();
                        if d <= rf {
                            let a = 0.30 * (1.0 - d / rf);
                            c.blend(x + ddx, y + ddy, col, a);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Toda forma tiene que producir puntos dentro de su caja. Si una tabla
    /// tiene un valor fuera de 0..1, la forma se desborda en el lienzo y se ve
    /// recortada — este test lo caza sin tener que mirarla.
    #[test]
    fn toda_forma_queda_dentro_de_su_caja() {
        for s in ALL_SHAPES {
            let pts = s.points(100.0, 200.0, 300.0, 400.0);
            assert!(!pts.is_empty(), "{:?} no produjo puntos", s);
            for p in &pts {
                assert!(
                    p.0 >= 99.0 && p.0 <= 301.0 && p.1 >= 199.0 && p.1 <= 401.0,
                    "{:?} se sale de su caja: {:?}",
                    s,
                    p
                );
            }
            if s.is_closed() {
                assert!(
                    pts.len() >= 3,
                    "{:?} es cerrada pero tiene {} puntos",
                    s,
                    pts.len()
                );
            }
        }
    }

    /// Que los puntos estén dentro de la caja no dice nada del *orden*, y en un
    /// polígono el orden es la forma: con la cola pegada al final de la lista,
    /// el contorno atraviesa el globo en diagonal y el relleno par-impar deja
    /// agujeros. Esto lo caza; el test de arriba no.
    #[test]
    fn ninguna_forma_cerrada_se_cruza_a_si_misma() {
        // ¿Se cruzan de verdad los segmentos ab y cd? (tocarse no cuenta)
        fn cruzan(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
            let s = |p: Pt, q: Pt, r: Pt| {
                let v = (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0);
                if v.abs() < 1e-6 {
                    0
                } else if v > 0.0 {
                    1
                } else {
                    -1
                }
            };
            s(a, b, c) * s(a, b, d) < 0 && s(c, d, a) * s(c, d, b) < 0
        }

        for sh in ALL_SHAPES {
            if !sh.is_closed() {
                continue;
            }
            let p = sh.points(0.0, 0.0, 100.0, 100.0);
            let n = p.len();
            for i in 0..n {
                for j in i + 2..n {
                    // los segmentos primero y último comparten un vértice
                    if i == 0 && j == n - 1 {
                        continue;
                    }
                    assert!(
                        !cruzan(p[i], p[(i + 1) % n], p[j], p[(j + 1) % n]),
                        "{:?} se cruza a sí misma: segmento {i} contra {j}",
                        sh
                    );
                }
            }
        }
    }

    /// El relleno tiene que quedar adentro del contorno, no desbordarse.
    #[test]
    fn el_relleno_de_poligono_respeta_el_contorno() {
        let mut c = Canvas::new(40, 40);
        let cuadrado = [(10.0, 10.0), (30.0, 10.0), (30.0, 30.0), (10.0, 30.0)];
        fill_polygon(&mut c, &cuadrado, Color32::RED);

        // `geti` y no `get`: el lienzo sólo expone la lectura que puede
        // fallar, porque fuera de borde no hay color que devolver.
        let px = |x, y| c.geti(x, y).expect("dentro del lienzo");
        assert_eq!(px(20, 20), Color32::RED, "no rellenó el centro");
        assert_eq!(px(11, 11), Color32::RED, "no llegó a la esquina");
        assert_eq!(px(5, 20), Color32::WHITE, "se desbordó a la izquierda");
        assert_eq!(px(35, 20), Color32::WHITE, "se desbordó a la derecha");
        assert_eq!(px(20, 5), Color32::WHITE, "se desbordó arriba");
        assert_eq!(px(20, 35), Color32::WHITE, "se desbordó abajo");
    }

    #[test]
    fn el_generador_aleatorio_se_queda_en_rango() {
        let mut r = Rng::new();
        for _ in 0..10_000 {
            let u = r.unit();
            assert!((0.0..1.0).contains(&u), "xorshift devolvió {u}");
        }
    }
}
