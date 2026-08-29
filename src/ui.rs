//! La interfaz: los tres chromes y los widgets compartidos.
//!
//! Regla que rige este archivo: **geometría fija, nunca reflow.** El layout en
//! modo inmediato no sabe el tamaño de un grupo antes de dibujarlo (issue #4378
//! de egui, abierto desde 2024), así que los grupos que se acomodan solos
//! parpadean un frame. La cinta de Paint es geometría fija de todos modos, así
//! que se declaran las alturas y se dibuja una sola vez.
//!
//! El otro truco: **ningún archivo de iconos.** Las formas dibujan su propio
//! icono con la misma lista de puntos que usan en el lienzo, y los iconos de
//! herramienta son trazos vectoriales que toman el color del tema.

use crate::doc::{Doc, SelectMode, Stroke, Tool};
use crate::lang;
use crate::shapes::{Brush, Shape, ALL_BRUSHES, ALL_SHAPES};
use crate::text::TextBox;
use crate::theme::{gradient_bar, Chrome, Theme};
use ecolor::Color32;
use egui::{vec2, Align2, FontId, Pos2, Rect, Response, Sense, Ui};

/// Lo que la interfaz le pide a la aplicación. Se emiten como datos y se
/// aplican después, para no tener que prestar la app entera mientras se dibuja.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Cmd {
    New,
    Open,
    Save,
    SaveAs,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    /// Pegar una imagen desde un archivo, como "Pegar desde" de Paint.
    PasteFrom,
    SelectAll,
    Delete,
    Crop,
    ResizeDialog,
    PropertiesDialog,
    Rotate(u32),
    FlipH,
    FlipV,
    InvertColors,
    ZoomIn,
    ZoomOut,
    Zoom100,
    SetTheme(usize),
    ToggleGrid,
    ToggleRulers,
    ToggleStatusBar,
    ToggleThumbnail,
    FullScreen,
    Print,
    PrintPreview,
    /// Crear un dibujo con un tamaño elegido de la lista.
    NewSized(usize, usize),
    /// Abrir uno de los archivos recientes.
    OpenRecent(usize),
    /// Guardar una copia escalada, en porcentaje.
    Export(u32),
    /// Copiar el dibujo entero al portapapeles.
    CopyImage,
    /// Abrirlo en el explorador de archivos del sistema.
    Reveal,
    ToggleQatBelow,
    ToggleRibbonMin,
    About,
    Exit,
}

/// Los 20 colores fijos de la paleta de Paint, verificados contra el original.
/// La paleta de Paint de Windows 95 a XP: veintiocho colores en dos filas de
/// catorce, en su orden original.
///
/// Va aparte de `PALETTE` porque **son dos paletas distintas de verdad**: la de
/// la cinta de Windows 7 en adelante tiene veinte colores y otros tonos. Meter
/// los veintiocho en la grilla de diez columnas de la cinta descolocaría las
/// filas, y usar los veinte en XP dejaría la caja más corta que la ventana.
const XP_PALETTE: [[u8; 3]; 28] = [
    [0x00, 0x00, 0x00], [0x80, 0x80, 0x80], [0x80, 0x00, 0x00], [0x80, 0x80, 0x00],
    [0x00, 0x80, 0x00], [0x00, 0x80, 0x80], [0x00, 0x00, 0x80], [0x80, 0x00, 0x80],
    [0x80, 0x80, 0x40], [0x00, 0x40, 0x40], [0x00, 0x80, 0xff], [0x00, 0x40, 0x80],
    [0x80, 0x00, 0xff], [0x80, 0x40, 0x00],
    [0xff, 0xff, 0xff], [0xc0, 0xc0, 0xc0], [0xff, 0x00, 0x00], [0xff, 0xff, 0x00],
    [0x00, 0xff, 0x00], [0x00, 0xff, 0xff], [0x00, 0x00, 0xff], [0xff, 0x00, 0xff],
    [0xff, 0xff, 0x80], [0x00, 0xff, 0x80], [0x80, 0xff, 0xff], [0x80, 0x80, 0xff],
    [0xff, 0x00, 0x80], [0xff, 0x80, 0x40],
];

const PALETTE: [[u8; 3]; 20] = [
    [0x00, 0x00, 0x00],
    [0x7f, 0x7f, 0x7f],
    [0x88, 0x00, 0x15],
    [0xed, 0x1c, 0x24],
    [0xff, 0x7f, 0x27],
    [0xff, 0xf2, 0x00],
    [0x22, 0xb1, 0x4c],
    [0x00, 0xa2, 0xe8],
    [0x3f, 0x48, 0xcc],
    [0xa3, 0x49, 0xa4],
    [0xff, 0xff, 0xff],
    [0xc3, 0xc3, 0xc3],
    [0xb9, 0x7a, 0x57],
    [0xff, 0xae, 0xc9],
    [0xff, 0xc9, 0x0e],
    [0xef, 0xe4, 0xb0],
    [0xb5, 0xe6, 0x1d],
    [0x99, 0xd9, 0xea],
    [0x70, 0x92, 0xbe],
    [0xc8, 0xbf, 0xe7],
];

/// Los cuatro grosores de Paint. El borrador usa los suyos, más gruesos.
const WIDTHS: [f32; 4] = [1.0, 3.0, 5.0, 8.0];
const ERASER_WIDTHS: [f32; 4] = [4.0, 6.0, 8.0, 10.0];

// ---------------------------------------------------------------- iconos

/// Dibuja el icono de una herramienta con trazos vectoriales. Sin archivos de
/// imagen: escala perfecto en pantallas HiDPI y cambia de color con el tema.
fn tool_icon(ui: &Ui, r: Rect, tool: Tool, col: Color32) {
    let p = ui.painter();
    // El mismo grosor que los de archivo: en el riel del chrome propio están
    // uno debajo del otro y cualquier diferencia se lee como un error.
    let s = egui::Stroke::new(1.5, col);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    let pt = |fx: f32, fy: f32| Pos2::new(x + fx * w, y + fy * h);

    match tool {
        Tool::Pencil => {
            p.line_segment([pt(0.15, 0.85), pt(0.30, 0.80)], s);
            p.line_segment([pt(0.30, 0.80), pt(0.80, 0.22)], s);
            p.line_segment([pt(0.80, 0.22), pt(0.68, 0.12)], s);
            p.line_segment([pt(0.68, 0.12), pt(0.20, 0.70)], s);
            p.line_segment([pt(0.20, 0.70), pt(0.15, 0.85)], s);
        }
        // Pincel: cerdas rellenas abajo a la izquierda, virola y mango. Antes
        // eran tres rayas cruzadas y leía como jeringa: la cruz del extremo
        // parecía un émbolo.
        Tool::Brush => {
            // Cerdas: se ensanchan de la punta hacia la virola.
            p.add(egui::Shape::convex_polygon(
                vec![pt(0.11, 0.83), pt(0.33, 0.49), pt(0.52, 0.68), pt(0.17, 0.89)],
                col,
                egui::Stroke::NONE,
            ));
            // Virola: la banda de metal que las sujeta.
            p.add(egui::Shape::convex_polygon(
                vec![pt(0.33, 0.49), pt(0.43, 0.39), pt(0.62, 0.58), pt(0.52, 0.68)],
                col,
                egui::Stroke::NONE,
            ));
            // Mango: hueco, para que no sea un bloque macizo de color.
            p.add(egui::Shape::convex_polygon(
                vec![pt(0.45, 0.41), pt(0.82, 0.10), pt(0.90, 0.18), pt(0.60, 0.56)],
                Color32::TRANSPARENT,
                s,
            ));
        }
        Tool::Fill => {
            p.line_segment([pt(0.20, 0.50), pt(0.52, 0.16)], s);
            p.line_segment([pt(0.52, 0.16), pt(0.84, 0.50)], s);
            p.line_segment([pt(0.84, 0.50), pt(0.52, 0.82)], s);
            p.line_segment([pt(0.52, 0.82), pt(0.20, 0.50)], s);
            p.circle_filled(pt(0.86, 0.74), 0.09 * w, col);
        }
        Tool::Text => {
            p.line_segment([pt(0.20, 0.20), pt(0.80, 0.20)], s);
            p.line_segment([pt(0.50, 0.20), pt(0.50, 0.82)], s);
        }
        Tool::Eraser => {
            p.line_segment([pt(0.18, 0.66), pt(0.55, 0.24)], s);
            p.line_segment([pt(0.55, 0.24), pt(0.82, 0.48)], s);
            p.line_segment([pt(0.82, 0.48), pt(0.52, 0.78)], s);
            p.line_segment([pt(0.52, 0.78), pt(0.18, 0.66)], s);
            p.line_segment([pt(0.12, 0.88), pt(0.88, 0.88)], s);
        }
        Tool::Picker => {
            p.line_segment([pt(0.76, 0.16), pt(0.88, 0.28)], s);
            p.line_segment([pt(0.72, 0.24), pt(0.34, 0.62)], s);
            p.line_segment([pt(0.30, 0.66), pt(0.18, 0.86)], s);
        }
        Tool::Magnifier => {
            p.circle_stroke(pt(0.44, 0.44), 0.26 * w, s);
            p.line_segment([pt(0.64, 0.64), pt(0.86, 0.86)], s);
        }
        Tool::Select => {
            let d = egui::Stroke::new(1.0, col);
            for (a, b) in [
                ((0.15, 0.20), (0.42, 0.20)),
                ((0.58, 0.20), (0.85, 0.20)),
                ((0.15, 0.80), (0.42, 0.80)),
                ((0.58, 0.80), (0.85, 0.80)),
                ((0.15, 0.20), (0.15, 0.44)),
                ((0.15, 0.56), (0.15, 0.80)),
                ((0.85, 0.20), (0.85, 0.44)),
                ((0.85, 0.56), (0.85, 0.80)),
            ] {
                p.line_segment([pt(a.0, a.1), pt(b.0, b.1)], d);
            }
        }
        Tool::Shape => {
            p.rect_stroke(
                Rect::from_min_max(pt(0.18, 0.24), pt(0.82, 0.76)),
                0.0,
                s,
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// La forma dibuja su propio icono. Misma tabla de puntos que en el lienzo.
fn shape_icon(ui: &Ui, r: Rect, shape: Shape, col: Color32) {
    let p = ui.painter();
    let s = egui::Stroke::new(1.2, col);
    let inset = r.shrink(r.width() * 0.18);
    let pts = shape.points(inset.left(), inset.top(), inset.right(), inset.bottom());
    if pts.len() < 2 {
        return;
    }
    for i in 0..pts.len() - 1 {
        p.line_segment(
            [Pos2::new(pts[i].0, pts[i].1), Pos2::new(pts[i + 1].0, pts[i + 1].1)],
            s,
        );
    }
    if shape.is_closed() {
        let (a, b) = (pts[pts.len() - 1], pts[0]);
        p.line_segment([Pos2::new(a.0, a.1), Pos2::new(b.0, b.1)], s);
    }
}

/// Iconos chicos que no corresponden a una herramienta: los de la cinta.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ico {
    Cut,
    Copy,
    Paste,
    Crop,
    Resize,
    Rotate,
    New,
    Open,
    Save,
    Export,
    Reveal,
    Clock,
    Info,
    Exit,
    ZoomIn,
    ZoomOut,
    Zoom100,
    FullScreen,
    Thumbnail,
    Palette,
    FlipH,
    FlipV,
    Spectrum,
    Settings,
}

/// Qué icono lleva un botón. Unifica los de herramienta con los de la cinta
/// para que `big_button` y `row_button` compartan un solo parámetro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    T(Tool),
    I(Ico),
    None,
}

pub fn draw_icon(ui: &Ui, r: Rect, icon: Icon, col: Color32) {
    match icon {
        Icon::T(t) => tool_icon(ui, r, t, col),
        Icon::I(i) => small_icon(ui, r, i, col),
        Icon::None => {}
    }
}

fn small_icon(ui: &Ui, r: Rect, ico: Ico, col: Color32) {
    let p = ui.painter();
    // Un solo grosor para los treinta y pico. Cuando cada icono elegía el suyo,
    // deshacer salía al triple que rehacer y leían como dos programas distintos.
    let s = egui::Stroke::new(1.5, col);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    let pt = |fx: f32, fy: f32| Pos2::new(x + fx * w, y + fy * h);
    let rc = |a: (f32, f32), b: (f32, f32)| Rect::from_min_max(pt(a.0, a.1), pt(b.0, b.1));

    match ico {
        // Tijera: dos anillos y las hojas cruzadas.
        Ico::Cut => {
            p.circle_stroke(pt(0.28, 0.75), w * 0.13, s);
            p.circle_stroke(pt(0.72, 0.75), w * 0.13, s);
            p.line_segment([pt(0.35, 0.63), pt(0.75, 0.14)], s);
            p.line_segment([pt(0.65, 0.63), pt(0.25, 0.14)], s);
        }
        // Dos hojas: la de adelante entera y la de atrás asomando en ele.
        // Antes la de adelante se rellenaba de blanco para tapar a la otra, y
        // en un tema oscuro el blanco es justo el color que no va.
        Ico::Copy => {
            p.rect_stroke(rc((0.325, 0.325), (0.80, 0.875)), 1.0, s, egui::StrokeKind::Inside);
            p.add(egui::Shape::line(
                vec![pt(0.650, 0.325), pt(0.650, 0.125), pt(0.175, 0.125),
                     pt(0.175, 0.675), pt(0.325, 0.675)],
                s,
            ));
        }
        // Portapapeles: pinza arriba y dos renglones.
        Ico::Paste => {
            p.rect_stroke(rc((0.22, 0.16), (0.78, 0.90)), 1.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(rc((0.375, 0.125), (0.625, 0.225)), 1.0, s, egui::StrokeKind::Inside);
            p.line_segment([pt(0.36, 0.46), pt(0.64, 0.46)], s);
            p.line_segment([pt(0.36, 0.62), pt(0.64, 0.62)], s);
        }
        // Escuadras de recortar.
        Ico::Crop => {
            p.line_segment([pt(0.30, 0.06), pt(0.30, 0.70)], s);
            p.line_segment([pt(0.30, 0.70), pt(0.94, 0.70)], s);
            p.line_segment([pt(0.06, 0.30), pt(0.70, 0.30)], s);
            p.line_segment([pt(0.70, 0.30), pt(0.70, 0.94)], s);
        }
        // Dos rectángulos escalados.
        Ico::Resize => {
            p.rect_stroke(rc((0.10, 0.10), (0.58, 0.58)), 0.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(rc((0.42, 0.42), (0.90, 0.90)), 0.0, s, egui::StrokeKind::Inside);
        }
        // Flecha circular.
        Ico::Rotate => {
            let mut pts = Vec::with_capacity(15);
            for i in 0..=14 {
                let a = std::f32::consts::PI * (-0.35 + 1.55 * i as f32 / 14.0);
                pts.push(Pos2::new(
                    x + w * (0.5 + 0.36 * a.cos()),
                    y + h * (0.55 - 0.36 * a.sin()),
                ));
            }
            p.add(egui::Shape::line(pts.clone(), s));
            let tip = pts[0];
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(tip.x - 1.0, tip.y - 4.2),
                    Pos2::new(tip.x + 3.8, tip.y + 0.4),
                    Pos2::new(tip.x - 3.2, tip.y + 1.6),
                ],
                col,
                egui::Stroke::NONE,
            ));
        }
        // Hoja con la esquina doblada, sobre el rectángulo parado de la rejilla.
        Ico::New => {
            p.add(egui::Shape::line(
                vec![pt(0.275, 0.125), pt(0.625, 0.125), pt(0.750, 0.250),
                     pt(0.750, 0.875), pt(0.275, 0.875), pt(0.275, 0.125)],
                s,
            ));
            p.add(egui::Shape::line(
                vec![pt(0.625, 0.125), pt(0.625, 0.250), pt(0.750, 0.250)],
                s,
            ));
        }
        // Carpeta abierta: el cuerpo y la tapa inclinada al frente.
        Ico::Open => {
            p.add(egui::Shape::line(
                vec![pt(0.125, 0.775), pt(0.125, 0.225), pt(0.400, 0.225),
                     pt(0.475, 0.325), pt(0.875, 0.325), pt(0.875, 0.500)],
                s,
            ));
            p.add(egui::Shape::line(
                vec![pt(0.125, 0.775), pt(0.250, 0.450), pt(0.950, 0.450),
                     pt(0.825, 0.775), pt(0.125, 0.775)],
                s,
            ));
        }
        // Disquete con la esquina cortada. De contorno: el obturador macizo
        // era la única mancha rellena entre puros trazos.
        Ico::Save => {
            p.add(egui::Shape::line(
                vec![pt(0.150, 0.175), pt(0.700, 0.175), pt(0.850, 0.325),
                     pt(0.850, 0.875), pt(0.150, 0.875), pt(0.150, 0.175)],
                s,
            ));
            p.add(egui::Shape::line(
                vec![pt(0.300, 0.175), pt(0.300, 0.400), pt(0.650, 0.400), pt(0.650, 0.175)],
                s,
            ));
            p.add(egui::Shape::line(
                vec![pt(0.300, 0.875), pt(0.300, 0.625), pt(0.700, 0.625), pt(0.700, 0.875)],
                s,
            ));
        }
        // Flecha hacia una bandeja: exportar.
        Ico::Export => {
            p.line_segment([pt(0.50, 0.08), pt(0.50, 0.56)], s);
            p.line_segment([pt(0.32, 0.40), pt(0.50, 0.58)], s);
            p.line_segment([pt(0.68, 0.40), pt(0.50, 0.58)], s);
            p.line_segment([pt(0.14, 0.66), pt(0.14, 0.88)], s);
            p.line_segment([pt(0.14, 0.88), pt(0.86, 0.88)], s);
            p.line_segment([pt(0.86, 0.88), pt(0.86, 0.66)], s);
        }
        // Carpeta con flecha: mostrar en el explorador.
        Ico::Reveal => {
            p.line_segment([pt(0.08, 0.80), pt(0.08, 0.22)], s);
            p.line_segment([pt(0.08, 0.22), pt(0.40, 0.22)], s);
            p.line_segment([pt(0.40, 0.22), pt(0.50, 0.34)], s);
            p.line_segment([pt(0.50, 0.34), pt(0.84, 0.34)], s);
            p.line_segment([pt(0.08, 0.80), pt(0.84, 0.80)], s);
            p.line_segment([pt(0.62, 0.62), pt(0.80, 0.50)], s);
            p.line_segment([pt(0.80, 0.50), pt(0.62, 0.40)], s);
        }
        // Reloj: los recientes.
        Ico::Clock => {
            p.circle_stroke(pt(0.5, 0.5), w * 0.38, s);
            p.line_segment([pt(0.5, 0.26), pt(0.5, 0.52)], s);
            p.line_segment([pt(0.5, 0.52), pt(0.68, 0.62)], s);
        }
        // Círculo con la i.
        Ico::Info => {
            p.circle_stroke(pt(0.5, 0.5), w * 0.38, s);
            p.line_segment([pt(0.5, 0.46), pt(0.5, 0.72)], s);
            p.circle_filled(pt(0.5, 0.32), 1.0, col);
        }
        // Puerta con flecha: salir.
        Ico::Exit => {
            p.line_segment([pt(0.42, 0.12), pt(0.12, 0.12)], s);
            p.line_segment([pt(0.12, 0.12), pt(0.12, 0.88)], s);
            p.line_segment([pt(0.12, 0.88), pt(0.42, 0.88)], s);
            p.line_segment([pt(0.60, 0.30), pt(0.82, 0.50)], s);
            p.line_segment([pt(0.82, 0.50), pt(0.60, 0.70)], s);
            p.line_segment([pt(0.34, 0.50), pt(0.82, 0.50)], s);
        }
        // Lupa con signo: `+` acerca, `-` aleja. Antes compartían icono y no se
        // distinguían sin leer la etiqueta.
        Ico::ZoomIn | Ico::ZoomOut => {
            p.circle_stroke(pt(0.42, 0.42), w * 0.28, s);
            p.line_segment([pt(0.62, 0.62), pt(0.88, 0.88)], s);
            p.line_segment([pt(0.29, 0.42), pt(0.55, 0.42)], s);
            if ico == Ico::ZoomIn {
                p.line_segment([pt(0.42, 0.29), pt(0.42, 0.55)], s);
            }
        }
        // Una regla con la diagonal del 100%.
        Ico::Zoom100 => {
            p.rect_stroke(rc((0.10, 0.26), (0.90, 0.74)), 1.0, s, egui::StrokeKind::Inside);
            for k in 0..3 {
                let x = 0.24 + k as f32 * 0.13;
                p.line_segment([pt(x, 0.40), pt(x, 0.60)], s);
            }
            p.line_segment([pt(0.62, 0.62), pt(0.78, 0.38)], s);
        }
        // Pantalla con las cuatro esquinas marcadas.
        Ico::FullScreen => {
            p.rect_stroke(rc((0.08, 0.16), (0.92, 0.84)), 1.0, s, egui::StrokeKind::Inside);
            for (cx, cy, dx, dy) in [
                (0.22, 0.36, 1.0, -1.0),
                (0.78, 0.36, -1.0, -1.0),
                (0.22, 0.64, 1.0, 1.0),
                (0.78, 0.64, -1.0, 1.0),
            ] {
                p.line_segment([pt(cx, cy), pt(cx + 0.11 * dx, cy)], s);
                p.line_segment([pt(cx, cy), pt(cx, cy + 0.11 * dy)], s);
            }
        }
        // Ventana con la vista chica adentro.
        Ico::Thumbnail => {
            p.rect_stroke(rc((0.08, 0.14), (0.92, 0.86)), 1.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(rc((0.50, 0.50), (0.84, 0.78)), 0.0, s, egui::StrokeKind::Inside);
        }
        // Paleta de pintor, monocroma. Los cuatro pegotes eran rojo, amarillo,
        // verde y azul fijos: en un tema oscuro no combinaban con nada, y en
        // uno cuya idea es que el único color sea tu dibujo, sobraban.
        Ico::Palette => {
            p.circle_stroke(pt(0.5, 0.5), w * 0.375, s);
            p.circle_stroke(pt(0.66, 0.62), w * 0.09, s);
            for (fx, fy) in [(0.30, 0.40), (0.42, 0.28), (0.58, 0.28), (0.70, 0.40)] {
                p.circle_filled(pt(fx, fy), w * 0.075, col);
            }
        }
        // Voltear: dos triángulos espejados con el eje punteado.
        Ico::FlipH | Ico::FlipV => {
            let horiz = ico == Ico::FlipH;
            let tri = |a: (f32, f32), b: (f32, f32), c: (f32, f32), fill: bool| {
                let v = vec![pt(a.0, a.1), pt(b.0, b.1), pt(c.0, c.1)];
                if fill {
                    p.add(egui::Shape::convex_polygon(v, col, egui::Stroke::NONE));
                } else {
                    p.add(egui::Shape::closed_line(v, s));
                }
            };
            if horiz {
                tri((0.06, 0.16), (0.42, 0.16), (0.42, 0.84), true);
                tri((0.94, 0.16), (0.58, 0.16), (0.58, 0.84), false);
                for k in 0..4 {
                    let y = 0.14 + k as f32 * 0.24;
                    p.line_segment([pt(0.5, y), pt(0.5, y + 0.12)], s);
                }
            } else {
                tri((0.16, 0.06), (0.16, 0.42), (0.84, 0.42), true);
                tri((0.16, 0.94), (0.16, 0.58), (0.84, 0.58), false);
                for k in 0..4 {
                    let x = 0.14 + k as f32 * 0.24;
                    p.line_segment([pt(x, 0.5), pt(x + 0.12, 0.5)], s);
                }
            }
        }
        // Espectro de "Editar colores": cuatro franjas de color.
        // Engranaje: un círculo y ocho dientes cortos. Ocho radios sueltos
        // leerían como un sol, así que el diente arranca *en* la circunferencia.
        // Engranaje: corona, cubo y ocho dientes **gruesos y cortos**.
        //
        // La versión de antes eran ocho rayas finas de radio 0,22 a 0,38 sobre
        // un círculo chico: eso es un sol, no un engranaje. Lo que lo vuelve
        // engranaje es que el diente sea ancho —más que el trazo— y que se vea
        // el agujero del centro.
        Ico::Settings => {
            let diente = egui::Stroke::new(w * 0.13, col);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU;
                let (c, sn) = (a.cos(), a.sin());
                p.line_segment(
                    [
                        pt(0.5 + 0.27 * c, 0.5 + 0.27 * sn),
                        pt(0.5 + 0.42 * c, 0.5 + 0.42 * sn),
                    ],
                    diente,
                );
            }
            p.circle_stroke(pt(0.5, 0.5), w * 0.29, egui::Stroke::new(w * 0.13, col));
            p.circle_filled(pt(0.5, 0.5), w * 0.15, col);
            p.circle_filled(pt(0.5, 0.5), w * 0.09, Color32::TRANSPARENT);
        }
        Ico::Spectrum => {
            let bands = [
                Color32::from_rgb(0x3f, 0x48, 0xcc),
                Color32::from_rgb(0x22, 0xb1, 0x4c),
                Color32::from_rgb(0xff, 0xf2, 0x00),
                Color32::from_rgb(0xed, 0x1c, 0x24),
            ];
            for (i, c) in bands.iter().enumerate() {
                let a = 0.10 + i as f32 * 0.20;
                p.rect_filled(rc((a, 0.18), (a + 0.20, 0.82)), 0.0, *c);
            }
            p.rect_stroke(
                rc((0.10, 0.18), (0.90, 0.82)),
                0.0,
                egui::Stroke::new(1.0, col),
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// Triangulito de desplegable. La fuente por defecto de egui **no trae** `▾`
/// ni `↶`/`↷`: salían como cuadrados vacíos. Dibujarlos sale más barato que
/// cargar una fuente con esos glifos.
fn caret(ui: &Ui, c: Pos2, col: Color32) {
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            Pos2::new(c.x - 3.5, c.y - 2.0),
            Pos2::new(c.x + 3.5, c.y - 2.0),
            Pos2::new(c.x, c.y + 2.5),
        ],
        col,
        egui::Stroke::NONE,
    ));
}
/// Flecha curva de deshacer/rehacer: un arco de 3/4 con la punta rellena.
/// La anterior era medio círculo con dos rayitas y se veía pobre.
/// Deshacer y rehacer: la misma flecha, espejada.
///
/// Antes eran dos dibujos distintos —un arco de 1,9 con punta rellena para una,
/// y otro más fino para la otra— y puestos uno al lado del otro se notaba.
/// Acá sale una sola: la punta en galón, el asta recta y la curva de vuelta.
fn undo_arrow(ui: &Ui, r: Rect, col: Color32, forward: bool) {
    let p = ui.painter();
    let s = egui::Stroke::new(1.5, col);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    // Espejo sobre el eje vertical de la caja, para que las dos sean la misma.
    let pt = |fx: f32, fy: f32| {
        let fx = if forward { 1.0 - fx } else { fx };
        Pos2::new(x + fx * w, y + fy * h)
    };

    // La punta.
    p.add(egui::Shape::line(
        vec![pt(0.375, 0.325), pt(0.175, 0.525), pt(0.375, 0.725)],
        s,
    ));
    // El asta y la curva que vuelve hacia abajo.
    let mut pts = vec![pt(0.175, 0.525), pt(0.575, 0.525)];
    // Media circunferencia a la derecha del asta: el radio es la mitad de lo
    // que baja, así el codo queda redondo y no en escuadra.
    for k in 0..=10 {
        let a = std::f32::consts::PI * (-0.5 + k as f32 / 10.0);
        let (cx, cy, rad) = (0.575, 0.700, 0.175);
        pts.push(pt(cx + rad * a.cos(), cy + rad * a.sin()));
    }
    pts.push(pt(0.475, 0.875));
    p.add(egui::Shape::line(pts, s));
}

/// El icono de la aplicación: una paleta de pintor con sus colores y el hueco
/// del pulgar. Dibujado, como todo lo demás — cero archivos de imagen.
/// La marca: la paleta de pintor.
///
/// El cuerpo y los pegotes tienen color propio y no el del tema, como cualquier
/// icono de aplicación: es la única cosa de la interfaz que sí puede tener
/// color, porque no es interfaz sino identidad. El contorno **sí** sale del
/// tema, y es lo que hace que se despegue igual del blanco de la barra clara
/// que del negro de la oscura.
pub fn app_icon(ui: &Ui, r: Rect, col: Color32) {
    let p = ui.painter();
    let c = r.center();
    let rx = r.width() * 0.44;
    let ry = r.height() * 0.40;

    // El cuerpo: una elipse con la muesca del pulgar abajo a la derecha.
    let mut body = Vec::with_capacity(28);
    for i in 0..28 {
        let a = i as f32 / 28.0 * std::f32::consts::TAU;
        let k = if a > 0.15 && a < 1.15 { 0.72 } else { 1.0 };
        body.push(Pos2::new(c.x + rx * a.cos() * k, c.y + ry * a.sin() * k));
    }
    p.add(egui::Shape::convex_polygon(
        body,
        Color32::from_rgb(0xf3, 0xe4, 0xc8),
        egui::Stroke::new((r.width() * 0.055).clamp(1.0, 2.4), col),
    ));

    // El hueco del pulgar.
    p.circle_filled(
        Pos2::new(c.x + rx * 0.34, c.y + ry * 0.30),
        r.width() * 0.10,
        Color32::WHITE,
    );

    // Los pegotes, en el orden de la paleta de Paint.
    for (i, pig) in [
        Color32::from_rgb(0xed, 0x1c, 0x24),
        Color32::from_rgb(0xff, 0xf2, 0x00),
        Color32::from_rgb(0x22, 0xb1, 0x4c),
        Color32::from_rgb(0x00, 0xa2, 0xe8),
    ]
    .iter()
    .enumerate()
    {
        let a = std::f32::consts::PI * (1.15 + i as f32 * 0.24);
        p.circle_filled(
            Pos2::new(c.x + rx * 0.52 * a.cos(), c.y + ry * 0.52 * a.sin()),
            r.width() * 0.085,
            *pig,
        );
    }
}

/// Qué botones puede llevar la barra de acceso rápido.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Qat {
    New,
    Open,
    Save,
    Print,
    Preview,
    Undo,
    Redo,
}

pub const ALL_QAT: [Qat; 7] = [
    Qat::New,
    Qat::Open,
    Qat::Save,
    Qat::Print,
    Qat::Preview,
    Qat::Undo,
    Qat::Redo,
];

impl Qat {
    pub fn label(self) -> &'static str {
        lang::t(match self {
            Self::New => "Nuevo",
            Self::Open => "Abrir",
            Self::Save => "Guardar",
            Self::Print => "Imprimir",
            Self::Preview => "Vista previa de impresión",
            Self::Undo => "Deshacer",
            Self::Redo => "Rehacer",
        })
    }

    pub fn cmd(self) -> Cmd {
        match self {
            Self::New => Cmd::New,
            Self::Open => Cmd::Open,
            Self::Save => Cmd::Save,
            Self::Print => Cmd::Print,
            Self::Preview => Cmd::PrintPreview,
            Self::Undo => Cmd::Undo,
            Self::Redo => Cmd::Redo,
        }
    }
}

