//! El cuadro de texto que se edita sobre el lienzo, y su volcado a píxeles.
//!
//! Vive del lado de la interfaz —no en `canvas.rs`— porque medir y rasterizar
//! letras es trabajo de las fuentes de egui. El motor sólo recibe píxeles ya
//! resueltos, así que sigue compilando y testeándose sin ventana.

use crate::canvas::{Canvas, Rect as CRect};
use egui::{Color32, Context, FontFamily, FontId};

/// Las familias que egui trae cargadas.
///
/// `ponytail:` son las dos de fábrica. Para ofrecer las fuentes instaladas en
/// la máquina hay que sumar `fontdb`, leerlas al arrancar y registrarlas en
/// `FontDefinitions`; el resto de este archivo no cambia, sólo crece esta lista.
pub const FAMILIES: [&str; 2] = ["Proporcional", "Monoespaciada"];

/// Los tamaños del desplegable, en píxeles del lienzo.
pub const SIZES: [f32; 12] = [
    8.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 24.0, 28.0, 36.0, 48.0,
];

pub fn family(i: usize) -> FontFamily {
    if i == 1 {
        FontFamily::Monospace
    } else {
        FontFamily::Proportional
    }
}

/// Un cuadro más chico que esto no se puede escribir ni agarrar por las manijas.
pub const MIN_W: f32 = 24.0;
pub const MIN_H: f32 = 14.0;

/// El cuadro que se está editando. Vive en la aplicación y no en el documento
/// a propósito: hasta que se confirma no tocó un solo píxel, así que cancelar
/// no tiene nada que deshacer.
#[derive(Clone, Debug)]
pub struct TextBox {
    /// Esquina y tamaño, en píxeles del lienzo.
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub s: String,
    pub family: usize,
    /// Alto de la fuente en píxeles del lienzo.
    pub size: f32,
    /// Con fondo opaco, el cuadro entero se rellena con el Color 2 y borra lo
    /// que había debajo. Transparente estampa sólo los píxeles de las letras.
    pub opaque: bool,
}

impl TextBox {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            x,
            y,
            w: w.max(MIN_W),
            h: h.max(MIN_H),
            s: String::new(),
            family: 0,
            size: 18.0,
            opaque: false,
        }
    }

    /// El cuadro que sale de un clic sin arrastre, en vez de uno de cero píxeles.
    pub fn default_at(x: f32, y: f32) -> Self {
        Self::new(x, y, 180.0, 44.0)
    }
}

/// La fuente con la que se mide y se dibuja. `scale` vale 1.0 para el volcado
/// al lienzo y el zoom para la vista previa en pantalla.
///
/// Se divide por `pixels_per_point` porque egui trabaja en puntos y nosotros en
/// píxeles: en una pantalla HiDPI, pedir 18 puntos rasteriza 36 téxeles, y al
/// copiarlos uno a uno al lienzo el texto saldría del doble de alto.
pub fn font_id(tb: &TextBox, ctx: &Context, scale: f32) -> FontId {
    let px = (tb.size * scale / ctx.pixels_per_point()).max(1.0);
    FontId::new(px, family(tb.family))
}

/// Vuelca el texto al lienzo: un solo paso de deshacer para todo el cuadro.
///
/// `ink` es el Color 1 y `paper` el Color 2, la misma regla que las formas.
pub fn rasterize(ctx: &Context, c: &mut Canvas, tb: &TextBox, ink: Color32, paper: Color32) {
    let ppp = ctx.pixels_per_point();
    let font = font_id(tb, ctx, 1.0);
    let galley = ctx.fonts_mut(|f| f.layout(tb.s.clone(), font, ink, tb.w / ppp));
    // El atlas de fuentes: una imagen donde cada glifo ya rasterizado ocupa un
    // recorte. `layout` lo acaba de poblar con los que hagan falta.
    let atlas = ctx.fonts(|f| f.image());
    let aw = atlas.size[0];

    c.begin_stroke();

    if tb.opaque {
        if let Some(r) = CRect::from_corners(
            (tb.x.floor() as i32, tb.y.floor() as i32),
            ((tb.x + tb.w).ceil() as i32, (tb.y + tb.h).ceil() as i32),
            c.w,
            c.h,
        ) {
            c.clear_region(r, paper);
        }
    }

    for row in &galley.rows {
        for g in &row.row.glyphs {
            if g.uv_rect.is_nothing() {
                continue;
            }
            // La misma cuenta que hace el tesselador de egui para ponerlo en
            // pantalla: posición del glifo más el desfase que guarda su recorte.
            let lx = tb.x + (row.pos.x + g.pos.x + g.uv_rect.offset.x) * ppp;
            let ly = tb.y + (row.pos.y + g.pos.y + g.uv_rect.offset.y) * ppp;
            let (u0, v0) = (g.uv_rect.min[0] as usize, g.uv_rect.min[1] as usize);
            let gw = g.uv_rect.max[0] as usize - u0;
            let gh = g.uv_rect.max[1] as usize - v0;

            for dy in 0..gh {
                for dx in 0..gw {
                    // El atlas guarda la **cobertura en el alfa**; el color lo
                    // pone quien dibuja. Por eso se mezcla, no se copia: sin
                    // eso las letras salen con los bordes dentados.
                    let a = atlas.pixels[(v0 + dy) * aw + u0 + dx].a();
                    if a > 0 {
                        c.blend(
                            lx.round() as i32 + dx as i32,
                            ly.round() as i32 + dy as i32,
                            ink,
                            a as f32 / 255.0,
                        );
                    }
                }
            }
        }
    }

    c.end_stroke();
}