/// Dibuja el icono de un botón de la barra rápida.
fn qat_icon(ui: &Ui, r: Rect, kind: Qat, col: Color32) {
    let p = ui.painter();
    let s = egui::Stroke::new(1.5, col);
    let (x, y, w, h) = (r.left(), r.top(), r.width(), r.height());
    let pt = |fx: f32, fy: f32| Pos2::new(x + fx * w, y + fy * h);
    let rc = |a: (f32, f32), b: (f32, f32)| Rect::from_min_max(pt(a.0, a.1), pt(b.0, b.1));

    match kind {
        // Disquete: cuerpo, etiqueta abajo y obturador arriba.
        Qat::Save => {
            p.rect_stroke(rc((0.14, 0.14), (0.86, 0.86)), 1.0, s, egui::StrokeKind::Inside);
            p.rect_filled(rc((0.32, 0.14), (0.68, 0.42)), 0.0, col);
            p.rect_stroke(rc((0.28, 0.54), (0.72, 0.86)), 0.0, s, egui::StrokeKind::Inside);
        }
        // Hoja con la esquina doblada.
        Qat::New => {
            p.line_segment([pt(0.24, 0.12), pt(0.62, 0.12)], s);
            p.line_segment([pt(0.62, 0.12), pt(0.78, 0.30)], s);
            p.line_segment([pt(0.78, 0.30), pt(0.78, 0.88)], s);
            p.line_segment([pt(0.78, 0.88), pt(0.24, 0.88)], s);
            p.line_segment([pt(0.24, 0.88), pt(0.24, 0.12)], s);
            p.line_segment([pt(0.62, 0.12), pt(0.62, 0.30)], s);
            p.line_segment([pt(0.62, 0.30), pt(0.78, 0.30)], s);
        }
        // Carpeta abierta.
        Qat::Open => {
            p.line_segment([pt(0.12, 0.80), pt(0.12, 0.26)], s);
            p.line_segment([pt(0.12, 0.26), pt(0.42, 0.26)], s);
            p.line_segment([pt(0.42, 0.26), pt(0.52, 0.38)], s);
            p.line_segment([pt(0.52, 0.38), pt(0.84, 0.38)], s);
            p.line_segment([pt(0.12, 0.80), pt(0.88, 0.80)], s);
            p.line_segment([pt(0.84, 0.38), pt(0.88, 0.80)], s);
        }
        // Impresora: papel arriba, cuerpo, salida abajo.
        Qat::Print => {
            p.rect_stroke(rc((0.28, 0.10), (0.72, 0.34)), 0.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(rc((0.14, 0.34), (0.86, 0.66)), 1.0, s, egui::StrokeKind::Inside);
            p.rect_stroke(rc((0.28, 0.66), (0.72, 0.90)), 0.0, s, egui::StrokeKind::Inside);
        }
        // Hoja con lupa.
        Qat::Preview => {
            p.rect_stroke(rc((0.16, 0.12), (0.66, 0.80)), 0.0, s, egui::StrokeKind::Inside);
            p.circle_stroke(pt(0.62, 0.62), w * 0.20, s);
            p.line_segment([pt(0.76, 0.76), pt(0.90, 0.90)], s);
        }
        Qat::Undo => undo_arrow(ui, r, col, false),
        Qat::Redo => undo_arrow(ui, r, col, true),
    }
}

// --------------------------------------------------------------- botones

fn frame_button(ui: &mut Ui, size: f32, active: bool, theme: &Theme) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(vec2(size, size), Sense::click());

    // En los temas de caja de herramientas la herramienta puesta se ve
    // **hundida**, no pintada de celeste. Es lo más característico de un
    // programa de los noventa, y con el relleno plano el XP se veía como un
    // Windows 10 con otros colores.
    if theme.chrome == Chrome::Palette {
        if active {
            sunken(ui, rect, theme);
        } else if resp.hovered() {
            raised(ui, rect, theme);
        }
        return (rect, resp);
    }

    let bg = if active {
        theme.button_active.into()
    } else if resp.hovered() {
        theme.button_hover.into()
    } else {
        Color32::TRANSPARENT
    };
    let cr = theme.button_rounding;
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, cr, bg);
    }
    if active || resp.hovered() {
        ui.painter().rect_stroke(
            rect,
            cr,
            egui::Stroke::new(1.0, Color32::from(theme.accent)),
            egui::StrokeKind::Inside,
        );
    }
    (rect, resp)
}

fn tool_button(ui: &mut Ui, theme: &Theme, tool: Tool, active: bool) -> bool {
    let (rect, resp) = frame_button(ui, 24.0, active, theme);
    tool_icon(ui, rect.shrink(4.0), tool, theme.icon.into());
    resp.on_hover_text(tool.label()).clicked()
}

fn shape_button(ui: &mut Ui, theme: &Theme, shape: Shape, active: bool) -> bool {
    let (rect, resp) = frame_button(ui, 20.0, active, theme);
    shape_icon(ui, rect, shape, theme.icon.into());
    resp.on_hover_text(shape.label()).clicked()
}

/// Botón grande: icono arriba, etiqueta centrada debajo y triangulito al pie.
/// El ancho sale del texto — "Seleccionar" no entra en 46 px y se desbordaba
/// sobre el grupo de al lado.
/// Qué mitad de un botón partido se tocó.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Half {
    /// La zona del icono: ejecuta la acción principal.
    Main,
    /// El triangulito de abajo: abre el menú.
    Caret,
}

/// Botón partido, como Pegar o Seleccionar en Paint: el icono ejecuta la acción
/// y el triangulito abre el desplegable. Antes era un solo clic, así que las
/// entradas del menú no tenían cómo alcanzarse.
fn big_split(ui: &mut Ui, theme: &Theme, label: &str, icon: Icon, active: bool) -> (Option<Half>, Rect) {
    let w = (label.chars().count() as f32 * theme.font_size * 0.58 + 12.0).max(46.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 66.0), Sense::click());
    let bg = if active {
        theme.button_active.into()
    } else if resp.hovered() {
        theme.button_hover.into()
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, theme.button_rounding, bg);
        if active {
            ui.painter().rect_stroke(
                rect,
                theme.button_rounding,
                egui::Stroke::new(1.0, Color32::from(theme.accent)),
                egui::StrokeKind::Inside,
            );
        }
    }
    draw_icon(
        ui,
        Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 19.0), vec2(26.0, 26.0)),
        icon,
        theme.icon.into(),
    );
    ui.painter().text(
        Pos2::new(rect.center().x, rect.top() + 43.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    // La franja del triangulito se resalta aparte, para que se vea partido.
    let split_y = rect.bottom() - 16.0;
    if resp.hovered() {
        ui.painter().line_segment(
            [Pos2::new(rect.left() + 3.0, split_y), Pos2::new(rect.right() - 3.0, split_y)],
            egui::Stroke::new(1.0, Color32::from(theme.border)),
        );
    }
    caret(ui, Pos2::new(rect.center().x, rect.bottom() - 8.0), theme.text.into());

    let half = resp.clicked().then(|| {
        let y = resp.interact_pointer_pos().map(|p| p.y).unwrap_or(rect.top());
        if y >= split_y { Half::Caret } else { Half::Main }
    });
    (half, rect)
}

/// Como `big_button` pero sin triangulito: para los que no abren nada.
fn big_plain(ui: &mut Ui, theme: &Theme, label: &str, icon: Icon, active: bool) -> bool {
    let w = (label.chars().count() as f32 * theme.font_size * 0.58 + 12.0).max(46.0);
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 66.0), Sense::click());
    if active {
        ui.painter()
            .rect_filled(rect, theme.button_rounding, Color32::from(theme.button_active));
        ui.painter().rect_stroke(
            rect,
            theme.button_rounding,
            egui::Stroke::new(1.0, Color32::from(theme.accent)),
            egui::StrokeKind::Inside,
        );
    } else if resp.hovered() {
        ui.painter()
            .rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
    }
    draw_icon(
        ui,
        Rect::from_center_size(Pos2::new(rect.center().x, rect.top() + 22.0), vec2(26.0, 26.0)),
        icon,
        theme.icon.into(),
    );
    for (i, line) in label.split('\n').enumerate() {
        ui.painter().text(
            Pos2::new(rect.center().x, rect.top() + 46.0 + i as f32 * 13.0),
            Align2::CENTER_CENTER,
            line,
            FontId::proportional(theme.font_size),
            theme.text.into(),
        );
    }
    resp.clicked()
}

/// Color 1 y Color 2, lado a lado y con etiqueta de una línea, como en la
/// cinta. (Superpuestos es el estilo de XP.)
fn color_boxes(ui: &mut Ui, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    for (is_c1, label) in [(true, "Color 1"), (false, "Color 2")] {
        let (rect, resp) = ui.allocate_exact_size(vec2(44.0, 60.0), Sense::click());
        if out.picking_c1 == is_c1 {
            ui.painter()
                .rect_filled(rect, theme.button_rounding, Color32::from(theme.button_active));
            ui.painter().rect_stroke(
                rect,
                theme.button_rounding,
                egui::Stroke::new(1.0, Color32::from(theme.accent)),
                egui::StrokeKind::Inside,
            );
        }
        let sw = Rect::from_center_size(
            Pos2::new(rect.center().x, rect.top() + 20.0),
            vec2(26.0, 26.0),
        );
        ui.painter()
            .rect_filled(sw, 0.0, if is_c1 { doc.color1 } else { doc.color2 });
        ui.painter().rect_stroke(
            sw,
            0.0,
            egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            Pos2::new(rect.center().x, rect.bottom() - 14.0),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(theme.font_size),
            theme.text.into(),
        );
        if resp.clicked() {
            out.picking_c1 = is_c1;
        }
    }
}

/// El grupo Tamaño: las cuatro líneas de grosor, la etiqueta y el triangulito.
/// Los grosores se dibujan a escala real (1, 3, 5 y 8 px) — antes salían casi
/// iguales y no se entendía qué elegía uno.
fn size_button(ui: &mut Ui, theme: &Theme, doc: &Doc, out: &mut UiOut) {
    let widths = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
    let (rect, resp) = ui.allocate_exact_size(vec2(46.0, 66.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
    }
    // El paso se calcula con el grosor de cada línea, no con un salto fijo: con
    // 9,5 px la de 8 quedaba pegada a la de 5 y el bloque se leía como una
    // mancha. Las muestras van en negro, como en Paint.
    const GAP: f32 = 5.5;
    const ARRIBA: f32 = 8.0;
    // Hasta dónde puede llegar el bloque: la etiqueta va centrada en
    // `bottom - 20`, así que su borde de arriba está unos 7 px más alto.
    let tope = rect.bottom() - 29.0;
    // El bloque se encoge parejo si no entra. Los grosores de la goma
    // (4, 6, 8 y 10) suman el doble que los del lápiz y **se comían la
    // etiqueta**: las cuatro rayas caían encima de la palabra «Tamaño».
    // Escalar los cuatro por igual mantiene legible cuál es más gordo.
    let pide: f32 =
        widths.iter().sum::<f32>() + GAP * (widths.len() - 1) as f32;
    let k = ((tope - rect.top() - ARRIBA) / pide).min(1.0);
    let col = Color32::from(theme.text);
    let mut y = rect.top() + ARRIBA;
    for (i, w) in widths.iter().enumerate() {
        let paso = if i == 0 { *w / 2.0 } else { widths[i - 1] / 2.0 + GAP + *w / 2.0 };
        y += paso * k;
        ui.painter().line_segment(
            [Pos2::new(rect.left() + 9.0, y), Pos2::new(rect.right() - 9.0, y)],
            egui::Stroke::new(*w * k, col),
        );
    }
    ui.painter().text(
        Pos2::new(rect.center().x, rect.bottom() - 20.0),
        Align2::CENTER_CENTER,
        lang::t("Tamaño"),
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    caret(ui, Pos2::new(rect.center().x, rect.bottom() - 7.0), theme.text.into());
    // Antes ciclaba al hacer clic: cambiaba el grosor sin decir a cuál, y las
    // cuatro opciones no se veían nunca.
    if resp.clicked() {
        out.menu_anchor = rect.left_bottom();
        out.open_size_menu = true;
    }
}

/// Una fila del menú de grosor: la línea a su ancho real, centrada.
pub fn menu_row_width(ui: &mut Ui, theme: &Theme, w: f32, checked: bool) -> bool {
    let aw = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(aw, 26.0), Sense::click());
    if checked {
        ui.painter().rect_filled(rect, 0.0, Color32::from(theme.button_active));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, Color32::from(theme.button_hover));
    }
    ui.painter().line_segment(
        [
            Pos2::new(rect.left() + 12.0, rect.center().y),
            Pos2::new(rect.right() - 12.0, rect.center().y),
        ],
        egui::Stroke::new(w, Color32::from(theme.text)),
    );
    resp.on_hover_text(format!("{w} px")).clicked()
}

/// Qué se tocó en la paleta de la cinta.
pub enum PaletteHit {
    /// Uno de los veinte fijos.
    Fixed(usize),
    /// Uno de los diez personalizados.
    Custom(usize),
}

/// Los veinte colores fijos en 10×2, y debajo los diez personalizados.
///
/// La fila de personalizados es lo que más faltaba: los colores que guardás en
/// el diálogo no se veían en ninguna parte de la cinta, así que para usarlos
/// había que volver a abrir el diálogo.
fn palette_grid(ui: &mut Ui, theme: &Theme, custom: &[Option<Color32>]) -> Option<PaletteHit> {
    const CELL: f32 = 15.0;
    const GAP: f32 = 2.0;
    const STEP: f32 = CELL + GAP;
    // Tres filas: dos de fijos y una de personalizados, con más aire entre medio.
    const SPLIT: f32 = 5.0;

    let (rect, resp) = ui.allocate_exact_size(
        vec2(10.0 * STEP, 3.0 * STEP + SPLIT),
        Sense::click(),
    );
    let p = ui.painter();
    let border = egui::Stroke::new(1.0, Color32::from(theme.border_strong));

    let cell_rect = |col: usize, row: usize| {
        let y = rect.top() + row as f32 * STEP + if row == 2 { SPLIT } else { 0.0 };
        Rect::from_min_size(
            Pos2::new(rect.left() + col as f32 * STEP, y),
            vec2(CELL, CELL),
        )
    };

    for (i, rgb) in PALETTE.iter().enumerate() {
        let r = cell_rect(i % 10, i / 10);
        p.rect_filled(r, 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        p.rect_stroke(r, 0.0, border, egui::StrokeKind::Inside);
    }

    for col in 0..10 {
        let r = cell_rect(col, 2);
        match custom.get(col).copied().flatten() {
            Some(c) => {
                p.rect_filled(r, 0.0, c);
                p.rect_stroke(r, 0.0, border, egui::StrokeKind::Inside);
            }
            // Hueco libre: punteado, igual que en el diálogo.
            None => {
                p.rect_filled(r, 0.0, Color32::WHITE);
                let d = egui::Stroke::new(1.0, Color32::from(theme.border_strong));
                let mut x = r.left();
                while x < r.right() {
                    let x2 = (x + 2.0).min(r.right());
                    p.line_segment([Pos2::new(x, r.top()), Pos2::new(x2, r.top())], d);
                    p.line_segment([Pos2::new(x, r.bottom()), Pos2::new(x2, r.bottom())], d);
                    x += 4.0;
                }
                let mut y = r.top();
                while y < r.bottom() {
                    let y2 = (y + 2.0).min(r.bottom());
                    p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.left(), y2)], d);
                    p.line_segment([Pos2::new(r.right(), y), Pos2::new(r.right(), y2)], d);
                    y += 4.0;
                }
            }
        }
    }

    if resp.clicked() {
        let pos = resp.interact_pointer_pos()?;
        for row in 0..3 {
            for col in 0..10 {
                if cell_rect(col, row).contains(pos) {
                    return Some(if row == 2 {
                        PaletteHit::Custom(col)
                    } else {
                        PaletteHit::Fixed(row * 10 + col)
                    });
                }
            }
        }
    }
    None
}


/// Aplica el color tocado al Color 1 o al 2, según cuál esté elegido.
fn apply_palette_hit(hit: PaletteHit, doc: &mut Doc, out: &UiOut) {
    let c = match hit {
        PaletteHit::Fixed(i) => {
            Color32::from_rgb(PALETTE[i][0], PALETTE[i][1], PALETTE[i][2])
        }
        PaletteHit::Custom(i) => match out.custom.get(i).copied().flatten() {
            Some(c) => c,
            // Hueco vacío: no hay nada que elegir.
            None => return,
        },
    };
    if out.picking_c1 {
        doc.color1 = c
    } else {
        doc.color2 = c
    }
}

// ------------------------------------------------------------ menús propios

/// Un panel de menú anclado a su botón, como los desplegables de Paint.
///
/// Antes esto eran `egui::Window`: título centrado gigante, botones sueltos
/// apilados y ninguna relación visual con el botón que los abría. Un menú de
/// verdad es un panel angosto pegado debajo, con filas de alto parejo, icono a
/// la izquierda y resaltado de ancho completo.
///
/// Devuelve `false` cuando hay que cerrarlo: se eligió algo, se hizo clic
/// afuera o se apretó Escape.
pub fn menu_panel(
    ctx: &egui::Context,
    theme: &Theme,
    id: &str,
    anchor: Pos2,
    width: f32,
    add: impl FnOnce(&mut Ui) -> bool,
) -> bool {
    let mut keep = true;

    let area = egui::Area::new(egui::Id::new(id))
        .order(egui::Order::Foreground)
        .fixed_pos(anchor)
        .show(ctx, |ui| {
            ui.set_width(width);
            let frame = egui::Frame::NONE
                .fill(Color32::from(theme.surface))
                .stroke(egui::Stroke::new(1.0, Color32::from(theme.border_strong)))
                .inner_margin(egui::Margin::symmetric(1, 3))
                .shadow(egui::Shadow {
                    offset: [1, 2],
                    blur: 6,
                    spread: 0,
                    color: Color32::from_black_alpha(40),
                });
            frame
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.set_width(width);
                    add(ui)
                })
                .inner
        });

    if !area.inner {
        keep = false;
    }

    // Clic afuera o Escape cierran, como cualquier menú.
    let outside = ctx.input(|i| {
        i.pointer.any_pressed()
            && i.pointer
                .interact_pos()
                .is_some_and(|p| !area.response.rect.contains(p))
    });
    if outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        keep = false;
    }
    keep
}

/// Una fila de menú: icono, etiqueta y, si corresponde, marca de verificación.
pub fn menu_row(
    ui: &mut Ui,
    theme: &Theme,
    icon: Icon,
    label: &str,
    checked: Option<bool>,
) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    if resp.hovered() {
        ui.painter()
            .rect_filled(rect, 0.0, Color32::from(theme.button_hover));
    }

    let icon_x = rect.left() + 15.0;
    if let Some(true) = checked {
        // La marca ocupa la columna del icono, como en Windows.
        let c = Pos2::new(icon_x, rect.center().y);
        let s = egui::Stroke::new(1.5, Color32::from(theme.icon));
        let mid = Pos2::new(c.x - 0.5, c.y + 3.0);
        ui.painter().line_segment([Pos2::new(c.x - 4.5, c.y), mid], s);
        ui.painter().line_segment([mid, Pos2::new(c.x + 4.5, c.y - 4.0)], s);
    } else if icon != Icon::None {
        draw_icon(
            ui,
            Rect::from_center_size(Pos2::new(icon_x, rect.center().y), vec2(16.0, 16.0)),
            icon,
            theme.icon.into(),
        );
    }

    ui.painter().text(
        Pos2::new(rect.left() + 30.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// Fila de menú con una muestra de imagen en lugar de icono. La usan las
/// galerías de contorno y relleno, que en Paint muestran cómo queda cada trazo.
pub fn menu_row_sample(
    ui: &mut Ui,
    theme: &Theme,
    tex: Option<(egui::TextureId, Rect)>,
    slash: bool,
    label: &str,
    checked: bool,
) -> bool {
    let w = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 26.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, 0.0, Color32::from(theme.button_hover));
    }

    let sw = Rect::from_min_size(Pos2::new(rect.left() + 5.0, rect.top() + 3.0), vec2(30.0, 20.0));
    ui.painter().rect_filled(sw, 0.0, Color32::from(theme.surface));
    if let Some((id, uv)) = tex {
        ui.painter().image(id, sw, uv, Color32::WHITE);
    }
    if slash {
        // "Sin contorno": el cuadro vacío con la diagonal roja del original.
        ui.painter().line_segment(
            [
                Pos2::new(sw.left() + 2.0, sw.bottom() - 2.0),
                Pos2::new(sw.right() - 2.0, sw.top() + 2.0),
            ],
            egui::Stroke::new(1.6, Color32::from_rgb(0xd0, 0x21, 0x21)),
        );
    }
    ui.painter().rect_stroke(
        sw,
        0.0,
        egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
        egui::StrokeKind::Inside,
    );

    if checked {
        ui.painter().rect_stroke(
            rect.shrink(1.0),
            0.0,
            egui::Stroke::new(1.0, Color32::from(theme.accent)),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        Pos2::new(rect.left() + 42.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// Línea divisoria de menú, con márgenes a los lados.
pub fn menu_sep(ui: &mut Ui, theme: &Theme) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, 7.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.left() + 8.0, rect.center().y),
            Pos2::new(rect.right() - 8.0, rect.center().y),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    );
}

// ------------------------------------------------------------------ diálogos

/// El marco de un diálogo: sin la barra de título de egui, que mide el doble y
/// centra el texto como un cartel.
pub fn dialog_frame(theme: &Theme) -> egui::Frame {
    egui::Frame::NONE
        .fill(Color32::from(theme.surface))
        .stroke(egui::Stroke::new(1.0, Color32::from(theme.border_strong)))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 28,
            spread: 0,
            color: Color32::from_black_alpha(56),
        })
}

/// Cabecera con título a la izquierda y cerrar a la derecha. Devuelve `true` si
/// se tocó la cruz.
pub fn dialog_header(ui: &mut Ui, theme: &Theme, w: f32, title: &str) -> bool {
    let (hd, _) = ui.allocate_exact_size(vec2(w, 34.0), Sense::hover());
    ui.painter().rect_filled(hd, 0.0, Color32::from(theme.surface_alt));
    ui.painter().line_segment(
        [
            Pos2::new(hd.left(), hd.bottom() - 0.5),
            Pos2::new(hd.right(), hd.bottom() - 0.5),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    );
    ui.painter().text(
        Pos2::new(hd.left() + 12.0, hd.center().y),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(theme.font_size + 0.5),
        theme.text.into(),
    );
    let xr = Rect::from_center_size(Pos2::new(hd.right() - 19.0, hd.center().y), vec2(26.0, 26.0));
    let resp = ui.interact(xr, ui.id().with(("cerrar", title)), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(xr, 2.0, Color32::from(theme.button_hover));
    }
    let s = egui::Stroke::new(1.2, Color32::from(theme.text_dim));
    ui.painter()
        .line_segment([xr.center() + vec2(-4.5, -4.5), xr.center() + vec2(4.5, 4.5)], s);
    ui.painter()
        .line_segment([xr.center() + vec2(4.5, -4.5), xr.center() + vec2(-4.5, 4.5)], s);
    resp.clicked()
}

/// Pie con los botones a la derecha, el principal en color pleno. Devuelve
/// `Some(true)` si se aceptó y `Some(false)` si se canceló.
///
/// Con los dos botones iguales y a la izquierda no se sabía cuál era la acción.
pub fn dialog_footer(
    ui: &mut Ui,
    theme: &Theme,
    w: f32,
    primary: &str,
    secondary: Option<&str>,
) -> Option<bool> {
    let (ft, _) = ui.allocate_exact_size(vec2(w, 48.0), Sense::hover());
    ui.painter().rect_filled(ft, 0.0, Color32::from(theme.surface_alt));
    ui.painter().line_segment(
        [
            Pos2::new(ft.left(), ft.top() + 0.5),
            Pos2::new(ft.right(), ft.top() + 0.5),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    );

    let mut out = None;
    let mut x = ft.right() - 12.0;
    for (label, is_primary) in [(Some(primary), true), (secondary, false)] {
        let Some(label) = label else { continue };
        let bw = (label.chars().count() as f32 * theme.font_size * 0.62 + 32.0).max(84.0);
        let r = Rect::from_min_size(Pos2::new(x - bw, ft.center().y - 14.0), vec2(bw, 28.0));
        x -= bw + 8.0;

        let resp = ui.interact(r, ui.id().with(("pie", label)), Sense::click());
        let (bg, fg, border) = if is_primary {
            (
                Color32::from(theme.accent),
                Color32::from(theme.accent_text),
                Color32::from(theme.accent),
            )
        } else if resp.hovered() {
            (
                Color32::from(theme.button_hover),
                Color32::from(theme.text),
                Color32::from(theme.border_strong),
            )
        } else {
            (
                Color32::from(theme.surface),
                Color32::from(theme.text),
                Color32::from(theme.border),
            )
        };
        ui.painter().rect_filled(r, theme.button_rounding, bg);
        ui.painter().rect_stroke(
            r,
            theme.button_rounding,
            egui::Stroke::new(1.0, border),
            egui::StrokeKind::Inside,
        );
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            label,
            FontId::proportional(theme.font_size),
            fg,
        );
        if resp.clicked() {
            out = Some(is_primary);
        }
    }
    out
}

/// Una fila de "etiqueta a la izquierda, valor a la derecha", para los datos
/// que sólo se leen.
pub fn info_row(ui: &mut Ui, theme: &Theme, w: f32, label: &str, value: &str) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 24.0), Sense::hover());
    ui.painter().text(
        Pos2::new(r.left(), r.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text_dim.into(),
    );
    ui.painter().text(
        Pos2::new(r.right(), r.center().y),
        Align2::RIGHT_CENTER,
        value,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
}

// ------------------------------------------------------------- los chromes

pub struct UiOut {
    pub cmds: Vec<Cmd>,
    /// Cuál de los dos colores está seleccionado para editar (true = Color 1).
    pub picking_c1: bool,
    pub open_color_dialog: bool,
    pub open_file_menu: bool,
    pub open_brushes: bool,
    pub open_outline_menu: bool,
    pub open_fill_menu: bool,
    pub open_rotate_menu: bool,
    pub open_size_menu: bool,
    pub open_theme_menu: bool,
    /// Qué tema está puesto, para mostrarlo en el botón y marcarlo en el menú.
    pub theme_idx: usize,
    pub open_select_menu: bool,
    pub open_paste_menu: bool,
    /// Debajo de qué botón hay que anclar el menú que se acaba de abrir.
    pub menu_anchor: Pos2,
    pub set_tab: Option<Tab>,
    // Estado que la cinta necesita *mostrar*: entra por `UiIn` y sale igual.
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_status: bool,
    pub show_thumbnail: bool,
    /// Qué botones lleva la barra de acceso rápido, y dónde va.
    pub qat: [bool; ALL_QAT.len()],
    pub qat_below: bool,
    pub ribbon_min: bool,
    pub open_qat_menu: bool,
    /// Abrir Archivo directamente en Configuración. En el chrome de macOS es
    /// la **única** puerta: ahí los menús son del sistema y no hay barra
    /// propia donde poner Archivo.
    pub open_settings: bool,
    /// Los diez colores guardados; entra por `UiIn` y sale igual.
    pub custom: [Option<Color32>; 10],
}

/// Lo que la aplicación le cuenta a la cinta sobre su estado actual. Sin esto,
/// las casillas del panel Ver no sabrían si van marcadas.
#[derive(Clone, Copy, Default)]
pub struct UiIn {
    pub tab: Tab,
    pub picking_c1: bool,
    pub show_rulers: bool,
    pub show_grid: bool,
    pub show_status: bool,
    pub show_thumbnail: bool,
    pub qat: [bool; ALL_QAT.len()],
    pub qat_below: bool,
    pub ribbon_min: bool,
    pub theme_idx: usize,
    /// Los diez colores guardados, para la fila de abajo de la paleta.
    pub custom: [Option<Color32>; 10],
}

impl UiOut {
    pub fn new(i: UiIn) -> Self {
        Self {
            cmds: Vec::new(),
            picking_c1: i.picking_c1,
            open_color_dialog: false,
            open_file_menu: false,
            open_brushes: false,
            open_outline_menu: false,
            open_fill_menu: false,
            open_rotate_menu: false,
            open_size_menu: false,
            open_theme_menu: false,
            theme_idx: i.theme_idx,
            custom: i.custom,
            open_select_menu: false,
            open_paste_menu: false,
            menu_anchor: Pos2::ZERO,
            set_tab: None,
            show_rulers: i.show_rulers,
            show_grid: i.show_grid,
            show_status: i.show_status,
            show_thumbnail: i.show_thumbnail,
            qat: i.qat,
            qat_below: i.qat_below,
            ribbon_min: i.ribbon_min,
            open_qat_menu: false,
            open_settings: false,
        }
    }
}

pub fn chrome(
    ui: &mut Ui,
    doc: &mut Doc,
    theme: &Theme,
    themes: &[(String, usize)],
    i: UiIn,
    text: Option<&mut TextBox>,
) -> UiOut {
    let mut out = UiOut::new(i);
    match theme.chrome {
        Chrome::Ribbon => ribbon(ui, doc, theme, themes, &mut out, i.tab, text),
        Chrome::Palette => palette_chrome(ui, doc, theme, themes, &mut out),
        Chrome::Mac => mac_chrome(ui, doc, theme, &mut out),
        Chrome::Gnome => gnome_chrome(ui, doc, theme, &mut out),
        Chrome::Kde => kde_chrome(ui, doc, theme, themes, &mut out),
        Chrome::Studio => studio(ui, doc, theme, &mut out, text),
        Chrome::Neon => neon_chrome(ui, doc, theme, &mut out),
        // SW no crea ni un panel: la consola y el lector **flotan** sobre el
        // lienzo, así que se dibujan después del área de dibujo y no acá.
        // Un panel les daría el borde de la ventana, que es justo lo que no
        // tienen que tocar.
        Chrome::Holo => {}
    }
    out
}

/// Qué pestaña de la cinta está activa.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Home,
    View,
    /// Contextual: sólo existe mientras hay un cuadro de texto abierto, y la
    /// cinta salta sola a ella. Así los controles de fuente no ocupan lugar en
    /// Inicio el resto del tiempo.
    Text,
}

/// Una columna de alto declarado. Es la pieza que faltaba: dejando que cada
/// grupo fluyera solo, las columnas de botones chicos quedaban pegadas arriba
/// en vez de centradas. Con un tamaño fijo, el layout del padre las centra.
fn column(ui: &mut Ui, w: f32, h: f32, add: impl FnOnce(&mut Ui)) {
    ui.allocate_ui_with_layout(vec2(w, h), egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.spacing_mut().item_spacing.y = 2.0;
        add(ui);
    });
}

/// Botón chico de una fila: icono a la izquierda, texto a la derecha. Es la
/// forma de "Recortar", "Cambiar tamaño" y las entradas del panel Ver.
fn row_button(ui: &mut Ui, theme: &Theme, label: &str, icon: Icon, arrow: bool) -> bool {
    let w = label.chars().count() as f32 * theme.font_size * 0.56
        + if icon == Icon::None { 12.0 } else { 26.0 }
        + if arrow { 12.0 } else { 0.0 };
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 19.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
    }
    let mut x = rect.left() + 3.0;
    if icon != Icon::None {
        let ir = Rect::from_center_size(Pos2::new(x + 8.0, rect.center().y), vec2(16.0, 16.0));
        draw_icon(ui, ir, icon, theme.icon.into());
        x += 20.0;
    }
    ui.painter().text(
        Pos2::new(x, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    if arrow {
        caret(ui, Pos2::new(rect.right() - 7.0, rect.center().y), theme.text.into());
    }
    resp.clicked()
}

/// Casilla de verificación con etiqueta, para el panel Ver.
/// Una casilla de diálogo. Es `check_row` pero pública y con el alto de los
/// diálogos, no el de un menú.
pub fn dlg_check(ui: &mut Ui, theme: &Theme, label: &str, on: bool) -> bool {
    let w = label.chars().count() as f32 * theme.font_size * 0.58 + 30.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    let b = Rect::from_center_size(Pos2::new(rect.left() + 8.0, rect.center().y), vec2(14.0, 14.0));
    ui.painter().rect_filled(b, 2.0, Color32::from(theme.surface));
    ui.painter().rect_stroke(
        b,
        2.0,
        egui::Stroke::new(
            1.0,
            Color32::from(if resp.hovered() { theme.accent } else { theme.border_strong }),
        ),
        egui::StrokeKind::Inside,
    );
    if on {
        let s = egui::Stroke::new(2.0, Color32::from(theme.accent));
        let mid = Pos2::new(b.center().x - 0.5, b.bottom() - 3.5);
        ui.painter().line_segment([Pos2::new(b.left() + 3.0, b.center().y), mid], s);
        ui.painter().line_segment([mid, Pos2::new(b.right() - 3.0, b.top() + 3.5)], s);
    }
    ui.painter().text(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// Un botón de opción. Excluyente: el punto se llena, no se tilda.
///
/// La forma importa y no es un gusto: redondo quiere decir «una de estas» y
/// cuadrado «sí o no». Con los dos iguales hay que probar para saber cuál es.
pub fn dlg_radio(ui: &mut Ui, theme: &Theme, label: &str, on: bool) -> bool {
    let w = label.chars().count() as f32 * theme.font_size * 0.58 + 30.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    let c = Pos2::new(rect.left() + 8.0, rect.center().y);
    ui.painter().circle_filled(c, 7.0, Color32::from(theme.surface));
    ui.painter().circle_stroke(
        c,
        7.0,
        egui::Stroke::new(
            1.0,
            Color32::from(if resp.hovered() { theme.accent } else { theme.border_strong }),
        ),
    );
    if on {
        ui.painter().circle_filled(c, 3.5, Color32::from(theme.accent));
    }
    ui.painter().text(
        Pos2::new(rect.left() + 22.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// La línea que separa dos bloques de un diálogo.
pub fn dlg_sep(ui: &mut Ui, theme: &Theme, w: f32) {
    let (r, _) = ui.allocate_exact_size(vec2(w, 17.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(r.left(), r.center().y),
            Pos2::new(r.right(), r.center().y),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    );
}

/// Una fila de campo: la etiqueta en una columna fija y el control alineado a
/// la derecha de ella.
///
/// La columna fija es todo el punto: con las etiquetas pegadas a su campo,
/// «Horizontal» y «Vertical» dejaban los dos números en sitios distintos y la
/// columna quedaba dentada.
pub fn dlg_field(
    ui: &mut Ui,
    theme: &Theme,
    label: &str,
    valor: &mut f32,
    rango: std::ops::RangeInclusive<f32>,
    sufijo: &str,
) -> bool {
    let mut cambio = false;
    ui.horizontal(|ui| {
        let (r, _) = ui.allocate_exact_size(vec2(84.0, 24.0), Sense::hover());
        ui.painter().text(
            Pos2::new(r.left(), r.center().y),
            Align2::LEFT_CENTER,
            label,
            FontId::proportional(theme.font_size),
            theme.text.into(),
        );
        cambio = ui
            .add_sized(
                vec2(88.0, 24.0),
                egui::DragValue::new(valor).range(rango).suffix(sufijo),
            )
            .changed();
    });
    cambio
}

fn check_row(ui: &mut Ui, theme: &Theme, label: &str, on: bool) -> bool {
    let w = label.chars().count() as f32 * theme.font_size * 0.56 + 28.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 19.0), Sense::click());
    if resp.hovered() {
        ui.painter().rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
    }
    let b = Rect::from_center_size(Pos2::new(rect.left() + 11.0, rect.center().y), vec2(12.0, 12.0));
    ui.painter().rect_filled(b, 1.0, Color32::from(theme.surface));
    ui.painter().rect_stroke(
        b,
        1.0,
        egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
        egui::StrokeKind::Inside,
    );
    if on {
        let s = egui::Stroke::new(1.8, Color32::from(theme.icon));
        let mid = Pos2::new(b.center().x - 0.5, b.bottom() - 3.0);
        ui.painter().line_segment([Pos2::new(b.left() + 2.5, b.center().y), mid], s);
        ui.painter().line_segment([mid, Pos2::new(b.right() - 2.5, b.top() + 3.0)], s);
    }
    ui.painter().text(
        Pos2::new(rect.left() + 21.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// Chrome de Windows 7, 10 y 11: la cinta.
fn ribbon(
    ui: &mut Ui,
    doc: &mut Doc,
    theme: &Theme,
    themes: &[(String, usize)],
    out: &mut UiOut,
    tab: Tab,
    text: Option<&mut TextBox>,
) {    // Barra de acceso rápido: icono de la app, los botones que el usuario haya
    // elegido, y el desplegable para personalizarla.
    let qat_h = 26.0;
    let draw_qat = |ui: &mut Ui, out: &mut UiOut, doc: &Doc| {
        let r = ui.max_rect();
        gradient_bar(ui.painter(), r, theme.bar_top.into(), theme.bar_bottom.into());
        ui.allocate_ui_with_layout(r.size(), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.add_space(5.0);

            let (ir, _) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::hover());
            app_icon(ui, ir, theme.icon.into());
            ui.add_space(3.0);
            ui.painter().line_segment(
                [
                    Pos2::new(ui.cursor().left(), r.top() + 5.0),
                    Pos2::new(ui.cursor().left(), r.bottom() - 5.0),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.add_space(4.0);

            for (idx, item) in ALL_QAT.iter().enumerate() {
                if !out.qat[idx] {
                    continue;
                }
                let enabled = match item {
                    Qat::Undo => doc.canvas.can_undo(),
                    Qat::Redo => doc.canvas.can_redo(),
                    _ => true,
                };
                let (rect, resp) = ui.allocate_exact_size(vec2(21.0, 21.0), Sense::click());
                // Apagado es el mismo dibujo transparentado, no otro color:
                // con `text_dim` cambiaba de tono además de apagarse, y en un
                // tema oscuro rehacer quedaba de un gris que no era de nadie.
                let col: Color32 = Color32::from(theme.icon)
                    .gamma_multiply(if enabled { 1.0 } else { 0.32 });
                if enabled && resp.hovered() {
                    ui.painter().rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
                    ui.painter().rect_stroke(
                        rect,
                        theme.button_rounding,
                        egui::Stroke::new(1.0, Color32::from(theme.accent)),
                        egui::StrokeKind::Inside,
                    );
                }
                qat_icon(ui, rect.shrink(4.0), *item, col);
                let resp = resp.on_hover_text(item.label());
                if enabled && resp.clicked() {
                    out.cmds.push(item.cmd());
                }
            }

            // El desplegable de personalizar.
            let (rect, resp) = ui.allocate_exact_size(vec2(16.0, 21.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, theme.button_rounding, Color32::from(theme.button_hover));
            }
            caret(ui, rect.center(), theme.text.into());
            if resp.clicked() {
                out.menu_anchor = rect.left_bottom();
                out.open_qat_menu = true;
            }
            let _ = resp.on_hover_text(lang::t("Personalizar barra de herramientas de acceso rápido"));
        });
    };

    if !out.qat_below {
        egui::Panel::top("qat")
            .exact_size(qat_h)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| draw_qat(ui, out, doc));
    }

    // Las pestañas. Archivo va en color pleno, como en Windows.
    egui::Panel::top("tabs")
        .exact_size(24.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.painter().rect_filled(full, 0.0, Color32::from(theme.window));
            ui.allocate_ui_with_layout(full.size(), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (rect, resp) = ui.allocate_exact_size(vec2(58.0, 24.0), Sense::click());
                ui.painter().rect_filled(rect, 0.0, Color32::from(theme.file_tab));
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    lang::t("Archivo"),
                    FontId::proportional(theme.font_size),
                    theme.file_tab_text.into(),
                );
                if resp.clicked() {
                    out.open_file_menu = true;
                }
                // La contextual sólo aparece con el cuadro de texto abierto.
                let mut tabs: Vec<(&str, Tab)> = vec![(lang::t("Inicio"), Tab::Home), (lang::t("Ver"), Tab::View)];
                if tab == Tab::Text {
                    tabs.push((lang::t("Texto"), Tab::Text));
                }
                for (label, t) in tabs {
                    let w = label.chars().count() as f32 * theme.font_size * 0.62 + 22.0;
                    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
                    if tab == t {
                        ui.painter().rect_filled(rect, 0.0, Color32::from(theme.ribbon_tab_active));
                    } else if resp.hovered() {
                        ui.painter().rect_filled(rect, 0.0, Color32::from(theme.button_hover));
                    }
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        label,
                        FontId::proportional(theme.font_size),
                        theme.text.into(),
                    );
                    if resp.clicked() {
                        out.set_tab = Some(t);
                    }
                }
            });
        });

    // La banda. Alto fijo a propósito: nada de reflow.
    if !out.ribbon_min {
    egui::Panel::top("ribbon")
        .exact_size(theme.ribbon_height)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let full = ui.max_rect();
            ui.painter().rect_filled(full, 0.0, Color32::from(theme.ribbon));
            ui.painter().line_segment(
                [
                    Pos2::new(full.left(), full.bottom() - 0.5),
                    Pos2::new(full.right(), full.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            // 16 px al pie para las etiquetas de grupo.
            let band = Rect::from_min_max(full.min, Pos2::new(full.max.x, full.max.y - 16.0));
            ui.allocate_ui_with_layout(band.size(), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.add_space(6.0);
                match tab {
                    Tab::Home => home_tab(ui, doc, theme, band, out),
                    Tab::View => view_tab(ui, theme, band, themes, out),
                    // Sin cuadro no hay nada que configurar: se cae a Inicio.
                    Tab::Text => match text {
                        Some(tb) => text_tab(ui, doc, theme, band, tb, out),
                        None => home_tab(ui, doc, theme, band, out),
                    },
                }
            });
        });
    }

    if out.qat_below {
        egui::Panel::top("qat_below")
            .exact_size(qat_h)
            .frame(egui::Frame::NONE)
            .show(ui, |ui| draw_qat(ui, out, doc));
    }
}

fn home_tab(ui: &mut Ui, doc: &mut Doc, theme: &Theme, band: Rect, out: &mut UiOut) {
    ribbon_group(ui, theme, band, lang::t("Portapapeles"), |ui| {
        let (half, r) = big_split(ui, theme, lang::t("Pegar"), Icon::I(Ico::Paste), false);
        match half {
            Some(Half::Main) => out.cmds.push(Cmd::Paste),
            Some(Half::Caret) => {
                out.menu_anchor = r.left_bottom();
                out.open_paste_menu = true;
            }
            None => {}
        }
        column(ui, 64.0, 42.0, |ui| {
            if row_button(ui, theme, lang::t("Cortar"), Icon::I(Ico::Cut), false) {
                out.cmds.push(Cmd::Cut);
            }
            if row_button(ui, theme, lang::t("Copiar"), Icon::I(Ico::Copy), false) {
                out.cmds.push(Cmd::Copy);
            }
        });
    });

    ribbon_group(ui, theme, band, lang::t("Imagen"), |ui| {
        let (half, r) = big_split(ui, theme, lang::t("Seleccionar"), Icon::T(Tool::Select), doc.tool == Tool::Select);
        match half {
            Some(Half::Main) => doc.set_tool(Tool::Select),
            Some(Half::Caret) => {
                out.menu_anchor = r.left_bottom();
                out.open_select_menu = true;
            }
            None => {}
        }
        // Girar va en esta misma columna, no en una aparte: separarla ensanchaba
        // el grupo al doble de lo que mide en Paint.
        column(ui, 128.0, 84.0, |ui| {
            if row_button(ui, theme, lang::t("Selección"), Icon::T(Tool::Select), true) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_select_menu = true;
            }
            if row_button(ui, theme, lang::t("Recortar"), Icon::I(Ico::Crop), false) {
                out.cmds.push(Cmd::Crop);
            }
            if row_button(ui, theme, lang::t("Cambiar tamaño"), Icon::I(Ico::Resize), false) {
                out.cmds.push(Cmd::ResizeDialog);
            }
            if row_button(ui, theme, lang::t("Girar"), Icon::I(Ico::Rotate), true) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_rotate_menu = true;
            }
        });
    });

    ribbon_group(ui, theme, band, lang::t("Herramientas"), |ui| {
        // Grilla 3x2, como en Paint.
        column(ui, 80.0, 52.0, |ui| {
            for row in [
                [Tool::Pencil, Tool::Fill, Tool::Text],
                [Tool::Eraser, Tool::Picker, Tool::Magnifier],
            ] {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for t in row {
                        if tool_button(ui, theme, t, doc.tool == t) {
                            doc.set_tool(t);
                        }
                    }
                });
            }
        });
    });

    ribbon_group(ui, theme, band, lang::t("Pinceles"), |ui| {
        let (half, r) = big_split(ui, theme, lang::t("Pinceles"), Icon::T(Tool::Brush), doc.tool == Tool::Brush);
        match half {
            Some(Half::Main) => doc.set_tool(Tool::Brush),
            Some(Half::Caret) => {
                out.menu_anchor = r.left_bottom();
                out.open_brushes = true;
            }
            None => {}
        }
    });

    ribbon_group(ui, theme, band, lang::t("Formas"), |ui| {
        // 8 por fila y 3 filas, como la galería original. La barra va aparte y
        // pintada a mano: la de egui es del ancho de un pelo sobre un fondo
        // claro y con sesenta formas nadie se entera de que hay más abajo.
        column(ui, 186.0, 68.0, |ui| {
            egui::ScrollArea::vertical()
                .max_height(68.0)
                .id_salt("shapes")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
                .show(ui, |ui| {
                    for chunk in ALL_SHAPES.chunks(8) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            for sh in chunk {
                                let active = doc.tool == Tool::Shape && doc.shape == *sh;
                                if shape_button(ui, theme, *sh, active) {
                                    doc.shape = *sh;
                                    doc.set_tool(Tool::Shape);
                                }
                            }
                        });
                    }
                });
        });
        column(ui, 100.0, 44.0, |ui| {
            if row_button(ui, theme, lang::t("Contorno"), Icon::None, true) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_outline_menu = true;
            }
            if row_button(ui, theme, lang::t("Relleno"), Icon::None, true) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_fill_menu = true;
            }
        });
    });

    ribbon_group(ui, theme, band, lang::t("Tamaño"), |ui| {
        size_button(ui, theme, doc, out);
    });

    // Último grupo: sin separador a la derecha, no hay nada que separar.
    ribbon_group_last(ui, theme, band, lang::t("Colores"), |ui| {
        color_boxes(ui, theme, doc, out);
        ui.add_space(4.0);
        if let Some(hit) = palette_grid(ui, theme, &out.custom) {
            apply_palette_hit(hit, doc, out);
        }
        ui.add_space(4.0);
        if big_plain(ui, theme, lang::t("Editar\ncolores"), Icon::I(Ico::Spectrum), false) {
            out.open_color_dialog = true;
        }
    });
}

/// La pestaña contextual Texto: fuente, fondo y colores.
///
/// `ponytail:` sólo la dibuja el chrome de cinta. En los temas de barra clásica
/// (XP, Linux, macOS) el cuadro de texto usa los valores por defecto; darles
/// controles propios es sumarles un menú, no cambiar esto.
fn text_tab(ui: &mut Ui, doc: &mut Doc, theme: &Theme, band: Rect, tb: &mut TextBox, out: &mut UiOut) {
    ribbon_group(ui, theme, band, lang::t("Fuente"), |ui| {
        column(ui, 210.0, 52.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                egui::ComboBox::from_id_salt("familia")
                    .selected_text(crate::text::FAMILIES[tb.family])
                    .width(130.0)
                    .show_ui(ui, |ui| {
                        for (i, name) in crate::text::FAMILIES.iter().enumerate() {
                            ui.selectable_value(&mut tb.family, i, *name);
                        }
                    });
                egui::ComboBox::from_id_salt("tamanio")
                    .selected_text(format!("{}", tb.size as i32))
                    .width(58.0)
                    .show_ui(ui, |ui| {
                        for sz in crate::text::SIZES {
                            ui.selectable_value(&mut tb.size, sz, format!("{}", sz as i32));
                        }
                    });
            });
        });
    });

    ribbon_group(ui, theme, band, lang::t("Fondo"), |ui| {
        // Los dos son excluyentes, así que se marcan como tal: nada de una
        // casilla que hay que deducir si está puesta o no.
        if big_plain(ui, theme, lang::t("Trans-\nparente"), Icon::I(Ico::Spectrum), !tb.opaque) {
            tb.opaque = false;
        }
        if big_plain(ui, theme, lang::t("Opaco"), Icon::I(Ico::Palette), tb.opaque) {
            tb.opaque = true;
        }
    });

    ribbon_group_last(ui, theme, band, lang::t("Colores"), |ui| {
        color_boxes(ui, theme, doc, out);
        ui.add_space(4.0);
        if let Some(hit) = palette_grid(ui, theme, &out.custom) {
            apply_palette_hit(hit, doc, out);
        }
    });
}

/// La pestaña Ver: zoom, qué mostrar y pantalla.
fn view_tab(ui: &mut Ui, theme: &Theme, band: Rect, themes: &[(String, usize)], out: &mut UiOut) {
    ribbon_group(ui, theme, band, lang::t("Zoom"), |ui| {
        for (label, ico, cmd) in [
            ("Acercar", Ico::ZoomIn, Cmd::ZoomIn),
            ("Alejar", Ico::ZoomOut, Cmd::ZoomOut),
            ("100%", Ico::Zoom100, Cmd::Zoom100),
        ] {
            if big_plain(ui, theme, label, Icon::I(ico), false) {
                out.cmds.push(cmd);
            }
        }
    });

    ribbon_group(ui, theme, band, lang::t("Mostrar u ocultar"), |ui| {
        column(ui, 136.0, 66.0, |ui| {
            for (label, on, cmd) in [
                ("Reglas", out.show_rulers, Cmd::ToggleRulers),
                ("Cuadrícula", out.show_grid, Cmd::ToggleGrid),
                ("Barra de estado", out.show_status, Cmd::ToggleStatusBar),
            ] {
                if check_row(ui, theme, label, on) {
                    out.cmds.push(cmd);
                }
            }
        });
    });

    ribbon_group(ui, theme, band, lang::t("Pantalla"), |ui| {
        if big_plain(ui, theme, lang::t("Pantalla\ncompleta"), Icon::I(Ico::FullScreen), false) {
            out.cmds.push(Cmd::FullScreen);
        }
        if big_plain(ui, theme, lang::t("Miniatura"), Icon::I(Ico::Thumbnail), out.show_thumbnail) {
            out.cmds.push(Cmd::ToggleThumbnail);
        }
    });

    // El tema es **un** botón con el nombre del actual, no una lista de seis:
    // apiladas no entran en la banda, se desbordaban y pisaban la etiqueta del
    // grupo. Y así se ve cuál está puesto, que antes no se notaba.
    ribbon_group(ui, theme, band, lang::t("Tema"), |ui| {
        // El botón muestra la **familia** puesta, no la variante: si dice
        // «Windows 10 oscuro» está repitiendo lo que ya dice el interruptor.
        let actual = themes
            .iter()
            .find(|(_, i)| *i == out.theme_idx)
            .map(|(n, _)| n.clone())
            .unwrap_or_else(|| lang::t("Tema").to_string());
        let (half, r) = big_split(ui, theme, &actual, Icon::I(Ico::Palette), true);
        if half.is_some() {
            out.menu_anchor = r.left_bottom();
            out.open_theme_menu = true;
        }
    });
}

/// Como `ribbon_group` pero sin el separador: para el último de la fila, donde
/// no hay nada que separar.
fn ribbon_group_last(ui: &mut Ui, theme: &Theme, band: Rect, label: &str, add: impl FnOnce(&mut Ui)) {
    let start = ui.cursor().left();
    add(ui);
    let end = ui.cursor().left();
    ui.painter().text(
        Pos2::new((start + end) / 2.0, band.bottom() + 8.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(theme.font_size - 1.0),
        theme.ribbon_group_label.into(),
    );
    ui.add_space(9.0);
}

/// Un grupo de la cinta: contenido, etiqueta gris debajo y separador vertical.
///
/// `band` es el rectángulo real de la banda y hay que pasarlo: `ui.max_rect()`
/// dentro de un `horizontal()` mide sólo `interact_size.y` (18 px), no el alto
/// del panel, así que la etiqueta caía sobre los iconos y el separador salía
/// con el extremo inferior por encima del superior — invisible.
fn ribbon_group(ui: &mut Ui, theme: &Theme, band: Rect, label: &str, add: impl FnOnce(&mut Ui)) {
    let start = ui.cursor().left();
    add(ui);
    let end = ui.cursor().left();

    // La etiqueta va en la franja de 16 px que reservamos al pie de la banda.
    ui.painter().text(
        Pos2::new((start + end) / 2.0, band.bottom() + 8.0),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(theme.font_size - 1.0),
        theme.ribbon_group_label.into(),
    );

    let x = end + 4.0;
    ui.painter().line_segment(
        [
            Pos2::new(x, band.top() + 6.0),
            Pos2::new(x, band.bottom() - 6.0),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.ribbon_separator)),
    );
    ui.add_space(7.0);
}

/// Chrome de Windows XP y 98: herramientas en grilla a la izquierda.
/// Un botón de la caja de Paint.
///
/// En Paint las figuras y los modos de selección **son herramientas sueltas**,
/// no una herramienta con un desplegable: la caja tiene dieciséis botones y en
/// eso se le va media pantalla. Acá cada uno pone la herramienta y además la
/// figura, el pincel o el modo que le corresponde.
#[derive(Clone, Copy, PartialEq)]
enum XpPick {
    Simple(Tool),
    Seleccion(SelectMode),
    Pincel(Brush),
    Figura(Shape),
}

/// La caja de Paint, leída por filas: dos columnas y ocho filas.
const XP_TOOLS: [XpPick; 16] = [
    XpPick::Seleccion(SelectMode::FreeForm), XpPick::Seleccion(SelectMode::Rectangular),
    XpPick::Simple(Tool::Eraser),            XpPick::Simple(Tool::Fill),
    XpPick::Simple(Tool::Picker),            XpPick::Simple(Tool::Magnifier),
    XpPick::Simple(Tool::Pencil),            XpPick::Pincel(Brush::Round),
    XpPick::Pincel(Brush::Airbrush),         XpPick::Simple(Tool::Text),
    XpPick::Figura(Shape::Line),             XpPick::Figura(Shape::Curve),
    XpPick::Figura(Shape::Rectangle),        XpPick::Figura(Shape::Polygon),
    XpPick::Figura(Shape::Oval),             XpPick::Figura(Shape::RoundedRect),
];

impl XpPick {
    /// Sólo uno de los dieciséis puede estar hundido: el estado se mira entero,
    /// herramienta **y** ajuste. Comparando sólo la herramienta se encenderían
    /// las seis figuras a la vez.
    fn activo(self, doc: &Doc) -> bool {
        match self {
            Self::Simple(t) => doc.tool == t,
            Self::Seleccion(m) => doc.tool == Tool::Select && doc.select_mode == m,
            Self::Pincel(b) => doc.tool == Tool::Brush && doc.brush == b,
            Self::Figura(f) => doc.tool == Tool::Shape && doc.shape == f,
        }
    }

    fn aplicar(self, doc: &mut Doc) {
        match self {
            Self::Simple(t) => doc.set_tool(t),
            Self::Seleccion(m) => {
                doc.select_mode = m;
                doc.set_tool(Tool::Select);
            }
            Self::Pincel(b) => {
                doc.brush = b;
                doc.set_tool(Tool::Brush);
            }
            Self::Figura(f) => {
                doc.shape = f;
                doc.set_tool(Tool::Shape);
            }
        }
    }

    fn dibujar(self, ui: &Ui, r: Rect, col: Color32) {
        match self {
            Self::Simple(t) => tool_icon(ui, r, t, col),
            Self::Seleccion(SelectMode::Rectangular) => tool_icon(ui, r, Tool::Select, col),
            // El lazo: la selección libre se dibuja como un lazo, no como el
            // mismo rectángulo punteado que su vecina.
            Self::Seleccion(SelectMode::FreeForm) => {
                let p = ui.painter();
                let s = egui::Stroke::new(1.5, col);
                let pt = |fx: f32, fy: f32| {
                    Pos2::new(r.left() + fx * r.width(), r.top() + fy * r.height())
                };
                p.add(egui::Shape::line(
                    vec![
                        pt(0.30, 0.86), pt(0.10, 0.60), pt(0.16, 0.28), pt(0.44, 0.12),
                        pt(0.76, 0.18), pt(0.90, 0.44), pt(0.78, 0.72), pt(0.52, 0.80),
                    ],
                    s,
                ));
                p.line_segment([pt(0.52, 0.80), pt(0.62, 0.94)], s);
            }
            Self::Pincel(b) => xp_brush_mark(ui, r, b, col),
            Self::Figura(f) => shape_icon(ui, r, f, col),
        }
    }

    fn nombre(self) -> &'static str {
        match self {
            Self::Simple(t) => t.label(),
            Self::Seleccion(SelectMode::FreeForm) => lang::t("Selección libre"),
            Self::Seleccion(SelectMode::Rectangular) => lang::t("Seleccionar"),
            Self::Pincel(Brush::Airbrush) => lang::t("Aerógrafo"),
            Self::Pincel(_) => lang::t("Pincel"),
            Self::Figura(f) => f.label(),
        }
    }
}

/// Hundido de Windows clásico: una sombra arriba y a la izquierda, y una luz
/// abajo y a la derecha. Son dos líneas de un píxel, y sin ellas ningún tema
/// noventoso se ve como tal.
fn sunken(ui: &Ui, r: Rect, theme: &Theme) {
    let p = ui.painter();
    let sombra = egui::Stroke::new(1.0, Color32::from(theme.border));
    let luz = egui::Stroke::new(1.0, Color32::WHITE);
    p.rect_filled(r, 0.0, Color32::from(theme.surface_alt));
    p.line_segment([r.left_bottom(), r.left_top()], sombra);
    p.line_segment([r.left_top(), r.right_top()], sombra);
    p.line_segment([r.right_top(), r.right_bottom()], luz);
    p.line_segment([r.right_bottom(), r.left_bottom()], luz);
}

/// Relieve de Windows clásico: la luz arriba y a la izquierda, la sombra abajo
/// y a la derecha. Es `sunken` al revés, y juntos son los dos únicos estados
/// que necesita un botón de esa época.
fn raised(ui: &Ui, r: Rect, theme: &Theme) {
    let p = ui.painter();
    let sombra = egui::Stroke::new(1.0, Color32::from(theme.border));
    let luz = egui::Stroke::new(1.0, Color32::WHITE);
    p.rect_filled(r, 0.0, Color32::from(theme.button));
    p.line_segment([r.left_bottom(), r.left_top()], luz);
    p.line_segment([r.left_top(), r.right_top()], luz);
    p.line_segment([r.right_top(), r.right_bottom()], sombra);
    p.line_segment([r.right_bottom(), r.left_bottom()], sombra);
}

/// La marca de un pincel dentro de la caja de opciones.
///
/// No es la vista previa del trazo de verdad —esa se rasteriza con el motor y
/// vive en la galería de la cinta—, sino el símbolo que lo distingue en once
/// píxeles: redondo, cuadrado, o la barra inclinada de los caligráficos.
fn xp_brush_mark(ui: &Ui, r: Rect, b: Brush, col: Color32) {
    let p = ui.painter();
    let c = r.center();
    let k = r.width();
    match b {
        Brush::Round => {
            p.circle_filled(c, k * 0.34, col);
        }
        Brush::Airbrush => {
            for (dx, dy) in [(0.0, 0.0), (-0.3, -0.2), (0.3, -0.25), (-0.25, 0.3), (0.28, 0.28)] {
                p.circle_filled(c + vec2(dx * k, dy * k), k * 0.09, col);
            }
        }
        Brush::Calligraphy1 => {
            p.line_segment([c + vec2(-k * 0.3, k * 0.3), c + vec2(k * 0.3, -k * 0.3)],
                egui::Stroke::new(k * 0.28, col));
        }
        Brush::Calligraphy2 => {
            p.line_segment([c + vec2(-k * 0.3, -k * 0.3), c + vec2(k * 0.3, k * 0.3)],
                egui::Stroke::new(k * 0.28, col));
        }
        Brush::Marker => {
            p.rect_filled(Rect::from_center_size(c, vec2(k * 0.62, k * 0.62)), 0.0, col);
        }
        Brush::Oil => {
            p.circle_filled(c, k * 0.42, col);
        }
        Brush::Crayon => {
            for i in 0..4 {
                let y = c.y + (i as f32 - 1.5) * k * 0.2;
                p.line_segment([Pos2::new(r.left(), y), Pos2::new(r.right(), y)],
                    egui::Stroke::new(k * 0.1, col));
            }
        }
        Brush::NaturalPencil => {
            p.line_segment([r.left_bottom(), r.right_top()], egui::Stroke::new(k * 0.14, col));
        }
        Brush::Watercolor => {
            p.circle_filled(c, k * 0.40, col.gamma_multiply(0.45));
            p.circle_filled(c, k * 0.2, col);
        }
    }
}


/// El color con el que se dibuja dentro de una celda.
fn xp_cell_ink(theme: &Theme, on: bool) -> Color32 {
    if on {
        Color32::from(theme.accent_text)
    } else {
        Color32::from(theme.text)
    }
}

/// La caja de opciones de Paint: **cambia entera con la herramienta**.
///
/// Es la mitad de la experiencia del programa original y era lo que faltaba.
/// En el Paint de verdad ahí aparecen los doce pinceles, los cuatro tamaños de
/// borrador, los niveles de la lupa, los tres estilos de figura y la selección
/// transparente; con el cuentagotas y el lápiz queda vacía, y eso también es
/// información: esa herramienta no configura nada.
fn xp_options(ui: &mut Ui, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    match doc.tool {
        Tool::Brush => {
            // Los pinceles en tres columnas, como la rejilla de Paint.
            let (caja, _) = ui.allocate_exact_size(vec2(46.0, 62.0), Sense::hover());
            sunken(ui, caja, theme);
            for (i, b) in ALL_BRUSHES.iter().enumerate() {
                let c = Rect::from_min_size(
                    Pos2::new(
                        caja.left() + 2.0 + (i % 3) as f32 * 14.0,
                        caja.top() + 2.0 + (i / 3) as f32 * 14.0,
                    ),
                    vec2(14.0, 14.0),
                );
                let on = doc.brush == *b;
                let resp = ui.interact(c, ui.id().with(("pincel", i)), Sense::click());
                if on {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.accent));
                } else if resp.hovered() {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.button_hover));
                }
                xp_brush_mark(ui, c.shrink(2.0), *b, xp_cell_ink(theme, on));
                if resp.clicked() {
                    doc.brush = *b;
                }
            }
        }

        Tool::Eraser => {
            let (caja, _) = ui.allocate_exact_size(vec2(46.0, 58.0), Sense::hover());
            sunken(ui, caja, theme);
            for (i, w) in ERASER_WIDTHS.iter().enumerate() {
                let c = Rect::from_min_size(
                    Pos2::new(caja.left() + 2.0, caja.top() + 2.0 + i as f32 * 13.5),
                    vec2(42.0, 13.5),
                );
                let on = (doc.width - w).abs() < 0.5;
                let resp = ui.interact(c, ui.id().with(("goma", i)), Sense::click());
                if on {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.accent));
                } else if resp.hovered() {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.button_hover));
                }
                // Un cuadrado del tamaño real: la goma de Paint es cuadrada.
                let lado = w * 1.1;
                ui.painter().rect_filled(
                    Rect::from_center_size(c.center(), vec2(lado, lado)),
                    0.0,
                    xp_cell_ink(theme, on),
                );
                if resp.clicked() {
                    doc.width = *w;
                }
            }
        }

        Tool::Magnifier => {
            let (caja, _) = ui.allocate_exact_size(vec2(46.0, 56.0), Sense::hover());
            sunken(ui, caja, theme);
            for (i, (etiqueta, cmd)) in [
                ("1x", Cmd::Zoom100),
                ("2x", Cmd::ZoomIn),
                ("4x", Cmd::ZoomIn),
                ("8x", Cmd::ZoomIn),
            ]
            .into_iter()
            .enumerate()
            {
                let c = Rect::from_min_size(
                    Pos2::new(caja.left() + 2.0, caja.top() + 2.0 + i as f32 * 13.0),
                    vec2(42.0, 13.0),
                );
                let resp = ui.interact(c, ui.id().with(("lupa", i)), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.button_hover));
                }
                ui.painter().text(
                    c.center(),
                    Align2::CENTER_CENTER,
                    etiqueta,
                    FontId::proportional(theme.font_size - 1.0),
                    theme.text.into(),
                );
                if resp.clicked() {
                    out.cmds.push(cmd);
                }
            }
        }

        Tool::Shape => {
            // Los tres estilos de figura de Paint: contorno, contorno con
            // relleno, y relleno solo.
            let (caja, _) = ui.allocate_exact_size(vec2(46.0, 56.0), Sense::hover());
            sunken(ui, caja, theme);
            let estilos = [
                (Stroke::Solid, Stroke::None),
                (Stroke::Solid, Stroke::Solid),
                (Stroke::None, Stroke::Solid),
            ];
            for (i, (borde, relleno)) in estilos.into_iter().enumerate() {
                let c = Rect::from_min_size(
                    Pos2::new(caja.left() + 2.0, caja.top() + 2.0 + i as f32 * 17.0),
                    vec2(42.0, 17.0),
                );
                let on = doc.outline == borde && doc.fill_style == relleno;
                let resp = ui.interact(c, ui.id().with(("figura", i)), Sense::click());
                if on {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.accent));
                } else if resp.hovered() {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.button_hover));
                }
                let tinta = xp_cell_ink(theme, on);
                let muestra = Rect::from_center_size(c.center(), vec2(24.0, 11.0));
                if relleno != Stroke::None {
                    ui.painter().rect_filled(muestra, 0.0, tinta);
                }
                if borde != Stroke::None {
                    ui.painter().rect_stroke(
                        muestra,
                        0.0,
                        egui::Stroke::new(1.5, tinta),
                        egui::StrokeKind::Inside,
                    );
                }
                if resp.clicked() {
                    doc.outline = borde;
                    doc.fill_style = relleno;
                }
            }
        }

        Tool::Select => {
            let (caja, _) = ui.allocate_exact_size(vec2(46.0, 40.0), Sense::hover());
            sunken(ui, caja, theme);
            for (i, transp) in [false, true].into_iter().enumerate() {
                let c = Rect::from_min_size(
                    Pos2::new(caja.left() + 2.0, caja.top() + 2.0 + i as f32 * 18.0),
                    vec2(42.0, 18.0),
                );
                let on = doc.transparent_selection == transp;
                let resp = ui.interact(c, ui.id().with(("transp", i)), Sense::click());
                if on {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.accent));
                } else if resp.hovered() {
                    ui.painter().rect_filled(c, 0.0, Color32::from(theme.button_hover));
                }
                // Dos rectángulos superpuestos: opaco tapa, transparente deja ver.
                let tinta = xp_cell_ink(theme, on);
                let atras = Rect::from_min_size(
                    Pos2::new(c.left() + 9.0, c.center().y - 6.0),
                    vec2(15.0, 11.0),
                );
                let frente = atras.translate(vec2(9.0, 3.0));
                ui.painter().rect_stroke(
                    atras,
                    0.0,
                    egui::Stroke::new(1.0, tinta),
                    egui::StrokeKind::Inside,
                );
                if !transp {
                    ui.painter().rect_filled(frente, 0.0, Color32::from(theme.surface));
                }
                ui.painter().rect_stroke(
                    frente,
                    0.0,
                    egui::Stroke::new(1.0, tinta),
                    egui::StrokeKind::Inside,
                );
                if resp.clicked() {
                    doc.transparent_selection = transp;
                }
            }
        }

        // Lápiz, relleno, texto y cuentagotas no configuran nada: la caja se va.
        // En Paint también desaparece, y eso dice algo tan claro como mostrarla.
        _ => {}
    }
}

/// El selector de grosor de XP: un recuadro hundido con las rayas dibujadas,
/// una debajo de la otra. Antes era una lista con los números «1 3 5 8», que
/// no dice nada: el grosor se elige mirándolo, no leyéndolo.
fn xp_widths(ui: &mut Ui, theme: &Theme, doc: &mut Doc) {
    let ws = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
    // 13 px por fila, no 15: en Paint este recuadro es un cuadradito discreto
    // debajo de las herramientas, no un panel del alto de la caja entera.
    const FILA: f32 = 13.0;
    let (caja, _) = ui.allocate_exact_size(
        vec2(46.0, FILA * ws.len() as f32 + 4.0),
        Sense::hover(),
    );
    sunken(ui, caja, theme);

    for (i, w) in ws.iter().enumerate() {
        let fila = Rect::from_min_size(
            Pos2::new(caja.left() + 2.0, caja.top() + 2.0 + i as f32 * FILA),
            vec2(caja.width() - 4.0, FILA),
        );
        let resp = ui.interact(fila, ui.id().with(("grosor", i)), Sense::click());
        let elegido = (doc.width - w).abs() < 0.5;
        if elegido {
            ui.painter().rect_filled(fila, 0.0, Color32::from(theme.accent));
        } else if resp.hovered() {
            ui.painter().rect_filled(fila, 0.0, Color32::from(theme.button_hover));
        }
        let col = if elegido {
            Color32::from(theme.accent_text)
        } else {
            Color32::from(theme.text)
        };
        ui.painter().line_segment(
            [
                Pos2::new(fila.left() + 5.0, fila.center().y),
                Pos2::new(fila.right() - 5.0, fila.center().y),
            ],
            egui::Stroke::new(*w, col),
        );
        if resp.clicked() {
            doc.width = *w;
        }
    }
}

/// Chrome de Windows XP y 98: barra de menú, columna de herramientas a la
/// izquierda y paleta abajo.
///
/// La barra de título la dibuja el sistema, no nosotros. Antes esta función
/// pintaba el degradado azul de XP **detrás de los menús**, y quedaba un menú
/// azul brillante que no existe en ningún Windows.
fn palette_chrome(ui: &mut Ui, doc: &mut Doc, theme: &Theme, themes: &[(String, usize)], out: &mut UiOut) {
    egui::Panel::top("xp_menu")
        .exact_size(23.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.bottom() - 0.5),
                    Pos2::new(r.right(), r.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // En Windows los renglones del menú son **texto suelto**:
                    // el recuadro aparece sólo al pasar por encima. Con el
                    // botón de fábrica cada uno salía metido en su cajita.
                    let w = &mut ui.style_mut().visuals.widgets;
                    w.inactive.weak_bg_fill = Color32::TRANSPARENT;
                    w.inactive.bg_stroke = egui::Stroke::NONE;
                    w.hovered.weak_bg_fill = Color32::from(theme.accent);
                    w.hovered.bg_stroke = egui::Stroke::NONE;
                    w.hovered.fg_stroke.color = Color32::from(theme.accent_text);
                    w.open.weak_bg_fill = Color32::from(theme.accent);
                    w.open.bg_stroke = egui::Stroke::NONE;
                    ui.spacing_mut().button_padding = vec2(7.0, 2.0);
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(2.0);
                    file_menu(ui, out);
                    edit_menu(ui, out);
                    image_menu(ui, out);
                    view_menu(ui, themes, out);
                    colors_menu(ui, out);
                    help_menu(ui, out);
                },
            );
        });

    // 2 columnas de 24 px más los márgenes: el ancho exacto de la caja de
    // herramientas de Paint, no un panel de 72 con las herramientas nadando.
    egui::Panel::left("xp_tools")
        .exact_size(56.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.painter().line_segment(
                [
                    Pos2::new(r.right() - 0.5, r.top()),
                    Pos2::new(r.right() - 0.5, r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing = vec2(1.0, 1.0);
                    ui.add_space(3.0);
                    for par in XP_TOOLS.chunks(2) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(1.0, 1.0);
                            for pick in par {
                                let on = pick.activo(doc);
                                let (rect, resp) = frame_button(ui, 24.0, on, theme);
                                pick.dibujar(ui, rect.shrink(4.0), theme.icon.into());
                                if resp.on_hover_text(pick.nombre()).clicked() {
                                    pick.aplicar(doc);
                                }
                            }
                        });
                    }
                    ui.add_space(7.0);
                    // El grosor sólo con las herramientas que trazan; el resto
                    // muestra sus propias opciones debajo.
                    if matches!(doc.tool, Tool::Pencil | Tool::Brush | Tool::Shape) {
                        xp_widths(ui, theme, doc);
                        ui.add_space(4.0);
                    }
                    xp_options(ui, theme, doc, out);
                },
            );
        });

    egui::Panel::bottom("xp_palette")
        .exact_size(50.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(4.0);
                    xp_colors(ui, theme, doc, out);
                },
            );
        });
}

/// La caja de colores de Paint clásico: los dos colores encimados a la
/// izquierda y la paleta en dos filas.
///
/// Sin las etiquetas «Color 1» y «Color 2», que son de la cinta de Windows 7 en
/// adelante, y sin la fila de personalizados: en XP esa fila no existe, los
/// colores propios se guardan sobre la paleta misma.
fn xp_colors(ui: &mut Ui, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    const CELDA: f32 = 16.0;
    const CAJA: f32 = 17.0;
    let borde = egui::Stroke::new(1.0, Color32::from(theme.border_strong));

    // Los dos colores, encimados.
    let (par, presp) = ui.allocate_exact_size(vec2(30.0, 30.0), Sense::click());
    let r2 = Rect::from_min_size(
        Pos2::new(par.right() - CAJA, par.bottom() - CAJA),
        vec2(CAJA, CAJA),
    );
    let r1 = Rect::from_min_size(par.min, vec2(CAJA, CAJA));
    for (rr, c) in [(r2, doc.color2), (r1, doc.color1)] {
        sunken(ui, rr.expand(1.0), theme);
        ui.painter().rect_filled(rr, 0.0, c);
        ui.painter().rect_stroke(rr, 0.0, borde, egui::StrokeKind::Inside);
    }
    if presp.clicked() {
        if let Some(p) = presp.interact_pointer_pos() {
            out.picking_c1 = r1.contains(p);
        }
    }

    ui.add_space(7.0);

    // La paleta: dos filas pegadas, sin hueco entre celdas, como en Paint.
    let filas = 2;
    let cols = XP_PALETTE.len() / filas;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(cols as f32 * CELDA, filas as f32 * CELDA),
        Sense::click_and_drag(),
    );
    let celda = |i: usize| {
        Rect::from_min_size(
            Pos2::new(
                rect.left() + (i % cols) as f32 * CELDA,
                rect.top() + (i / cols) as f32 * CELDA,
            ),
            vec2(CELDA, CELDA),
        )
    };
    for (i, rgb) in XP_PALETTE.iter().enumerate() {
        let c = celda(i);
        ui.painter()
            .rect_filled(c.shrink(1.0), 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        ui.painter()
            .rect_stroke(c.shrink(1.0), 0.0, borde, egui::StrokeKind::Inside);
    }

    // Izquierdo pone el Color 1 y derecho el Color 2, sin tener que elegir
    // antes cuál estás editando. Es como funciona Paint desde siempre, y es lo
    // que hace que pintar con dos colores no cueste un viaje extra.
    if resp.clicked() || resp.secondary_clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if let Some(i) = (0..XP_PALETTE.len()).find(|i| celda(*i).contains(p)) {
                let rgb = XP_PALETTE[i];
                let c = Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                if resp.secondary_clicked() {
                    doc.color2 = c;
                } else {
                    doc.color1 = c;
                }
            }
        }
    }
}

// ------------------------------------------------------------ chrome propio

/// El orden del riel. No cambia nunca, que es todo el punto: la mano aprende
/// dónde está cada cosa y deja de leer.
const RAIL: [Tool; 9] = [
    Tool::Select,
    Tool::Pencil,
    Tool::Brush,
    Tool::Fill,
    Tool::Text,
    Tool::Shape,
    Tool::Eraser,
    Tool::Picker,
    Tool::Magnifier,
];

/// Los dos colores apilados y encimados, como en cualquier programa de dibujo.
/// Ocupa 34 px de ancho en vez de los 120 del bloque de la cinta, que es lo que
/// permite que el color viva en el mismo riel que las herramientas.
fn well_colors(ui: &mut Ui, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    const CAJA: f32 = 21.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(34.0, 30.0), Sense::click());
    let borde = egui::Stroke::new(1.0, Color32::from(theme.border_strong));
    let r2 = Rect::from_min_size(Pos2::new(rect.right() - CAJA, rect.bottom() - CAJA), vec2(CAJA, CAJA));
    let r1 = Rect::from_min_size(rect.min, vec2(CAJA, CAJA));

    // El 2 primero: el 1 va encima, y encima es lo que significa «activo».
    for (r, c, es1) in [(r2, doc.color2, false), (r1, doc.color1, true)] {
        ui.painter().rect_filled(r, 0.0, c);
        ui.painter().rect_stroke(r, 0.0, borde, egui::StrokeKind::Inside);
        if out.picking_c1 == es1 {
            ui.painter().rect_stroke(
                r.expand(1.5),
                0.0,
                egui::Stroke::new(1.5, Color32::from(theme.accent)),
                egui::StrokeKind::Outside,
            );
        }
    }
    if let Some(p) = resp.interact_pointer_pos() {
        if resp.clicked() {
            // Doble clic sobre el que ya estaba elegido abre el editor.
            let sobre1 = r1.contains(p);
            if out.picking_c1 == sobre1 {
                out.open_color_dialog = true;
            }
            out.picking_c1 = sobre1;
        }
    }
    let _ = resp.on_hover_text(lang::t("Colores"));
}

/// La paleta del riel: dos columnas, porque el riel mide 52 px.
fn rail_palette(ui: &mut Ui, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    const CELDA: f32 = 13.0;
    const HUECO: f32 = 2.0;
    const PASO: f32 = CELDA + HUECO;
    let filas = PALETTE.len() / 2;
    let (rect, resp) =
        ui.allocate_exact_size(vec2(2.0 * PASO, filas as f32 * PASO), Sense::click());
    let borde = egui::Stroke::new(1.0, Color32::from(theme.border_strong));

    let celda = |i: usize| {
        Rect::from_min_size(
            Pos2::new(
                rect.left() + (i % 2) as f32 * PASO,
                rect.top() + (i / 2) as f32 * PASO,
            ),
            vec2(CELDA, CELDA),
        )
    };
    for (i, rgb) in PALETTE.iter().enumerate() {
        let r = celda(i);
        ui.painter().rect_filled(r, 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        ui.painter().rect_stroke(r, 0.0, borde, egui::StrokeKind::Inside);
    }
    if resp.clicked() {
        if let Some(p) = resp.interact_pointer_pos() {
            if let Some(i) = (0..PALETTE.len()).find(|i| celda(*i).contains(p)) {
                apply_palette_hit(PaletteHit::Fixed(i), doc, out);
            }
        }
    }
}

/// Una opción de la barra de contexto: un botón chato con contenido dibujado.
fn ctx_opt(ui: &mut Ui, theme: &Theme, w: f32, on: bool) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    let bg = if on {
        theme.button_active.into()
    } else if resp.hovered() {
        theme.button_hover.into()
    } else {
        Color32::TRANSPARENT
    };
    if bg != Color32::TRANSPARENT {
        ui.painter().rect_filled(rect, theme.button_rounding, bg);
        if on {
            ui.painter().rect_stroke(
                rect,
                theme.button_rounding,
                egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                egui::StrokeKind::Inside,
            );
        }
    }
    (rect, resp)
}

/// Un botón de la barra con texto.
fn ctx_text(ui: &mut Ui, theme: &Theme, label: &str, on: bool) -> bool {
    let w = label.chars().count() as f32 * theme.font_size * 0.60 + 14.0;
    let (rect, resp) = ctx_opt(ui, theme, w, on);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(theme.font_size),
        theme.text.into(),
    );
    resp.clicked()
}

/// Separador vertical de la barra.
fn ctx_sep(ui: &mut Ui, theme: &Theme) {
    let (r, _) = ui.allocate_exact_size(vec2(9.0, 24.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(r.center().x, r.top() + 2.0),
            Pos2::new(r.center().x, r.bottom() - 2.0),
        ],
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    );
}

/// Los grosores, dibujados del grosor real. Es la regla de esta barra: las
/// opciones se ven, no se leen. Nada de «1 3 5 8».
fn ctx_widths(ui: &mut Ui, theme: &Theme, doc: &mut Doc) {
    let ws = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
    for w in ws {
        let on = (doc.width - w).abs() < 0.5;
        let (rect, resp) = ctx_opt(ui, theme, 34.0, on);
        ui.painter().line_segment(
            [
                Pos2::new(rect.left() + 6.0, rect.center().y),
                Pos2::new(rect.right() - 6.0, rect.center().y),
            ],
            egui::Stroke::new(w, Color32::from(theme.text)),
        );
        if resp.clicked() {
            doc.width = w;
        }
    }
}

/// La barra que cambia con la herramienta.
///
/// Con el cuentagotas queda casi vacía, y está bien: no hay nada que
/// configurar. Mostrar el grosor del pincel mientras tomás un color es ruido.
fn context_bar(
    ui: &mut Ui,
    doc: &mut Doc,
    theme: &Theme,
    out: &mut UiOut,
    text: Option<&mut TextBox>,
) {
    ui.painter().text(
        Pos2::new(ui.cursor().left() + 2.0, ui.max_rect().center().y),
        Align2::LEFT_CENTER,
        doc.tool.label(),
        FontId::proportional(theme.font_size - 0.5),
        theme.text_dim.into(),
    );
    let ancho = doc.tool.label().chars().count() as f32 * (theme.font_size - 0.5) * 0.60 + 6.0;
    ui.add_space(ancho);
    ctx_sep(ui, theme);

    match doc.tool {
        Tool::Pencil | Tool::Eraser => ctx_widths(ui, theme, doc),

        Tool::Brush => {
            ctx_widths(ui, theme, doc);
            ctx_sep(ui, theme);
            if ctx_text(ui, theme, lang::t("Pinceles"), false) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_brushes = true;
            }
        }

        Tool::Shape => {
            // La galería entera entra a lo ancho, con scroll: acá sobra el
            // espacio que en la cinta faltaba.
            egui::ScrollArea::horizontal()
                .id_salt("formas_ctx")
                .max_width(ui.available_width() - 190.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for sh in ALL_SHAPES {
                            let on = doc.shape == sh;
                            if shape_button(ui, theme, sh, on) {
                                doc.shape = sh;
                                doc.set_tool(Tool::Shape);
                            }
                        }
                    });
                });
            ctx_sep(ui, theme);
            if ctx_text(ui, theme, lang::t("Contorno"), false) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_outline_menu = true;
            }
            if ctx_text(ui, theme, lang::t("Relleno"), false) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_fill_menu = true;
            }
        }

        Tool::Text => {
            if let Some(tb) = text {
                egui::ComboBox::from_id_salt("familia_ctx")
                    .selected_text(crate::text::FAMILIES[tb.family])
                    .width(126.0)
                    .show_ui(ui, |ui| {
                        for (i, n) in crate::text::FAMILIES.iter().enumerate() {
                            ui.selectable_value(&mut tb.family, i, *n);
                        }
                    });
                egui::ComboBox::from_id_salt("tam_ctx")
                    .selected_text(format!("{}", tb.size as i32))
                    .width(56.0)
                    .show_ui(ui, |ui| {
                        for sz in crate::text::SIZES {
                            ui.selectable_value(&mut tb.size, sz, format!("{}", sz as i32));
                        }
                    });
                ctx_sep(ui, theme);
                // El salto de línea es para el botón alto de la cinta; acá va
                // en una sola línea.
                let transp = lang::t("Trans-\nparente").replace('\n', "");
                if ctx_text(ui, theme, &transp, !tb.opaque) {
                    tb.opaque = false;
                }
                if ctx_text(ui, theme, lang::t("Opaco"), tb.opaque) {
                    tb.opaque = true;
                }
            } else {
                ctx_hint(ui, theme, "Arrastrá sobre el lienzo para abrir un cuadro");
            }
        }

        Tool::Select => {
            if ctx_text(ui, theme, lang::t("Selección"), false) {
                out.menu_anchor = ui.min_rect().left_bottom();
                out.open_select_menu = true;
            }
            ctx_sep(ui, theme);
            if ctx_text(ui, theme, lang::t("Recortar"), false) {
                out.cmds.push(Cmd::Crop);
            }
            if ctx_text(ui, theme, lang::t("Eliminar"), false) {
                out.cmds.push(Cmd::Delete);
            }
        }

        Tool::Fill => ctx_hint(ui, theme, "Tocá una zona para rellenarla con el Color 1"),
        Tool::Picker => ctx_hint(ui, theme, "Tocá el lienzo para tomar un color"),

        Tool::Magnifier => {
            if ctx_text(ui, theme, "−", false) {
                out.cmds.push(Cmd::ZoomOut);
            }
            if ctx_text(ui, theme, lang::t("100%"), false) {
                out.cmds.push(Cmd::Zoom100);
            }
            if ctx_text(ui, theme, "+", false) {
                out.cmds.push(Cmd::ZoomIn);
            }
        }
    }
}

/// Una línea de ayuda para las herramientas que no configuran nada.
fn ctx_hint(ui: &mut Ui, theme: &Theme, s: &str) {
    ui.painter().text(
        Pos2::new(ui.cursor().left() + 2.0, ui.max_rect().center().y),
        Align2::LEFT_CENTER,
        s,
        FontId::proportional(theme.font_size - 0.5),
        theme.text_dim.into(),
    );
}

/// El chrome propio de Lienzo.
///
/// El panel izquierdo se declara **antes** que el de arriba a propósito: en
/// egui el primero se queda con el borde entero, y así el riel llega de arriba
/// abajo y la barra de contexto arranca a su derecha, como en el diseño.
fn studio(ui: &mut Ui, doc: &mut Doc, theme: &Theme, out: &mut UiOut, text: Option<&mut TextBox>) {
    egui::Panel::left("riel")
        .exact_size(52.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.surface));
            ui.painter().line_segment(
                [
                    Pos2::new(r.right() - 0.5, r.top()),
                    Pos2::new(r.right() - 0.5, r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );

            let raya = |ui: &mut Ui| {
                let (s, _) = ui.allocate_exact_size(vec2(26.0, 11.0), Sense::hover());
                ui.painter().line_segment(
                    [
                        Pos2::new(s.left(), s.center().y),
                        Pos2::new(s.right(), s.center().y),
                    ],
                    egui::Stroke::new(1.0, Color32::from(theme.border)),
                );
            };

            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    ui.add_space(5.0);

                    // Archivo, Edición, Imagen y Ver entran acá, a la pantalla
                    // completa que ya existe. No ocupan lugar el resto del tiempo.
                    let (mr, mresp) = ui.allocate_exact_size(vec2(34.0, 30.0), Sense::click());
                    if mresp.hovered() {
                        ui.painter().rect_filled(
                            mr,
                            theme.button_rounding,
                            Color32::from(theme.button_hover),
                        );
                    }
                    let st = egui::Stroke::new(1.5, Color32::from(theme.icon));
                    for k in 0..3 {
                        let y = mr.center().y + (k as f32 - 1.0) * 5.0;
                        ui.painter().line_segment(
                            [Pos2::new(mr.left() + 9.0, y), Pos2::new(mr.right() - 9.0, y)],
                            st,
                        );
                    }
                    if mresp.clicked() {
                        out.open_file_menu = true;
                    }
                    let _ = mresp.on_hover_text(lang::t("Archivo"));

                    raya(ui);

                    for t in RAIL {
                        if tool_button(ui, theme, t, doc.tool == t) {
                            doc.set_tool(t);
                        }
                    }

                    // El color al pie del mismo riel: la columna entera se lee
                    // como un instrumento —elegís, ajustás, pintás—.
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                        ui.add_space(5.0);
                        rail_palette(ui, theme, doc, out);
                        ui.add_space(4.0);
                        well_colors(ui, theme, doc, out);
                        raya(ui);
                    });
                },
            );
        });

    egui::Panel::top("contexto")
        .exact_size(38.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.surface));
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.bottom() - 0.5),
                    Pos2::new(r.right(), r.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    ui.add_space(10.0);
                    context_bar(ui, doc, theme, out, text);
                },
            );
        });
}

// --------------------------------------------------- chromes de escritorio

/// Una fila de la lateral de macOS: icono, nombre, y el acento lleno cuando
/// está elegida. Es la pieza que reemplaza a la cinta en ese sistema.
fn mac_row(ui: &mut Ui, theme: &Theme, w: f32, icon: Icon, label: &str, on: bool) -> bool {
    // 28 px, no 26: un Mac respira más que una cinta, y con el ritmo apretado
    // la lateral se leía como un panel de Windows con las esquinas redondas.
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
    if on {
        ui.painter().rect_filled(rect, 6.0, Color32::from(theme.accent));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, Color32::from(theme.button_hover));
    }
    let col: Color32 = if on { theme.accent_text.into() } else { mac_ink(theme) };
    draw_icon(
        ui,
        Rect::from_center_size(Pos2::new(rect.left() + 18.0, rect.center().y), vec2(17.0, 17.0)),
        icon,
        col,
    );
    ui.painter().text(
        Pos2::new(rect.left() + 34.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size),
        col,
    );
    resp.clicked()
}

/// El encabezado gris de una sección, en versalitas. Va tanto en la lateral
/// como en el inspector.
fn mac_head(ui: &mut Ui, theme: &Theme, w: f32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 28.0), Sense::hover());
    ui.painter().text(
        Pos2::new(rect.left() + 6.0, rect.bottom() - 6.0),
        Align2::LEFT_BOTTOM,
        label.to_uppercase(),
        FontId::proportional(theme.font_size - 2.0),
        theme.text_dim.into(),
    );
}

/// Control segmentado: opciones excluyentes fundidas en una pastilla, con la
/// elegida sobresaliendo. Es la pieza más reconocible de un Mac.
fn mac_segmented(ui: &mut Ui, theme: &Theme, items: &[&str], on: usize) -> Option<usize> {
    let anchos: Vec<f32> = items
        .iter()
        .map(|t| (t.chars().count() as f32 * theme.font_size * 0.58 + 14.0).max(28.0))
        .collect();
    let total: f32 = anchos.iter().sum::<f32>() + 2.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(total, 24.0), Sense::click());
    ui.painter().rect_filled(rect, 7.0, Color32::from(theme.surface_alt));
    ui.painter().rect_stroke(
        rect,
        7.0,
        egui::Stroke::new(1.0, Color32::from(theme.border)),
        egui::StrokeKind::Inside,
    );

    let mut x = rect.left() + 1.0;
    let mut tocado = None;
    for (i, (t, w)) in items.iter().zip(&anchos).enumerate() {
        let seg = Rect::from_min_size(Pos2::new(x, rect.top() + 1.0), vec2(*w, 22.0));
        if i == on {
            ui.painter().rect_filled(seg, 6.0, Color32::from(theme.surface));
            ui.painter().rect_stroke(
                seg,
                6.0,
                egui::Stroke::new(1.0, Color32::from(theme.border)),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter().text(
            seg.center(),
            Align2::CENTER_CENTER,
            *t,
            FontId::proportional(theme.font_size - 0.5),
            theme.text.into(),
        );
        if resp.clicked() {
            if let Some(p) = resp.interact_pointer_pos() {
                if seg.contains(p) {
                    tocado = Some(i);
                }
            }
        }
        x += w;
    }
    tocado
}

/// Una fila del inspector: nombre a la izquierda y control a la derecha.
fn mac_field(ui: &mut Ui, theme: &Theme, w: f32, label: &str, value: &str) -> Response {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    ui.painter().text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size - 0.5),
        theme.text.into(),
    );
    let caja = Rect::from_min_max(
        Pos2::new(rect.right() - 62.0, rect.top() + 1.0),
        Pos2::new(rect.right(), rect.bottom() - 1.0),
    );
    ui.painter().rect_filled(caja, 6.0, Color32::from(theme.surface));
    ui.painter().rect_stroke(
        caja,
        6.0,
        egui::Stroke::new(1.0, Color32::from(theme.border)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        caja.center(),
        Align2::CENTER_CENTER,
        value,
        FontId::proportional(theme.font_size - 0.5),
        theme.text.into(),
    );
    resp
}

/// Un botón cuadrado de la barra de macOS, con el icono dibujado por quien
/// llama. Devuelve el rectángulo para poder pintar dentro.
/// El color de los iconos de la barra.
///
/// **Grafito, no acento.** En un Mac el azul marca una cosa sola: qué está
/// elegido. Una barra con todos los iconos azules es una costumbre de Windows,
/// y era lo que más delataba que este tema tenía forma de Mac y pesos de otro
/// lado. El único azul de la ventana es la herramienta activa de la lateral.
fn mac_ink(theme: &Theme) -> Color32 {
    Color32::from(theme.text).gamma_multiply(0.82)
}

/// Un grupo de la barra: los botones metidos en una pastilla con borde fino y
/// una sombra de un píxel. Sueltos sobre el fondo, los iconos flotan.
fn mac_group(ui: &mut Ui, theme: &Theme, n: usize) -> (Rect, Response) {
    const B: f32 = 30.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(B * n as f32 + 2.0, 26.0), Sense::click());
    ui.painter().rect_filled(
        rect.translate(vec2(0.0, 1.0)),
        7.0,
        Color32::from_black_alpha(14),
    );
    ui.painter().rect_filled(rect, 7.0, Color32::from(theme.surface));
    ui.painter().rect_stroke(
        rect,
        7.0,
        egui::Stroke::new(1.0, Color32::from(theme.border)),
        egui::StrokeKind::Inside,
    );
    (rect, resp)
}

/// Qué botón de un grupo se tocó.
fn mac_hit(rect: Rect, resp: &Response, n: usize) -> Option<usize> {
    if !resp.clicked() {
        return None;
    }
    let p = resp.interact_pointer_pos()?;
    let b = (rect.width() - 2.0) / n as f32;
    let i = ((p.x - rect.left() - 1.0) / b).floor();
    if i < 0.0 || i >= n as f32 {
        return None;
    }
    Some(i as usize)
}

/// El nombre corto de una herramienta.
///
/// «Relleno con color» y «Selector de color» son de la cinta, donde sobraba
/// ancho. En una lateral de 172 px se salen de la fila.
fn mac_label(t: Tool) -> &'static str {
    match t {
        Tool::Fill => lang::t("Relleno"),
        Tool::Picker => lang::t("Cuentagotas"),
        Tool::Brush => lang::t("Pincel"),
        _ => t.label(),
    }
}

/// Chrome de macOS: barra unificada, herramientas en una lateral y opciones en
/// un inspector a la derecha.
///
/// Los semáforos **no** se dibujan: en un Mac los pone el sistema en esa misma
/// esquina, y pintarlos daría dos juegos. Lo que se hace es reservarles el
/// hueco, que es exactamente lo que hace cualquier aplicación con barra
/// unificada.
fn mac_chrome(ui: &mut Ui, doc: &mut Doc, theme: &Theme, out: &mut UiOut) {
    egui::Panel::top("mac_tb")
        .exact_size(50.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            gradient_bar(ui.painter(), r, theme.bar_top.into(), theme.bar_bottom.into());
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.bottom() - 0.5),
                    Pos2::new(r.right(), r.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );

            // El título, centrado sobre toda la barra y con el tamaño debajo.
            ui.painter().text(
                Pos2::new(r.center().x, r.center().y - 6.0),
                Align2::CENTER_CENTER,
                "Lienzo",
                FontId::proportional(theme.font_size + 0.5),
                theme.text.into(),
            );
            ui.painter().text(
                Pos2::new(r.center().x, r.center().y + 8.0),
                Align2::CENTER_CENTER,
                format!("{} × {}", doc.canvas.w, doc.canvas.h),
                FontId::proportional(theme.font_size - 2.0),
                theme.text_dim.into(),
            );

            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    // El engranaje pegado al borde: en un Mac los menús son del
                    // sistema, así que ésta es la única puerta a Archivo y a
                    // Configuración.
                    //
                    // Sin hueco para los semáforos. La barra unificada de verdad
                    // los mete *dentro* del contenido, pero eso hay que pedirlo
                    // al abrir la ventana y vale para todo el programa, no por
                    // tema. Como acá la barra de título la dibuja el sistema
                    // aparte, reservarles 76 px dejaba el botón flotando sin que
                    // nada explicara por qué.
                    ui.add_space(10.0);
                    let (gr, gresp) = mac_group(ui, theme, 1);
                    small_icon(
                        ui,
                        Rect::from_center_size(gr.center(), vec2(18.0, 18.0)),
                        Ico::Settings,
                        mac_ink(theme),
                    );
                    if gresp.on_hover_text(lang::t("Configuración")).clicked() {
                        out.open_settings = true;
                    }

                    // Todo lo demás a la derecha: en un Mac la izquierda de la
                    // barra es de los semáforos y de nada más.
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        if let Some(i) = mac_segmented(ui, theme, &["−", "100%", "+"], 1) {
                            out.cmds.push(match i {
                                0 => Cmd::ZoomOut,
                                2 => Cmd::ZoomIn,
                                _ => Cmd::Zoom100,
                            });
                        }
                        ui.add_space(8.0);
                        // Dibujados, no escritos: puestos como texto `↶ ↷`
                        // salían dos rectángulos vacíos, porque esos signos no
                        // están en la fuente que trae egui.
                        let (ur, uresp) = mac_group(ui, theme, 2);
                        let vivos = [doc.canvas.can_undo(), doc.canvas.can_redo()];
                        for k in 0..2 {
                            let b = Rect::from_min_size(
                                Pos2::new(ur.left() + 1.0 + 30.0 * k as f32, ur.top()),
                                vec2(30.0, ur.height()),
                            );
                            undo_arrow(
                                ui,
                                b.shrink(5.0),
                                mac_ink(theme).gamma_multiply(if vivos[k] { 1.0 } else { 0.30 }),
                                k == 1,
                            );
                        }
                        if let Some(k) = mac_hit(ur, &uresp, 2) {
                            if vivos[k] {
                                out.cmds.push(if k == 0 { Cmd::Undo } else { Cmd::Redo });
                            }
                        }
                    });
                },
            );
        });

    egui::Panel::left("mac_side")
        .exact_size(172.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.surface_alt));
            ui.painter().line_segment(
                [
                    Pos2::new(r.right() - 0.5, r.top()),
                    Pos2::new(r.right() - 0.5, r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            // Aire a los dos lados: la pastilla de selección **flota dentro** de
            // la lateral. Sin el margen se cortaba contra el marco de la ventana.
            ui.allocate_ui_with_layout(
                r.shrink2(vec2(9.0, 8.0)).size(),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 1.0;
                    let w = 154.0;
                    mac_head(ui, theme, w, lang::t("Herramientas"));
                    for t in [
                        Tool::Pencil,
                        Tool::Brush,
                        Tool::Fill,
                        Tool::Text,
                        Tool::Shape,
                        Tool::Eraser,
                        Tool::Picker,
                        Tool::Magnifier,
                    ] {
                        if mac_row(ui, theme, w, Icon::T(t), mac_label(t), doc.tool == t) {
                            doc.set_tool(t);
                        }
                    }
                    mac_head(ui, theme, w, lang::t("Selección"));
                    if mac_row(
                        ui,
                        theme,
                        w,
                        Icon::T(Tool::Select),
                        lang::t("Seleccionar"),
                        doc.tool == Tool::Select,
                    ) {
                        doc.set_tool(Tool::Select);
                    }
                    if mac_row(ui, theme, w, Icon::I(Ico::Crop), lang::t("Recortar"), false) {
                        out.cmds.push(Cmd::Crop);
                    }
                },
            );
        });

    egui::Panel::right("mac_insp")
        .exact_size(194.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.surface_alt));
            ui.painter().line_segment(
                [
                    Pos2::new(r.left() + 0.5, r.top()),
                    Pos2::new(r.left() + 0.5, r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.shrink2(vec2(13.0, 10.0)).size(),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.spacing_mut().item_spacing.y = 3.0;
                    let w = 168.0;

                    mac_head(ui, theme, w, lang::t("Trazo"));
                    mac_label_row(ui, theme, w, lang::t("Grosor"));
                    let ws = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
                    let puesto = ws
                        .iter()
                        .position(|x| (doc.width - x).abs() < 0.5)
                        .unwrap_or(0);
                    let etiquetas: Vec<String> =
                        ws.iter().map(|x| format!("{}", *x as i32)).collect();
                    let refs: Vec<&str> = etiquetas.iter().map(|x| x.as_str()).collect();
                    if let Some(i) = mac_segmented(ui, theme, &refs, puesto) {
                        doc.width = ws[i];
                    }

                    // Nombre a la izquierda y muestra a la derecha, que es como
                    // se lee un inspector. El bloque de la cinta ponía la
                    // etiqueta *debajo* del cuadrado y se pisaban.
                    //
                    // Sin encabezado propio: la paleta y los dos colores son
                    // parte del trazo, no otra sección. Cuatro títulos para
                    // tres controles es más estructura que contenido.
                    ui.add_space(8.0);
                    for (etiqueta, es1) in [(lang::t("Color"), true), (lang::t("Fondo"), false)] {
                        let (rect, resp) = ui.allocate_exact_size(vec2(w, 28.0), Sense::click());
                        ui.painter().text(
                            Pos2::new(rect.left(), rect.center().y),
                            Align2::LEFT_CENTER,
                            etiqueta,
                            FontId::proportional(theme.font_size - 0.5),
                            theme.text.into(),
                        );
                        let pozo = Rect::from_min_max(
                            Pos2::new(rect.right() - 34.0, rect.top() + 3.0),
                            Pos2::new(rect.right(), rect.bottom() - 3.0),
                        );
                        // Radio 4, no 11: con el radio grande el pozo se vuelve
                        // una cápsula y deja de leerse como muestra de color.
                        let c = if es1 { doc.color1 } else { doc.color2 };
                        ui.painter().rect_filled(pozo, 4.0, c);
                        ui.painter().rect_stroke(
                            pozo,
                            4.0,
                            egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                            egui::StrokeKind::Inside,
                        );
                        if out.picking_c1 == es1 {
                            ui.painter().rect_stroke(
                                pozo.expand(2.0),
                                6.0,
                                egui::Stroke::new(2.0, Color32::from(theme.accent)),
                                egui::StrokeKind::Outside,
                            );
                        }
                        if resp.clicked() {
                            out.picking_c1 = es1;
                        }
                    }

                    ui.add_space(7.0);
                    if let Some(hit) = mac_palette(ui, theme, &out.custom) {
                        apply_palette_hit(hit, doc, out);
                    }
                    ui.add_space(6.0);
                    if mac_button(ui, theme, w, lang::t("Editar colores…")) {
                        out.open_color_dialog = true;
                    }

                    mac_head(ui, theme, w, lang::t("Tamaño del lienzo"));
                    if mac_field(ui, theme, w, lang::t("Ancho"), &doc.canvas.w.to_string()).clicked()
                        || mac_field(ui, theme, w, lang::t("Alto"), &doc.canvas.h.to_string())
                            .clicked()
                    {
                        out.cmds.push(Cmd::PropertiesDialog);
                    }
                },
            );
        });
}

/// Una etiqueta suelta del inspector, sin control al lado.
fn mac_label_row(ui: &mut Ui, theme: &Theme, w: f32, label: &str) {
    let (rect, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
    ui.painter().text(
        Pos2::new(rect.left(), rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(theme.font_size - 0.5),
        theme.text.into(),
    );
}

/// Un botón de inspector: fondo claro, borde fino y el texto centrado.
fn mac_button(ui: &mut Ui, theme: &Theme, w: f32, label: &str) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 24.0), Sense::click());
    let bg: Color32 = if resp.hovered() { theme.button_hover.into() } else { theme.surface.into() };
    ui.painter().rect_filled(rect, 6.0, bg);
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, Color32::from(theme.border)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(theme.font_size - 0.5),
        theme.text.into(),
    );
    resp.clicked()
}

/// La paleta del inspector: los veinte fijos, y los personalizados **sólo si
/// hay alguno**. Diez casillas punteadas vacías no informan nada y se leen
/// como algo roto.
fn mac_palette(ui: &mut Ui, theme: &Theme, custom: &[Option<Color32>]) -> Option<PaletteHit> {
    const CELDA: f32 = 15.0;
    const HUECO: f32 = 2.0;
    const PASO: f32 = CELDA + HUECO;
    let guardados: Vec<(usize, Color32)> = custom
        .iter()
        .enumerate()
        .filter_map(|(i, c)| c.map(|c| (i, c)))
        .collect();
    let filas = 2 + if guardados.is_empty() { 0 } else { 1 };
    let extra = if guardados.is_empty() { 0.0 } else { 5.0 };

    let (rect, resp) = ui.allocate_exact_size(
        vec2(10.0 * PASO, filas as f32 * PASO + extra),
        Sense::click(),
    );
    let borde = egui::Stroke::new(1.0, Color32::from(theme.border_strong));
    let celda = |col: usize, fila: usize| {
        let y = rect.top() + fila as f32 * PASO + if fila == 2 { extra } else { 0.0 };
        Rect::from_min_size(
            Pos2::new(rect.left() + col as f32 * PASO, y),
            vec2(CELDA, CELDA),
        )
    };

    for (i, rgb) in PALETTE.iter().enumerate() {
        let r = celda(i % 10, i / 10);
        ui.painter().rect_filled(r, 3.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
        ui.painter().rect_stroke(r, 3.0, borde, egui::StrokeKind::Inside);
    }
    for (n, (_, c)) in guardados.iter().enumerate() {
        let r = celda(n, 2);
        ui.painter().rect_filled(r, 3.0, *c);
        ui.painter().rect_stroke(r, 3.0, borde, egui::StrokeKind::Inside);
    }

    if resp.clicked() {
        let p = resp.interact_pointer_pos()?;
        for i in 0..PALETTE.len() {
            if celda(i % 10, i / 10).contains(p) {
                return Some(PaletteHit::Fixed(i));
            }
        }
        for (n, (i, _)) in guardados.iter().enumerate() {
            if celda(n, 2).contains(p) {
                return Some(PaletteHit::Custom(*i));
            }
        }
    }
    None
}

/// Un grupo de botones enlazados: fundidos, con las esquinas de afuera
/// redondeadas y las de adentro rectas. Es la pieza que hace que algo se vea
/// de GNOME y no de otro lado.
fn gnome_linked(ui: &mut Ui, theme: &Theme, n: usize) -> (Rect, Response) {
    const B: f32 = 32.0;
    let (rect, resp) = ui.allocate_exact_size(vec2(B * n as f32, 30.0), Sense::click());
    ui.painter().rect_filled(rect, 6.0, Color32::from(theme.button));
    ui.painter().rect_stroke(
        rect,
        6.0,
        egui::Stroke::new(1.0, Color32::from(theme.button_border)),
        egui::StrokeKind::Inside,
    );
    for k in 1..n {
        let x = rect.left() + B * k as f32;
        ui.painter().line_segment(
            [Pos2::new(x, rect.top() + 1.0), Pos2::new(x, rect.bottom() - 1.0)],
            egui::Stroke::new(1.0, Color32::from(theme.button_border)),
        );
    }
    (rect, resp)
}

/// Chrome de GNOME: la barra de título **es** la barra de herramientas, y lo
/// del momento flota sobre el lienzo en vez de quitarle sitio.
fn gnome_chrome(ui: &mut Ui, doc: &mut Doc, theme: &Theme, out: &mut UiOut) {
    const HERR: [Tool; 6] = [
        Tool::Pencil,
        Tool::Brush,
        Tool::Fill,
        Tool::Text,
        Tool::Shape,
        Tool::Eraser,
    ];

    egui::Panel::top("gn_hb")
        .exact_size(47.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            gradient_bar(ui.painter(), r, theme.bar_top.into(), theme.bar_bottom.into());
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.bottom() - 0.5),
                    Pos2::new(r.right(), r.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );

            // Título y subtítulo, centrados sobre la barra entera.
            ui.painter().text(
                Pos2::new(r.center().x, r.center().y - 6.0),
                Align2::CENTER_CENTER,
                "Lienzo",
                FontId::proportional(theme.font_size),
                theme.text.into(),
            );
            ui.painter().text(
                Pos2::new(r.center().x, r.center().y + 8.0),
                Align2::CENTER_CENTER,
                format!("{} × {}", doc.canvas.w, doc.canvas.h),
                FontId::proportional(theme.font_size - 2.0),
                theme.text_dim.into(),
            );

            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add_space(8.0);

                    let (caja, resp) = gnome_linked(ui, theme, HERR.len());
                    for (i, t) in HERR.iter().enumerate() {
                        let b = Rect::from_min_size(
                            Pos2::new(caja.left() + 32.0 * i as f32, caja.top()),
                            vec2(32.0, caja.height()),
                        );
                        let on = doc.tool == *t;
                        if on {
                            ui.painter().rect_filled(
                                b.shrink(1.0),
                                4.0,
                                Color32::from(theme.button_active),
                            );
                        }
                        let col: Color32 = if on { theme.accent.into() } else { theme.icon.into() };
                        tool_icon(ui, b.shrink(7.0), *t, col);
                        if resp.clicked() {
                            if let Some(p) = resp.interact_pointer_pos() {
                                if b.contains(p) {
                                    doc.set_tool(*t);
                                }
                            }
                        }
                    }

                    let (caja, resp) = gnome_linked(ui, theme, 2);
                    for (i, cmd) in [Cmd::Undo, Cmd::Redo].into_iter().enumerate() {
                        let b = Rect::from_min_size(
                            Pos2::new(caja.left() + 32.0 * i as f32, caja.top()),
                            vec2(32.0, caja.height()),
                        );
                        let vivo = if i == 0 { doc.canvas.can_undo() } else { doc.canvas.can_redo() };
                        undo_arrow(
                            ui,
                            b.shrink(7.0),
                            Color32::from(theme.icon).gamma_multiply(if vivo { 1.0 } else { 0.32 }),
                            i == 1,
                        );
                        if vivo && resp.clicked() {
                            if let Some(p) = resp.interact_pointer_pos() {
                                if b.contains(p) {
                                    out.cmds.push(cmd);
                                }
                            }
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(8.0);
                        // El menú en tres rayas: todo lo que no es acción
                        // frecuente vive ahí, que es la regla de GNOME.
                        let (mr, mresp) = ui.allocate_exact_size(vec2(32.0, 30.0), Sense::click());
                        ui.painter().rect_filled(mr, 6.0, Color32::from(theme.button));
                        ui.painter().rect_stroke(
                            mr,
                            6.0,
                            egui::Stroke::new(1.0, Color32::from(theme.button_border)),
                            egui::StrokeKind::Inside,
                        );
                        let st = egui::Stroke::new(1.5, Color32::from(theme.icon));
                        for k in 0..3 {
                            let y = mr.center().y + (k as f32 - 1.0) * 4.5;
                            ui.painter().line_segment(
                                [Pos2::new(mr.left() + 9.0, y), Pos2::new(mr.right() - 9.0, y)],
                                st,
                            );
                        }
                        if mresp.clicked() {
                            out.open_file_menu = true;
                        }

                        ui.add_space(2.0);
                        // Una sola acción sugerida por ventana: GNOME no permite dos.
                        let etiqueta = lang::t("Guardar");
                        let w = etiqueta.chars().count() as f32 * theme.font_size * 0.62 + 26.0;
                        let (gr, gresp) = ui.allocate_exact_size(vec2(w, 30.0), Sense::click());
                        ui.painter().rect_filled(gr, 6.0, Color32::from(theme.accent));
                        ui.painter().text(
                            gr.center(),
                            Align2::CENTER_CENTER,
                            etiqueta,
                            FontId::proportional(theme.font_size),
                            theme.accent_text.into(),
                        );
                        if gresp.clicked() {
                            out.cmds.push(Cmd::Save);
                        }
                    });
                },
            );
        });
}

/// La píldora flotante de GNOME: color y grosor sobre el lienzo, sin quitarle
/// sitio. Se dibuja aparte del chrome porque va **encima** del área de dibujo,
/// no en un panel que le coma espacio.
pub fn gnome_pill(ctx: &egui::Context, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    egui::Area::new(egui::Id::new("gn_pill"))
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -22.0))
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(Color32::from(theme.surface))
                .stroke(egui::Stroke::new(1.0, Color32::from(theme.border)))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::symmetric(11, 7))
                .shadow(egui::Shadow {
                    offset: [0, 6],
                    blur: 24,
                    spread: 0,
                    color: Color32::from_black_alpha(52),
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        color_boxes(ui, theme, doc, out);
                        if let Some(hit) = palette_grid(ui, theme, &out.custom) {
                            apply_palette_hit(hit, doc, out);
                        }
                        let ws = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
                        for w in ws {
                            let on = (doc.width - w).abs() < 0.5;
                            let (rect, resp) = ctx_opt(ui, theme, 30.0, on);
                            ui.painter().line_segment(
                                [
                                    Pos2::new(rect.left() + 6.0, rect.center().y),
                                    Pos2::new(rect.right() - 6.0, rect.center().y),
                                ],
                                egui::Stroke::new(w, Color32::from(theme.text)),
                            );
                            if resp.clicked() {
                                doc.width = w;
                            }
                        }
                    });
                });
        });
}

/// Chrome de KDE: barra de menú, barra de iconos, panel de herramientas y panel
/// de color.
///
/// Es a propósito lo contrario de GNOME. Plasma es el escritorio de quien
/// quiere verlo todo a la vez, y esconder las cosas en un menú de tres rayas
/// sería traicionar eso tanto como ponerle una cinta a un Mac.
fn kde_chrome(ui: &mut Ui, doc: &mut Doc, theme: &Theme, themes: &[(String, usize)], out: &mut UiOut) {
    egui::Panel::top("kde_menu")
        .exact_size(24.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 1.0;
                    ui.add_space(4.0);
                    file_menu(ui, out);
                    edit_menu(ui, out);
                    image_menu(ui, out);
                    view_menu(ui, themes, out);
                    colors_menu(ui, out);
                    help_menu(ui, out);
                },
            );
        });

    egui::Panel::top("kde_tb")
        .exact_size(36.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            gradient_bar(ui.painter(), r, theme.bar_top.into(), theme.bar_bottom.into());
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.bottom() - 0.5),
                    Pos2::new(r.right(), r.bottom() - 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(5.0);
                    for (ico, cmd) in [
                        (Ico::New, Cmd::New),
                        (Ico::Open, Cmd::Open),
                        (Ico::Save, Cmd::Save),
                    ] {
                        let (rect, resp) = frame_button(ui, 28.0, false, theme);
                        small_icon(ui, rect.shrink(5.0), ico, theme.icon.into());
                        if resp.clicked() {
                            out.cmds.push(cmd);
                        }
                    }
                    ui.add_space(6.0);
                    for (i, cmd) in [Cmd::Undo, Cmd::Redo].into_iter().enumerate() {
                        let vivo = if i == 0 { doc.canvas.can_undo() } else { doc.canvas.can_redo() };
                        let (rect, resp) = frame_button(ui, 28.0, false, theme);
                        undo_arrow(
                            ui,
                            rect.shrink(6.0),
                            Color32::from(theme.icon).gamma_multiply(if vivo { 1.0 } else { 0.32 }),
                            i == 1,
                        );
                        if vivo && resp.clicked() {
                            out.cmds.push(cmd);
                        }
                    }
                    ui.add_space(6.0);
                    for (ico, cmd) in [
                        (Ico::Cut, Cmd::Cut),
                        (Ico::Copy, Cmd::Copy),
                        (Ico::Paste, Cmd::Paste),
                    ] {
                        let (rect, resp) = frame_button(ui, 28.0, false, theme);
                        small_icon(ui, rect.shrink(5.0), ico, theme.icon.into());
                        if resp.clicked() {
                            out.cmds.push(cmd);
                        }
                    }
                },
            );
        });

    egui::Panel::left("kde_dock")
        .exact_size(62.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.painter().line_segment(
                [
                    Pos2::new(r.right() - 0.5, r.top()),
                    Pos2::new(r.right() - 0.5, r.bottom()),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::top_down(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                    ui.add_space(6.0);
                    const T: [Tool; 10] = [
                        Tool::Select, Tool::Pencil,
                        Tool::Brush, Tool::Fill,
                        Tool::Text, Tool::Shape,
                        Tool::Eraser, Tool::Picker,
                        Tool::Magnifier, Tool::Select,
                    ];
                    for par in T.chunks(2) {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                            for t in par {
                                if tool_button(ui, theme, *t, doc.tool == *t) {
                                    doc.set_tool(*t);
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    size_button(ui, theme, doc, out);
                },
            );
        });

    egui::Panel::bottom("kde_colors")
        .exact_size(58.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.window));
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.top() + 0.5),
                    Pos2::new(r.right(), r.top() + 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );
            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.add_space(7.0);
                    color_boxes(ui, theme, doc, out);
                    ui.add_space(9.0);
                    if let Some(hit) = palette_grid(ui, theme, &out.custom) {
                        apply_palette_hit(hit, doc, out);
                    }
                },
            );
        });
}


// ------------------------------------------------------------ chrome 2077

/// Las herramientas de la barra, en el orden en que se usan.
const NEON_TOOLS: [Tool; 9] = [
    Tool::Pencil,
    Tool::Brush,
    Tool::Fill,
    Tool::Text,
    Tool::Shape,
    Tool::Eraser,
    Tool::Picker,
    Tool::Magnifier,
    Tool::Select,
];

/// Un botón de 2077.
///
/// Los tres estados son tres colores, no tres rellenos: el cursor encima pinta
/// el borde de amarillo, lo elegido lo pinta de cian **y repite el mismo borde
/// corrido dos píxeles en magenta**. Es el único tema de los ocho donde lo
/// activo no se marca con un fondo.
fn neon_btn(ui: &mut Ui, theme: &Theme, w: f32, on: bool) -> (Rect, Response) {
    let (rect, resp) = ui.allocate_exact_size(vec2(w, 38.0), Sense::click());
    let borde = if on {
        Some(Color32::from(theme.accent))
    } else if resp.hovered() {
        Some(Color32::from(theme.accent_hot))
    } else {
        None
    };
    if let Some(c) = borde {
        if on {
            // El canal corrido. Va primero para que el cian quede encima.
            ui.painter().rect_stroke(
                rect.translate(vec2(2.0, 0.0)),
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.accent_alt).gamma_multiply(0.6)),
                egui::StrokeKind::Inside,
            );
        }
        ui.painter()
            .rect_stroke(rect, 0.0, egui::Stroke::new(1.0, c), egui::StrokeKind::Inside);
    }
    (rect, resp)
}

/// El color del contenido de un botón, según su estado.
fn neon_ink(theme: &Theme, on: bool, hover: bool) -> Color32 {
    if on {
        Color32::from(theme.accent)
    } else if hover {
        Color32::from(theme.accent_hot)
    } else {
        Color32::from(theme.text)
    }
}

/// Una etiqueta de la barra: monoespaciada, versalitas y muy espaciada.
fn neon_label(ui: &mut Ui, theme: &Theme, texto: &str) {
    let ancho = texto.chars().count() as f32 * (theme.font_size - 3.0) * 0.95 + 8.0;
    let (r, _) = ui.allocate_exact_size(vec2(ancho, 38.0), Sense::hover());
    // Letra por letra, para poder separarlas: egui no tiene interletrado.
    let paso = ancho / (texto.chars().count() as f32 + 1.0);
    for (i, ch) in texto.to_uppercase().chars().enumerate() {
        ui.painter().text(
            Pos2::new(r.left() + paso * (i as f32 + 0.5), r.center().y),
            Align2::LEFT_CENTER,
            ch,
            FontId::monospace(theme.font_size - 3.0),
            theme.text_dim.into(),
        );
    }
}

/// Chrome de 2077: una sola barra al pie, y el lienzo hasta los cuatro bordes.
fn neon_chrome(ui: &mut Ui, doc: &mut Doc, theme: &Theme, out: &mut UiOut) {
    egui::Panel::bottom("neon")
        .exact_size(58.0)
        .frame(egui::Frame::NONE)
        .show(ui, |ui| {
            let r = ui.max_rect();
            ui.painter().rect_filled(r, 0.0, Color32::from(theme.surface));
            ui.painter().line_segment(
                [
                    Pos2::new(r.left(), r.top() + 0.5),
                    Pos2::new(r.right(), r.top() + 0.5),
                ],
                egui::Stroke::new(1.0, Color32::from(theme.border)),
            );

            let sep = |ui: &mut Ui| {
                let (s, _) = ui.allocate_exact_size(vec2(25.0, 38.0), Sense::hover());
                ui.painter().line_segment(
                    [
                        Pos2::new(s.center().x, s.top() + 5.0),
                        Pos2::new(s.center().x, s.bottom() - 5.0),
                    ],
                    egui::Stroke::new(1.0, Color32::from(theme.border)),
                );
            };

            ui.allocate_ui_with_layout(
                r.size(),
                egui::Layout::left_to_right(egui::Align::Center),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.add_space(12.0);

                    for t in NEON_TOOLS {
                        let on = doc.tool == t;
                        let (rect, resp) = neon_btn(ui, theme, 38.0, on);
                        tool_icon(
                            ui,
                            rect.shrink(9.0),
                            t,
                            neon_ink(theme, on, resp.hovered()),
                        );
                        if resp.on_hover_text(t.label()).clicked() {
                            doc.set_tool(t);
                        }
                    }

                    sep(ui);

                    // Los dos colores. El elegido lleva los tres canales.
                    for es1 in [true, false] {
                        let (rect, resp) = ui.allocate_exact_size(vec2(26.0, 26.0), Sense::click());
                        ui.painter()
                            .rect_filled(rect, 0.0, if es1 { doc.color1 } else { doc.color2 });
                        if out.picking_c1 == es1 {
                            ui.painter().rect_stroke(
                                rect.translate(vec2(2.0, 0.0)),
                                0.0,
                                egui::Stroke::new(1.0, Color32::from(theme.accent_alt)),
                                egui::StrokeKind::Outside,
                            );
                            ui.painter().rect_stroke(
                                rect.translate(vec2(-2.0, 0.0)),
                                0.0,
                                egui::Stroke::new(1.0, Color32::from(theme.accent_hot)),
                                egui::StrokeKind::Outside,
                            );
                        }
                        ui.painter().rect_stroke(
                            rect,
                            0.0,
                            egui::Stroke::new(
                                1.0,
                                if out.picking_c1 == es1 {
                                    Color32::from(theme.accent)
                                } else {
                                    Color32::from(theme.border)
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        if resp.clicked() {
                            out.picking_c1 = es1;
                        }
                    }

                    ui.add_space(9.0);
                    if let Some(hit) = neon_palette(ui, theme) {
                        apply_palette_hit(hit, doc, out);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(12.0);
                        let (mr, mresp) = neon_btn(ui, theme, 38.0, false);
                        let col = neon_ink(theme, false, mresp.hovered());
                        for k in 0..3 {
                            let y = mr.center().y + (k as f32 - 1.0) * 5.0;
                            ui.painter().line_segment(
                                [Pos2::new(mr.left() + 11.0, y), Pos2::new(mr.right() - 11.0, y)],
                                egui::Stroke::new(1.5, col),
                            );
                        }
                        if mresp.clicked() {
                            out.open_file_menu = true;
                        }

                        for (i, cmd) in [Cmd::Redo, Cmd::Undo].into_iter().enumerate() {
                            let vivo = if i == 1 { doc.canvas.can_undo() } else { doc.canvas.can_redo() };
                            let (rect, resp) = neon_btn(ui, theme, 38.0, false);
                            undo_arrow(
                                ui,
                                rect.shrink(9.0),
                                neon_ink(theme, false, vivo && resp.hovered())
                                    .gamma_multiply(if vivo { 1.0 } else { 0.30 }),
                                i == 0,
                            );
                            if vivo && resp.clicked() {
                                out.cmds.push(cmd);
                            }
                        }

                        sep(ui);
                        let (zr, zresp) = neon_btn(ui, theme, 56.0, false);
                        ui.painter().text(
                            zr.center(),
                            Align2::CENTER_CENTER,
                            "100%",
                            FontId::monospace(theme.font_size - 1.0),
                            neon_ink(theme, false, zresp.hovered()),
                        );
                        if zresp.clicked() {
                            out.cmds.push(Cmd::Zoom100);
                        }

                        sep(ui);
                        let ws = if doc.tool == Tool::Eraser { ERASER_WIDTHS } else { WIDTHS };
                        for w in ws.iter().rev() {
                            let on = (doc.width - w).abs() < 0.5;
                            let (rect, resp) = neon_btn(ui, theme, 32.0, on);
                            ui.painter().line_segment(
                                [
                                    Pos2::new(rect.left() + 7.0, rect.center().y),
                                    Pos2::new(rect.right() - 7.0, rect.center().y),
                                ],
                                egui::Stroke::new(*w, neon_ink(theme, on, resp.hovered())),
                            );
                            if resp.clicked() {
                                doc.width = *w;
                            }
                        }
                        neon_label(ui, theme, "Grosor");
                    });
                },
            );
        });
}

/// La paleta de la barra: dos filas pegadas, sin bordes.
///
/// Sin marco entre celdas: en un tema hecho de hairlines de neón, veinte
/// rectángulos con contorno gris serían lo más ruidoso de la pantalla.
fn neon_palette(ui: &mut Ui, theme: &Theme) -> Option<PaletteHit> {
    const CELDA: f32 = 16.0;
    let cols = PALETTE.len() / 2;
    let (rect, resp) = ui.allocate_exact_size(
        vec2(cols as f32 * CELDA, 2.0 * CELDA),
        Sense::click_and_drag(),
    );
    let celda = |i: usize| {
        Rect::from_min_size(
            Pos2::new(
                rect.left() + (i % cols) as f32 * CELDA,
                rect.top() + (i / cols) as f32 * CELDA,
            ),
            vec2(CELDA, CELDA),
        )
    };
    for (i, rgb) in PALETTE.iter().enumerate() {
        ui.painter()
            .rect_filled(celda(i), 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
    }
    if resp.hovered() {
        if let Some(p) = ui.ctx().pointer_latest_pos() {
            if let Some(i) = (0..PALETTE.len()).find(|i| celda(*i).contains(p)) {
                ui.painter().rect_stroke(
                    celda(i),
                    0.0,
                    egui::Stroke::new(1.5, Color32::from(theme.accent_hot)),
                    egui::StrokeKind::Outside,
                );
            }
        }
    }
    if resp.clicked() {
        let p = resp.interact_pointer_pos()?;
        return (0..PALETTE.len())
            .find(|i| celda(*i).contains(p))
            .map(PaletteHit::Fixed);
    }
    None
}


// -------------------------------------------------------------- chrome SW

/// Un panel con dos esquinas cortadas a 45°.
///
/// Es toda la identidad de forma del tema: los otros siete usan rectángulos,
/// redondeados o no, y el corte diagonal alcanza para reconocerlo de reojo.
/// `espejo` corta la otra diagonal, para que la consola y el lector se miren.
fn holo_panel(ui: &Ui, r: Rect, theme: &Theme, espejo: bool) {
    const C: f32 = 14.0;
    let v = if espejo {
        vec![
            Pos2::new(r.left() + C, r.top()),
            r.right_top(),
            Pos2::new(r.right(), r.bottom() - C),
            Pos2::new(r.right() - C, r.bottom()),
            r.left_bottom(),
            Pos2::new(r.left(), r.top() + C),
        ]
    } else {
        vec![
            r.left_top(),
            Pos2::new(r.right() - C, r.top()),
            Pos2::new(r.right(), r.top() + C),
            r.right_bottom(),
            Pos2::new(r.left() + C, r.bottom()),
            Pos2::new(r.left(), r.bottom() - C),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        v.clone(),
        Color32::from(theme.surface),
        egui::Stroke::new(1.0, Color32::from(theme.border)),
    ));

    // El barrido va **dentro del panel**, no en el fondo: el que brilla es la
    // proyección. En 2077 es al revés, y por eso los dos no se confunden.
    if theme.dark {
        let mut y = r.top() + 2.0;
        let raya = egui::Stroke::new(1.0, Color32::from(theme.border).gamma_multiply(0.13));
        while y < r.bottom() {
            ui.painter()
                .line_segment([Pos2::new(r.left() + 1.0, y), Pos2::new(r.right() - 1.0, y)], raya);
            y += 4.0;
        }
    }
}

/// La etiqueta de un módulo: ámbar, versalitas y muy espaciada.
///
/// El ámbar es de **lectura** y el acento de **mando**: todo lo que informa va
/// en uno y todo lo que se toca en el otro. Nunca se cruzan, y por eso no hay
/// que aprender nada.
fn holo_label(ui: &Ui, r: Rect, theme: &Theme, texto: &str) {
    // En la variante clara la etiqueta va sobre una ranura hundida, como las
    // bancadas de interruptores rotuladas de una cabina. En la proyección no:
    // ahí un recuadro macizo rompería la idea de que el panel es luz.
    if !theme.dark {
        let barra = Rect::from_min_max(
            Pos2::new(r.left() + 5.0, r.top() + 3.0),
            Pos2::new(r.right() - 5.0, r.bottom() - 3.0),
        );
        ui.painter().rect_filled(barra, 1.0, Color32::from(theme.surface_alt));
    }
    let t = texto.to_uppercase();
    let n = t.chars().count() as f32;
    let paso = (r.width() - 8.0) / n.max(1.0);
    for (i, ch) in t.chars().enumerate() {
        ui.painter().text(
            Pos2::new(r.left() + 4.0 + paso * (i as f32 + 0.5), r.center().y),
            Align2::CENTER_CENTER,
            ch,
            FontId::monospace(theme.font_size - 4.0),
            theme.accent_hot.into(),
        );
    }
}

/// Un botón de la consola.
fn holo_btn(ui: &mut Ui, theme: &Theme, r: Rect, id: usize, on: bool) -> Response {
    // `usize` y no un genérico: la clave de egui pide `Hash` **y** `Debug`, y
    // acá alcanza con un número distinto por botón.
    let resp = ui.interact(r, ui.id().with(("hb", id)), Sense::click());
    if on {
        ui.painter().rect_filled(r, 0.0, Color32::from(theme.accent));
    } else if resp.hovered() {
        ui.painter().rect_stroke(
            r,
            0.0,
            egui::Stroke::new(1.0, Color32::from(theme.accent_hot)),
            egui::StrokeKind::Inside,
        );
    }
    resp
}

fn holo_ink(theme: &Theme, on: bool, hover: bool) -> Color32 {
    if on {
        Color32::from(theme.accent_text)
    } else if hover {
        Color32::from(theme.accent_hot)
    } else {
        Color32::from(theme.text)
    }
}

/// La consola y el lector de SW, flotando sobre el lienzo.
///
/// La geometría está declarada de arriba abajo en vez de dejar que fluya: un
/// panel con las esquinas cortadas hay que pintarlo **antes** que su contenido,
/// y en modo inmediato el tamaño no se sabe hasta después de dibujarlo.
pub fn holo_overlay(ctx: &egui::Context, theme: &Theme, doc: &mut Doc, out: &mut UiOut) {
    const ANCHO: f32 = 78.0;
    const BOTON: f32 = 30.0;
    const LAB: f32 = 18.0;
    const CELDA: f32 = 13.0;

    // --- la consola ---
    // `movable` en vez de `anchor`: los dos paneles se agarran de cualquier
    // parte que no sea un botón —el fondo, los márgenes, las etiquetas— y se
    // llevan a donde molesten menos. egui les guarda la posición sola.
    //
    // `constrain` para que no se puedan tirar fuera de la ventana y quedar
    // inalcanzables, que es la forma clásica de romper un panel flotante.
    egui::Area::new(egui::Id::new("sw_consola"))
        .default_pos(Pos2::new(16.0, 16.0))
        .movable(true)
        .constrain(true)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            const FILAS: f32 = 4.0;
            // El alto se declara entero antes de dibujar nada: un panel con las
            // esquinas cortadas hay que pintarlo **antes** que su contenido, y
            // en modo inmediato el tamaño no se sabe hasta después.
            let alto = 9.0 + LAB + FILAS * (BOTON + 2.0) + LAB + 16.0
                + LAB + 26.0 + 4.0 + 5.0 * (CELDA + 1.0)
                + 16.0 + BOTON + 9.0;
            let (r, _) = ui.allocate_exact_size(vec2(ANCHO, alto), Sense::hover());
            holo_panel(ui, r, theme, false);

            let mut y = r.top() + 9.0;
            let par = |i: usize| Pos2::new(r.center().x - BOTON - 1.0 + (BOTON + 2.0) * i as f32, 0.0);

            holo_label(ui, Rect::from_min_size(Pos2::new(r.left(), y), vec2(ANCHO, LAB)), theme, lang::t("Trazo"));
            y += LAB;
            const TRAZO: [[Tool; 2]; 3] = [
                [Tool::Pencil, Tool::Brush],
                [Tool::Fill, Tool::Text],
                [Tool::Shape, Tool::Eraser],
            ];
            for fila in TRAZO {
                for (i, t) in fila.iter().enumerate() {
                    let b = Rect::from_min_size(Pos2::new(par(i).x, y), vec2(BOTON, BOTON));
                    let on = doc.tool == *t;
                    let resp = holo_btn(ui, theme, b, *t as usize, on);
                    tool_icon(ui, b.shrink(7.0), *t, holo_ink(theme, on, resp.hovered()));
                    if resp.on_hover_text(t.label()).clicked() {
                        doc.set_tool(*t);
                    }
                }
                y += BOTON + 2.0;
            }

            holo_label(ui, Rect::from_min_size(Pos2::new(r.left(), y), vec2(ANCHO, LAB)), theme, lang::t("Ver"));
            y += LAB;
            for (i, t) in [Tool::Picker, Tool::Magnifier].iter().enumerate() {
                let b = Rect::from_min_size(Pos2::new(par(i).x, y), vec2(BOTON, BOTON));
                let on = doc.tool == *t;
                let resp = holo_btn(ui, theme, b, 90 + i, on);
                tool_icon(ui, b.shrink(7.0), *t, holo_ink(theme, on, resp.hovered()));
                if resp.on_hover_text(t.label()).clicked() {
                    doc.set_tool(*t);
                }
            }
            y += BOTON + 2.0;

            y += 8.0;
            ui.painter().line_segment(
                [Pos2::new(r.left() + 16.0, y), Pos2::new(r.right() - 16.0, y)],
                egui::Stroke::new(1.0, Color32::from(theme.border).gamma_multiply(0.5)),
            );
            y += 8.0;

            holo_label(ui, Rect::from_min_size(Pos2::new(r.left(), y), vec2(ANCHO, LAB)), theme, lang::t("Color"));
            y += LAB;
            for (i, es1) in [true, false].into_iter().enumerate() {
                let b = Rect::from_min_size(
                    Pos2::new(r.center().x - 25.0 + 26.0 * i as f32, y),
                    vec2(24.0, 24.0),
                );
                ui.painter()
                    .rect_filled(b, 0.0, if es1 { doc.color1 } else { doc.color2 });
                let elegido = out.picking_c1 == es1;
                ui.painter().rect_stroke(
                    if elegido { b.expand(2.0) } else { b },
                    0.0,
                    egui::Stroke::new(
                        1.0,
                        Color32::from(if elegido { theme.accent } else { theme.border }),
                    ),
                    egui::StrokeKind::Outside,
                );
                if ui.interact(b, ui.id().with(("hc", i)), Sense::click()).clicked() {
                    out.picking_c1 = es1;
                }
            }
            y += 26.0 + 4.0;

            // La paleta en cuatro columnas: es lo que entra en 78 px de ancho.
            let cols = 4;
            for (i, rgb) in PALETTE.iter().enumerate() {
                let c = Rect::from_min_size(
                    Pos2::new(
                        r.center().x - 26.0 + (i % cols) as f32 * (CELDA + 1.0),
                        y + (i / cols) as f32 * (CELDA + 1.0),
                    ),
                    vec2(CELDA, CELDA),
                );
                ui.painter()
                    .rect_filled(c, 0.0, Color32::from_rgb(rgb[0], rgb[1], rgb[2]));
                let resp = ui.interact(c, ui.id().with(("hp", i)), Sense::click());
                if resp.hovered() {
                    ui.painter().rect_stroke(
                        c,
                        0.0,
                        egui::Stroke::new(1.5, Color32::from(theme.accent_hot)),
                        egui::StrokeKind::Outside,
                    );
                }
                if resp.clicked() {
                    apply_palette_hit(PaletteHit::Fixed(i), doc, out);
                }
            }
            y += 5.0 * (CELDA + 1.0) + 8.0;

            ui.painter().line_segment(
                [Pos2::new(r.left() + 16.0, y), Pos2::new(r.right() - 16.0, y)],
                egui::Stroke::new(1.0, Color32::from(theme.border).gamma_multiply(0.5)),
            );
            y += 8.0;

            // Archivo y Configuración al pie de la consola. Sin esto el tema no
            // tenía **ninguna** puerta a los dos: no hay barra de menú ni cinta
            // donde ponerlos, y era el mismo agujero que tenía macOS.
            for (i, es_menu) in [true, false].into_iter().enumerate() {
                let b = Rect::from_min_size(Pos2::new(par(i).x, y), vec2(BOTON, BOTON));
                let resp = holo_btn(ui, theme, b, 80 + i, false);
                let col = holo_ink(theme, false, resp.hovered());
                if es_menu {
                    // Tres rayas: el menú Archivo.
                    for k in 0..3 {
                        let yy = b.center().y + (k as f32 - 1.0) * 4.5;
                        ui.painter().line_segment(
                            [Pos2::new(b.left() + 8.0, yy), Pos2::new(b.right() - 8.0, yy)],
                            egui::Stroke::new(1.5, col),
                        );
                    }
                } else {
                    small_icon(ui, b.shrink(6.0), Ico::Settings, col);
                }
                let etiqueta = if es_menu { lang::t("Archivo") } else { lang::t("Configuración") };
                if resp.on_hover_text(etiqueta).clicked() {
                    if es_menu {
                        out.open_file_menu = true;
                    } else {
                        out.open_settings = true;
                    }
                }
            }
        });

    // --- el lector ---
    //
    // Sitio nuevo, no un adorno: en los demás temas el tamaño, el cursor y el
    // zoom viven apretados en la barra de estado.
    egui::Area::new(egui::Id::new("sw_lector"))
        // `content_rect` y no `screen_rect`: en 0.36 el rectángulo de la
        // ventana vive en el estado de entrada, no en el contexto.
        .default_pos(Pos2::new(
            ctx.input(|i| i.content_rect()).right() - 184.0,
            16.0,
        ))
        .movable(true)
        .constrain(true)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            let filas = [
                (lang::t("Tamaño"), format!("{} × {}", doc.canvas.w, doc.canvas.h)),
                (lang::t("Grosor"), format!("{} px", doc.width as i32)),
                (lang::t("Zoom"), "100 %".to_string()),
            ];
            let (r, _) = ui.allocate_exact_size(vec2(168.0, 66.0), Sense::hover());
            holo_panel(ui, r, theme, true);
            for (i, (etiqueta, valor)) in filas.iter().enumerate() {
                let y = r.top() + 15.0 + i as f32 * 17.0;
                ui.painter().text(
                    Pos2::new(r.left() + 14.0, y),
                    Align2::LEFT_CENTER,
                    etiqueta,
                    FontId::monospace(theme.font_size - 3.0),
                    theme.text_dim.into(),
                );
                ui.painter().text(
                    Pos2::new(r.right() - 14.0, y),
                    Align2::RIGHT_CENTER,
                    valor,
                    FontId::monospace(theme.font_size - 2.0),
                    theme.accent_hot.into(),
                );
            }
        });
}

// ------------------------------------------------------------------ menús

/// Una tanda de renglones sueltos de menú. La usan los chromes de barra
/// clásica (XP, Linux, macOS), donde no hay cinta y todo vive en menús.
fn menu_items(ui: &mut Ui, out: &mut UiOut, items: &[(&str, Cmd)]) {
    for (label, cmd) in items {
        if ui.button(*label).clicked() {
            out.cmds.push(*cmd);
            ui.close();
        }
    }
}

fn file_menu(ui: &mut Ui, out: &mut UiOut) {
    ui.menu_button(lang::t("Archivo"), |ui| {
        menu_items(ui, out, &[
            (lang::t("Nuevo"), Cmd::New),
            (lang::t("Abrir…"), Cmd::Open),
            (lang::t("Guardar"), Cmd::Save),
            (lang::t("Guardar como…"), Cmd::SaveAs),
        ]);
        ui.separator();
        menu_items(ui, out, &[
            (lang::t("Propiedades…"), Cmd::PropertiesDialog),
            (lang::t("Acerca de Lienzo"), Cmd::About),
        ]);
        ui.separator();
        menu_items(ui, out, &[(lang::t("Salir"), Cmd::Exit)]);
    });
}

fn edit_menu(ui: &mut Ui, out: &mut UiOut) {
    ui.menu_button(lang::t("Edición"), |ui| {
        menu_items(ui, out, &[
            (lang::t("Deshacer"), Cmd::Undo),
            (lang::t("Rehacer"), Cmd::Redo),
            (lang::t("Cortar"), Cmd::Cut),
            (lang::t("Copiar"), Cmd::Copy),
            (lang::t("Pegar"), Cmd::Paste),
            (lang::t("Seleccionar todo"), Cmd::SelectAll),
        ]);
    });
}

fn image_menu(ui: &mut Ui, out: &mut UiOut) {
    ui.menu_button(lang::t("Imagen"), |ui| {
        menu_items(ui, out, &[
            (lang::t("Recortar"), Cmd::Crop),
            (lang::t("Cambiar tamaño…"), Cmd::ResizeDialog),
            (lang::t("Girar 90° a la derecha"), Cmd::Rotate(1)),
            (lang::t("Girar 90° a la izquierda"), Cmd::Rotate(3)),
            (lang::t("Girar 180°"), Cmd::Rotate(2)),
            (lang::t("Voltear horizontalmente"), Cmd::FlipH),
            (lang::t("Voltear verticalmente"), Cmd::FlipV),
            (lang::t("Invertir colores"), Cmd::InvertColors),
        ]);
    });
}

/// El menú Colores, que en Paint clásico está entre Imagen y Ayuda.
fn colors_menu(ui: &mut Ui, out: &mut UiOut) {
    ui.menu_button(lang::t("Colores"), |ui| {
        if ui.button(lang::t("Editar colores…")).clicked() {
            out.open_color_dialog = true;
            ui.close();
        }
    });
}

/// El menú Ayuda: en Windows es siempre el último de la barra.
fn help_menu(ui: &mut Ui, out: &mut UiOut) {
    ui.menu_button(lang::t("Ayuda"), |ui| {
        menu_items(ui, out, &[(lang::t("Acerca de Lienzo"), Cmd::About)]);
    });
}

fn view_menu(ui: &mut Ui, themes: &[(String, usize)], out: &mut UiOut) {
    ui.menu_button("Ver", |ui| {
        menu_items(ui, out, &[
            (lang::t("Acercar"), Cmd::ZoomIn),
            (lang::t("Alejar"), Cmd::ZoomOut),
            (lang::t("100%"), Cmd::Zoom100),
        ]);
        ui.separator();
        menu_items(ui, out, &[(lang::t("Líneas de cuadrícula"), Cmd::ToggleGrid)]);
        ui.separator();
        ui.menu_button(lang::t("Tema"), |ui| {
            for (name, i) in themes {
                if ui.button(name).clicked() {
                    out.cmds.push(Cmd::SetTheme(*i));
                    ui.close();
                }
            }
        });
    });
}
