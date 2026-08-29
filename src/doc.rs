//! El documento y la lógica de las herramientas.
//!
//! Capa pura: opera sobre `Canvas` y no sabe que existe egui. Recibe eventos ya
//! traducidos a coordenadas del lienzo (`down` / `drag` / `up`), así que se
//! puede testear sin ventana.

use crate::canvas::{Canvas, Rect};
use crate::shapes::{self, Brush, Pt, Rng, Shape};
use ecolor::Color32;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush,
    Shape,
    Select,
}

impl Tool {
    /// El nombre que ve el usuario, ya traducido.
    ///
    /// La traducción va **acá adentro** y no en cada sitio que lo dibuja: son
    /// trece llamadas repartidas en dos archivos, y con envolverlas una por una
    /// la que se agregue mañana se olvida. `lang` es una tabla de texto de la
    /// biblioteca estándar, así que el motor sigue sin saber que egui existe.
    pub fn label(self) -> &'static str {
        crate::lang::t(match self {
            Self::Pencil => "Lápiz",
            Self::Fill => "Relleno con color",
            Self::Text => "Texto",
            Self::Eraser => "Borrador",
            Self::Picker => "Selector de color",
            Self::Magnifier => "Lupa",
            Self::Brush => "Pinceles",
            Self::Shape => "Formas",
            Self::Select => "Seleccionar",
        })
    }
}

/// Estilo de contorno y de relleno de las formas. Paint ofrece los mismos siete
/// en los dos menús, salvo la primera entrada.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stroke {
    None,
    Solid,
    Crayon,
    Marker,
    Oil,
    NaturalPencil,
    Watercolor,
}

pub const ALL_STROKES: [Stroke; 7] = [
    Stroke::None,
    Stroke::Solid,
    Stroke::Crayon,
    Stroke::Marker,
    Stroke::Oil,
    Stroke::NaturalPencil,
    Stroke::Watercolor,
];

impl Stroke {
    pub fn outline_label(self) -> &'static str {
        match self {
            Self::None => "Sin contorno",
            Self::Solid => "Color sólido",
            Self::Crayon => "Crayón",
            Self::Marker => "Marcador",
            Self::Oil => "Óleo",
            Self::NaturalPencil => "Lápiz natural",
            Self::Watercolor => "Acuarela",
        }
    }

    pub fn fill_label(self) -> &'static str {
        match self {
            Self::None => "Sin relleno",
            other => other.outline_label(),
        }
    }

    /// Con qué pincel se dibuja este estilo. `None` para los que no son
    /// texturados. Estaba enterrado adentro de `stroke_pts`; sacarlo permite
    /// que la galería previsualice cada estilo con el motor de verdad.
    pub fn brush(self) -> Option<Brush> {
        match self {
            Self::Crayon => Some(Brush::Crayon),
            Self::Marker => Some(Brush::Marker),
            Self::Oil => Some(Brush::Oil),
            Self::NaturalPencil => Some(Brush::NaturalPencil),
            Self::Watercolor => Some(Brush::Watercolor),
            Self::None | Self::Solid => None,
        }
    }

    /// Opacidad con la que se aplica. Los estilos texturados son translúcidos.
    fn alpha(self) -> f32 {
        match self {
            Self::None => 0.0,
            Self::Solid => 1.0,
            Self::Oil => 0.85,
            Self::Crayon => 0.55,
            Self::Marker => 0.35,
            Self::NaturalPencil => 0.40,
            Self::Watercolor => 0.30,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SelectMode {
    Rectangular,
    FreeForm,
}

/// Una selección. `px` presente significa que está *levantada* (flotando): los
/// píxeles ya se sacaron del lienzo y se mueven con ella.
pub struct Selection {
    pub r: Rect,
    pub px: Option<Vec<Color32>>,
    /// Máscara de forma libre, en coordenadas del lienzo. `None` = rectangular.
    pub lasso: Option<Vec<Pt>>,
    /// El tamaño con el que se levantaron los píxeles.
    ///
    /// `r` cambia al estirar la selección; `px` **no**. Se remuestrea al leerlo
    /// y siempre desde el original, porque reescalar sobre lo ya reescalado
    /// pierde detalle en cada tirón: agrandar y volver a achicar dejaría un
    /// borrón en vez de la imagen de antes.
    pub src: (usize, usize),
}

impl Selection {
    /// Los píxeles al tamaño que tiene `r` ahora.
    pub fn pixels(&self) -> Option<Vec<Color32>> {
        let px = self.px.as_ref()?;
        if (self.r.w, self.r.h) == self.src {
            return Some(px.clone());
        }
        let (sw, sh) = self.src;
        if sw == 0 || sh == 0 {
            return Some(px.clone());
        }
        // Vecino más cercano, igual que `Canvas::scale`: en un editor de píxeles
        // interpolar sería mentir sobre lo que hay.
        let mut out = Vec::with_capacity(self.r.w * self.r.h);
        for y in 0..self.r.h {
            let sy = y * sh / self.r.h;
            for x in 0..self.r.w {
                let sx = x * sw / self.r.w;
                out.push(px[sy * sw + sx]);
            }
        }
        Some(out)
    }

    /// Dónde está la manija `i`, en coordenadas del lienzo.
    pub fn handle(&self, i: usize) -> Pt {
        let (hx, hy) = SEL_HANDLES[i];
        (
            self.r.x as f32 + hx * self.r.w as f32,
            self.r.y as f32 + hy * self.r.h as f32,
        )
    }
}

/// Las ocho manijas en coordenadas de la caja: 0 es un borde y 1 el opuesto.
/// Con 0.5 ese eje no se mueve, así que estirar es una sola cuenta para las
/// ocho en vez de ocho casos distintos.
pub const SEL_HANDLES: [(f32, f32); 8] = [
    (0.0, 0.0), (0.5, 0.0), (1.0, 0.0), (1.0, 0.5),
    (1.0, 1.0), (0.5, 1.0), (0.0, 1.0), (0.0, 0.5),
];

/// Lo que está pasando con el puntero en este momento.
enum Drag {
    None,
    Freehand { last: Pt },
    Shape { a: Pt, b: Pt },
    /// La curva de Paint: primero el segmento, después dos tirones.
    Curve { a: Pt, b: Pt, ctrl: Option<Pt> },
    Polygon { pts: Vec<Pt> },
    SelectNew { a: Pt, lasso: Vec<Pt> },
    SelectMove { grab: Pt, origin: (usize, usize) },
    /// Estirando la selección por una manija. `orig` es la caja de antes de
    /// empezar: sin ella cada frame estiraría sobre lo estirado y el tirón se
    /// aceleraría solo.
    SelectScale { handle: usize, orig: Rect },
}

pub struct Doc {
    pub canvas: Canvas,
    pub tool: Tool,
    pub brush: Brush,
    pub shape: Shape,
    pub outline: Stroke,
    pub fill_style: Stroke,
    pub select_mode: SelectMode,
    /// Toggle global de Paint, no propiedad de cada selección.
    pub transparent_selection: bool,
    pub width: f32,
    pub color1: Color32,
    pub color2: Color32,
    pub sel: Option<Selection>,
    pub clipboard: Option<(usize, usize, Vec<Color32>)>,
    /// true mientras se dibuja con el botón derecho: invierte los colores.
    swap: bool,
    drag: Drag,
    rng: Rng,
    /// Vista previa de la forma en curso, para que la UI la dibuje encima.
    pub preview: Vec<Pt>,
    pub preview_closed: bool,
}

impl Doc {
    pub fn new(w: usize, h: usize) -> Self {
        Self {
            canvas: Canvas::new(w, h),
            tool: Tool::Pencil,
            brush: Brush::Round,
            shape: Shape::Line,
            outline: Stroke::Solid,
            fill_style: Stroke::None,
            select_mode: SelectMode::Rectangular,
            transparent_selection: false,
            width: 1.0,
            color1: Color32::BLACK,
            color2: Color32::WHITE,
            sel: None,
            clipboard: None,
            swap: false,
            drag: Drag::None,
            rng: Rng::new(),
            preview: Vec::new(),
            preview_closed: false,
        }
    }

    /// Color de trazo, ya considerando si se está usando el botón derecho.
    pub fn fg(&self) -> Color32 {
        if self.swap { self.color2 } else { self.color1 }
    }

    /// El otro color. Es con el que borra el borrador y con el que se rellena
    /// el hueco al mover una selección.
    pub fn bg(&self) -> Color32 {
        if self.swap { self.color1 } else { self.color2 }
    }

    pub fn set_tool(&mut self, t: Tool) {
        if self.tool != t {
            // Sin esto, una curva o un polígono a medio hacer queda armado: el
            // primer clic de la herramienta nueva lo consume `continue_multistep`
            // y además deja abierto el `begin_stroke` que nadie cierra.
            self.finish_multistep();
            self.commit_selection();
            self.tool = t;
        }
    }

    // ------------------------------------------------------------ selección

    pub fn select_all(&mut self) {
        self.commit_selection();
        self.tool = Tool::Select;
        self.sel = Some(Selection {
            r: Rect::new(0, 0, self.canvas.w, self.canvas.h),
            px: None,
            lasso: None,
            src: (self.canvas.w, self.canvas.h),
        });
    }

    /// Saca los píxeles de la selección del lienzo y los deja flotando.
    fn lift(&mut self) {
        let Some(sel) = &mut self.sel else { return };
        if sel.px.is_some() {
            return;
        }
        let mut px = self.canvas.region(sel.r);
        // En forma libre, lo de afuera del lazo no viaja con la selección.
        if let Some(poly) = &sel.lasso {
            let bg = if self.swap { self.color1 } else { self.color2 };
            mask_outside(&mut px, sel.r, poly, bg);
        }
        sel.px = Some(px);
        sel.src = (sel.r.w, sel.r.h);
        let r = sel.r;
        let bg = if self.swap { self.color1 } else { self.color2 };
        self.canvas.clear_region(r, bg);
    }

    /// Baja la selección flotante al lienzo. Es idempotente.
    pub fn commit_selection(&mut self) {
        let Some(sel) = self.sel.take() else { return };
        // `pixels` y no `px`: si la estiraste, hay que volcar el tamaño nuevo.
        let Some(px) = sel.pixels() else { return };
        // Si mover la selección ya dejó un trazo abierto, se reusa.
        if !self.canvas.stroke_open() {
            self.canvas.begin_stroke();
        }
        if self.transparent_selection {
            self.canvas.put_region_keyed(sel.r, &px, self.color2);
        } else {
            self.canvas.put_region(sel.r, &px);
        }
        self.canvas.end_stroke();
    }

    pub fn delete_selection(&mut self) {
        let Some(sel) = &self.sel else { return };
        let r = sel.r;
        let floating = sel.px.is_some();
        self.canvas.begin_stroke();
        if !floating {
            self.canvas.clear_region(r, self.color2);
        }
        self.canvas.end_stroke();
        self.sel = None;
    }

    pub fn copy_selection(&mut self) {
        let Some(sel) = &self.sel else { return };
        let px = sel.pixels().unwrap_or_else(|| self.canvas.region(sel.r));
        self.clipboard = Some((sel.r.w, sel.r.h, px));
    }

    pub fn cut_selection(&mut self) {
        self.copy_selection();
        self.delete_selection();
    }

    /// Pegar no es una función nueva: es crear una selección flotante en (0,0),
    /// que es la misma maquinaria que usa la herramienta Seleccionar.
    pub fn paste(&mut self, w: usize, h: usize, px: Vec<Color32>) {
        if w == 0 || h == 0 || px.len() != w * h {
            return;
        }
        self.commit_selection();
        self.tool = Tool::Select;
        // Se recorta al lienzo: Paint pregunta si agrandar el bitmap, y hasta
        // que exista ese diálogo lo pegado grande se recorta en vez de dejar un
        // rectángulo que se sale y hace estallar a `region`.
        let (cw, ch) = (w.min(self.canvas.w), h.min(self.canvas.h));
        let px = if (cw, ch) == (w, h) {
            px
        } else {
            (0..ch).flat_map(|y| px[y * w..y * w + cw].to_vec()).collect()
        };
        self.sel = Some(Selection {
            r: Rect::new(0, 0, cw, ch),
            px: Some(px),
            lasso: None,
            src: (cw, ch),
        });
    }

    pub fn paste_from_clipboard(&mut self) {
        if let Some((w, h, px)) = self.clipboard.clone() {
            self.paste(w, h, px);
        }
    }

    pub fn invert_selection_colors(&mut self) {
        self.commit_selection();
        let r = self.sel.as_ref().map(|s| s.r).unwrap_or(Rect::new(0, 0, self.canvas.w, self.canvas.h));
        self.canvas.begin_stroke();
        self.canvas.invert_region(r);
        self.canvas.end_stroke();
    }

    pub fn crop_to_selection(&mut self) {
        let Some(r) = self.sel.as_ref().map(|s| s.r) else { return };
        // Primero baja la selección: si está flotando, el lienzo tiene el hueco
        // y recortaríamos un rectángulo vacío.
        self.commit_selection();
        // Y recorta el rectángulo al lienzo: al mover o pegar puede haberse ido
        // afuera, y `region` indexa sin red.
        let x = r.x.min(self.canvas.w);
        let y = r.y.min(self.canvas.h);
        let r = Rect::new(
            x,
            y,
            r.w.min(self.canvas.w - x),
            r.h.min(self.canvas.h - y),
        );
        if r.area() == 0 {
            return;
        }
        let px = self.canvas.region(r);
        self.canvas.load(r.w, r.h, px);
        self.canvas.dirty_file = true;
        self.sel = None;
    }

    // --------------------------------------------------------- interacción

    /// `right` es true si se está usando el botón derecho.
    /// `grab` es el radio de agarre de las manijas **en píxeles del lienzo**.
    /// Se pasa desde afuera porque depende del zoom, y el documento no lo sabe.
    pub fn down(&mut self, p: Pt, right: bool, grab: f32) {
        self.swap = right;
        let ip = (p.0.round() as i32, p.1.round() as i32);

        // La curva y el polígono se dibujan en varios pasos, así que un clic
        // nuevo continúa el trazo en vez de empezar otro.
        if let Drag::Curve { .. } | Drag::Polygon { .. } = self.drag {
            self.continue_multistep(p);
            return;
        }

        match self.tool {
            Tool::Select => {
                // Se resuelve a un bool antes de seguir: dejar viva la
                // referencia a `self.sel` bloquearía la llamada a `lift`.
                // Las manijas primero: la esquina cae **dentro** de la caja,
                // así que probando el interior antes nunca se llegaría a ellas.
                let manija = self.sel.as_ref().and_then(|s| {
                    (0..8).find(|i| {
                        let h = s.handle(*i);
                        (p.0 - h.0).abs() <= grab && (p.1 - h.1).abs() <= grab
                    })
                });
                if let Some(i) = manija {
                    self.canvas.begin_stroke();
                    self.lift();
                    let orig = self.sel.as_ref().map(|s| s.r).unwrap_or(Rect::new(0, 0, 1, 1));
                    self.drag = Drag::SelectScale { handle: i, orig };
                    return;
                }

                let inside = self
                    .sel
                    .as_ref()
                    .is_some_and(|s| s.r.contains(ip.0, ip.1));
                if inside {
                    self.canvas.begin_stroke();
                    self.lift();
                    let origin = self.sel.as_ref().map(|s| (s.r.x, s.r.y)).unwrap_or((0, 0));
                    self.drag = Drag::SelectMove { grab: p, origin };
                    return;
                }
                self.commit_selection();
                self.drag = Drag::SelectNew { a: p, lasso: vec![p] };
            }

            Tool::Picker => {
                if let Some(c) = self.canvas.geti(ip.0, ip.1) {
                    if right {
                        self.color2 = c;
                    } else {
                        self.color1 = c;
                    }
                }
            }

            Tool::Magnifier => {} // el zoom lo maneja la UI

            Tool::Fill => {
                self.commit_selection();
                self.canvas.begin_stroke();
                self.canvas.fill(ip.0, ip.1, self.fg());
                self.canvas.end_stroke();
            }

            Tool::Text => {} // la UI abre el cuadro de texto

            Tool::Shape => {
                self.commit_selection();
                self.canvas.begin_stroke();
                self.drag = match self.shape {
                    Shape::Curve => Drag::Curve { a: p, b: p, ctrl: None },
                    Shape::Polygon => Drag::Polygon { pts: vec![p] },
                    _ => Drag::Shape { a: p, b: p },
                };
            }

            Tool::Pencil | Tool::Brush | Tool::Eraser => {
                self.commit_selection();
                self.canvas.begin_stroke();
                self.paint_at(p, p, right);
                self.drag = Drag::Freehand { last: p };
            }
        }
    }

    /// Se saca el estado del arrastre con `mem::replace` antes de tocar nada:
    /// si se lo dejara prestado, ningún brazo podría llamar a un método que
    /// necesite `self` entero.
    pub fn drag_to(&mut self, p: Pt, shift: bool) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::None => {}

            Drag::Freehand { last } => {
                let right = self.swap;
                self.paint_at(last, p, right);
                self.drag = Drag::Freehand { last: p };
            }

            Drag::Shape { a, .. } => {
                let b = if shift { constrain(a, p, self.shape) } else { p };
                self.update_shape_preview(a, b);
                self.drag = Drag::Shape { a, b };
            }

            Drag::Curve { a, b, ctrl } => {
                let (b, ctrl) = if ctrl.is_none() {
                    (if shift { constrain(a, p, Shape::Line) } else { p }, None)
                } else {
                    (b, Some(p))
                };
                self.preview = shapes::quadratic(a, ctrl.unwrap_or(mid(a, b)), b, 32);
                self.preview_closed = false;
                self.drag = Drag::Curve { a, b, ctrl };
            }

            Drag::Polygon { pts } => {
                let mut v = pts.clone();
                v.push(p);
                self.preview = v;
                self.preview_closed = false;
                self.drag = Drag::Polygon { pts };
            }

            Drag::SelectNew { a, mut lasso } => {
                let free = self.select_mode == SelectMode::FreeForm;
                if free {
                    lasso.push(p);
                }
                let poly = if free { Some(lasso.clone()) } else { None };
                if let Some(r) = Rect::from_corners(
                    (a.0 as i32, a.1 as i32),
                    (p.0 as i32, p.1 as i32),
                    self.canvas.w,
                    self.canvas.h,
                ) {
                    self.sel = Some(Selection { r, px: None, lasso: poly, src: (r.w, r.h) });
                }
                self.drag = Drag::SelectNew { a, lasso };
            }

            Drag::SelectScale { handle, orig } => {
                if let Some(sel) = self.sel.as_mut() {
                    let (hx, hy) = SEL_HANDLES[handle];
                    let (mut l, mut t) = (orig.x as f32, orig.y as f32);
                    let (mut r, mut b) = (l + orig.w as f32, t + orig.h as f32);
                    // El eje con 0.5 no se toca; los otros se topan contra 1 px
                    // para que la caja no se dé vuelta al pasarse de largo.
                    if hx == 0.0 {
                        l = p.0.min(r - 1.0);
                    } else if hx == 1.0 {
                        r = p.0.max(l + 1.0);
                    }
                    if hy == 0.0 {
                        t = p.1.min(b - 1.0);
                    } else if hy == 1.0 {
                        b = p.1.max(t + 1.0);
                    }
                    sel.r.x = l.max(0.0) as usize;
                    sel.r.y = t.max(0.0) as usize;
                    sel.r.w = ((r - l).round() as usize).max(1);
                    sel.r.h = ((b - t).round() as usize).max(1);
                }
                self.drag = Drag::SelectScale { handle, orig };
            }

            Drag::SelectMove { grab, origin } => {
                let (dx, dy) = (p.0 - grab.0, p.1 - grab.1);
                // Con tope arriba también: sin él la selección se puede sacar
                // del lienzo sin vuelta, y `region` termina leyendo fuera.
                let (mw, mh) = (self.canvas.w as f32, self.canvas.h as f32);
                if let Some(sel) = &mut self.sel {
                    sel.r.x = (origin.0 as f32 + dx).clamp(0.0, (mw - 1.0).max(0.0)) as usize;
                    sel.r.y = (origin.1 as f32 + dy).clamp(0.0, (mh - 1.0).max(0.0)) as usize;
                }
                self.drag = Drag::SelectMove { grab, origin };
            }
        }
    }

    pub fn up(&mut self, _p: Pt) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Freehand { .. } => {
                self.canvas.end_stroke();
            }

            Drag::Shape { a, b } => {
                self.rasterize_shape(a, b);
                self.canvas.end_stroke();
                self.preview.clear();
            }

            // La curva y el polígono siguen vivos tras soltar: se reponen.
            Drag::Curve { a, b, ctrl } => {
                self.drag = Drag::Curve { a, b, ctrl };
            }
            Drag::Polygon { pts } => {
                self.drag = Drag::Polygon { pts };
            }

            Drag::SelectNew { .. } => {
                // Un clic sin arrastre deselecciona.
                if let Some(s) = &self.sel {
                    if s.r.w < 2 || s.r.h < 2 {
                        self.sel = None;
                    }
                }
            }

            // No se cierra acá: el trazo que abrió `down` al levantarla queda
            // abierto y lo cierra `commit_selection`, así el hueco y lo que se
            // baja encima entran en un solo paso del historial.
            Drag::SelectMove { .. } | Drag::SelectScale { .. } => {}
            Drag::None => {}
        }
        self.swap = false;
    }

    /// Continúa una curva o un polígono con un clic más.
    fn continue_multistep(&mut self, p: Pt) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Curve { a, b, ctrl } => {
                if ctrl.is_none() {
                    self.drag = Drag::Curve { a, b, ctrl: Some(p) };
                } else {
                    // Segundo tirón: se confirma.
                    let pts = shapes::quadratic(a, p, b, 48);
                    self.stroke_pts(&pts, false);
                    self.canvas.end_stroke();
                    self.preview.clear();
                }
            }
            Drag::Polygon { mut pts } => {
                // Un clic sobre el primer punto cierra el polígono.
                let close = pts.len() > 2 && dist(p, pts[0]) < 8.0;
                if close {
                    let v = pts.clone();
                    self.rasterize_pts(&v, true);
                    self.canvas.end_stroke();
                    self.preview.clear();
                } else {
                    pts.push(p);
                    self.drag = Drag::Polygon { pts };
                }
            }
            other => self.drag = other,
        }
    }

    /// Cierra una curva o un polígono a medio hacer (doble clic, Enter, Escape).
    pub fn finish_multistep(&mut self) {
        match std::mem::replace(&mut self.drag, Drag::None) {
            Drag::Curve { a, b, ctrl } => {
                let pts = shapes::quadratic(a, ctrl.unwrap_or(mid(a, b)), b, 48);
                self.stroke_pts(&pts, false);
                self.canvas.end_stroke();
            }
            Drag::Polygon { pts } => {
                if pts.len() > 2 {
                    self.rasterize_pts(&pts, true);
                }
                self.canvas.end_stroke();
            }
            other => self.drag = other,
        }
        self.preview.clear();
    }

    pub fn is_multistep_active(&self) -> bool {
        matches!(self.drag, Drag::Curve { .. } | Drag::Polygon { .. })
    }

    // ------------------------------------------------------------ pintado

    fn paint_at(&mut self, a: Pt, b: Pt, right: bool) {
        let col = self.fg();
        match self.tool {
            Tool::Pencil => {
                // El lápiz de Paint es duro y sin antialiasing: es lo que hace
                // que el balde de relleno funcione sin dejar halos.
                shapes::line(&mut self.canvas, a, b, self.width, col);
            }
            Tool::Brush => {
                let (brush, width) = (self.brush, self.width.max(3.0));
                let mut rng = std::mem::take(&mut self.rng);
                shapes::brush_stroke(&mut self.canvas, brush, a, b, width, col, &mut rng);
                self.rng = rng;
            }
            Tool::Eraser => {
                let w = self.width.max(4.0);
                if right {
                    // Borrador selectivo: sólo reemplaza el Color 1 por el 2.
                    let r = (w / 2.0).ceil() as usize + 1;
                    let cx = b.0.max(0.0) as usize;
                    let cy = b.1.max(0.0) as usize;
                    let rect = Rect::new(
                        cx.saturating_sub(r),
                        cy.saturating_sub(r),
                        r * 2 + 1,
                        r * 2 + 1,
                    );
                    let (from, to) = (self.color1, self.color2);
                    self.canvas.replace_color_in(rect, from, to);
                } else {
                    // El nib del borrador es cuadrado, no redondo.
                    let bg = self.color2;
                    let steps = (b.0 - a.0).abs().max((b.1 - a.1).abs()).ceil().max(1.0);
                    for i in 0..=steps as i32 {
                        let t = i as f32 / steps;
                        shapes::stamp_square(
                            &mut self.canvas,
                            (a.0 + (b.0 - a.0) * t).round() as i32,
                            (a.1 + (b.1 - a.1) * t).round() as i32,
                            w,
                            bg,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    fn update_shape_preview(&mut self, a: Pt, b: Pt) {
        self.preview = self.shape.points(a.0, a.1, b.0, b.1);
        self.preview_closed = self.shape.is_closed();
    }

    fn rasterize_shape(&mut self, a: Pt, b: Pt) {
        let pts = self.shape.points(a.0, a.1, b.0, b.1);
        let closed = self.shape.is_closed();
        self.rasterize_pts(&pts, closed);
    }

    fn rasterize_pts(&mut self, pts: &[Pt], closed: bool) {
        // El relleno usa el Color 2 y va primero, para que el contorno lo tape.
        if closed && self.fill_style != Stroke::None {
            let a = self.fill_style.alpha();
            let col = self.bg();
            if a >= 1.0 {
                shapes::fill_polygon(&mut self.canvas, pts, col);
            } else {
                // Relleno translúcido: se pinta en un lienzo aparte y se mezcla.
                let mut tmp = Canvas::new(self.canvas.w, self.canvas.h);
                let key = Color32::from_rgb(1, 2, 3); // marca improbable
                tmp.clear_region(Rect::new(0, 0, tmp.w, tmp.h), key);
                shapes::fill_polygon(&mut tmp, pts, col);
                for y in 0..self.canvas.h as i32 {
                    for x in 0..self.canvas.w as i32 {
                        if tmp.geti(x, y) != Some(key) {
                            self.canvas.blend(x, y, col, a);
                        }
                    }
                }
            }
        }
        if self.outline != Stroke::None {
            self.stroke_pts(pts, closed);
        }
    }

    fn stroke_pts(&mut self, pts: &[Pt], closed: bool) {
        if self.outline == Stroke::None {
            return;
        }
        let col = self.fg();
        let w = self.width.max(1.0);
        if self.outline == Stroke::Solid {
            shapes::stroke_polyline(&mut self.canvas, pts, closed, w, col);
            return;
        }
        // Contornos texturados: se recorre con el pincel equivalente.
        let brush = self.outline.brush().unwrap_or(Brush::Watercolor);
        let mut rng = std::mem::take(&mut self.rng);
        let n = pts.len();
        for i in 0..n.saturating_sub(1) {
            shapes::brush_stroke(&mut self.canvas, brush, pts[i], pts[i + 1], w, col, &mut rng);
        }
        if closed && n > 1 {
            shapes::brush_stroke(&mut self.canvas, brush, pts[n - 1], pts[0], w, col, &mut rng);
        }
        self.rng = rng;
    }
}

fn mid(a: Pt, b: Pt) -> Pt {
    ((a.0 + b.0) / 2.0, (a.1 + b.1) / 2.0)
}

fn dist(a: Pt, b: Pt) -> f32 {
    ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
}

/// Shift en Paint: círculo o cuadrado perfecto, y líneas a múltiplos de 45°.
fn constrain(a: Pt, b: Pt, shape: Shape) -> Pt {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    if matches!(shape, Shape::Line | Shape::Curve) {
        let ang = (dy.atan2(dx) / std::f32::consts::FRAC_PI_4).round() * std::f32::consts::FRAC_PI_4;
        let d = (dx * dx + dy * dy).sqrt();
        (a.0 + ang.cos() * d, a.1 + ang.sin() * d)
    } else {
        let m = dx.abs().max(dy.abs());
        (
            a.0 + m * if dx < 0.0 { -1.0 } else { 1.0 },
            a.1 + m * if dy < 0.0 { -1.0 } else { 1.0 },
        )
    }
}

/// Pinta de `bg` todo lo que quede fuera del lazo, dentro del recorte.
fn mask_outside(px: &mut [Color32], r: Rect, poly: &[Pt], bg: Color32) {
    for row in 0..r.h {
        for col in 0..r.w {
            let p = ((r.x + col) as f32 + 0.5, (r.y + row) as f32 + 0.5);
            if !point_in_poly(p, poly) {
                px[row * r.w + col] = bg;
            }
        }
    }
}

fn point_in_poly(p: Pt, poly: &[Pt]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return true;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (xi, yi) = poly[i];
        let (xj, yj) = poly[j];
        if (yi > p.1) != (yj > p.1) && p.0 < (xj - xi) * (p.1 - yi) / (yj - yi) + xi {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dibujar(d: &mut Doc, a: Pt, b: Pt) {
        d.down(a, false, 3.0);
        d.drag_to(b, false);
        d.up(b);
    }

    /// El ciclo completo de una herramienta tiene que dejar exactamente un paso
    /// en el historial y ser reversible. Si `begin_stroke`/`end_stroke` quedan
    /// desparejos, el deshacer se rompe en silencio.
    #[test]
    fn cada_trazo_deja_un_paso_reversible() {
        let mut d = Doc::new(60, 60);
        let limpio = d.canvas.pixels().to_vec();

        d.tool = Tool::Pencil;
        d.width = 1.0;
        dibujar(&mut d, (10.0, 10.0), (40.0, 40.0));
        assert_eq!(d.canvas.undo_len(), 1, "el lápiz no dejó exactamente un paso");
        assert_ne!(d.canvas.pixels(), &limpio[..], "el lápiz no pintó nada");

        d.canvas.undo();
        assert_eq!(d.canvas.pixels(), &limpio[..], "deshacer el lápiz no fue exacto");
    }

    /// El borrador con clic derecho sólo debe tocar el Color 1.
    #[test]
    fn el_borrador_selectivo_respeta_los_otros_colores() {
        let mut d = Doc::new(40, 40);
        d.color1 = Color32::BLACK;
        d.color2 = Color32::WHITE;

        // Dos marcas: una negra (objetivo) y una azul (debe sobrevivir).
        d.canvas.set(20, 20, Color32::BLACK);
        d.canvas.set(21, 20, Color32::BLUE);

        d.tool = Tool::Eraser;
        d.width = 8.0;
        d.down((20.0, 20.0), true, 3.0);
        d.up((20.0, 20.0));

        assert_eq!(d.canvas.geti(20, 20).unwrap(), Color32::WHITE, "no borró el color objetivo");
        assert_eq!(d.canvas.geti(21, 20).unwrap(), Color32::BLUE, "se comió un color que no era");
    }

    /// Mover una selección deja el Color 2 atrás, no blanco ni transparente.
    #[test]
    fn mover_la_seleccion_deja_el_color_dos() {
        let mut d = Doc::new(50, 50);
        d.color2 = Color32::from_rgb(255, 0, 255);
        d.canvas.set(12, 12, Color32::BLACK);

        d.tool = Tool::Select;
        d.down((10.0, 10.0), false, 3.0);
        d.drag_to((20.0, 20.0), false);
        d.up((20.0, 20.0));
        assert!(d.sel.is_some(), "no quedó ninguna selección");

        // Agarrar adentro y arrastrar.
        d.down((15.0, 15.0), false, 3.0);
        d.drag_to((35.0, 35.0), false);
        d.up((35.0, 35.0));

        assert_eq!(
            d.canvas.geti(12, 12).unwrap(),
            d.color2,
            "el hueco no quedó del Color 2"
        );

        let pasos_antes = d.canvas.undo_len();
        d.commit_selection();
        assert!(d.sel.is_none(), "la selección no se confirmó");

        // Mover es UNA cosa para el usuario, así que tiene que ser UN paso.
        // Con dos, el primer Ctrl+Z dejaba el lienzo con el hueco vacío: un
        // estado intermedio que nadie vio nunca.
        assert_eq!(
            d.canvas.undo_len(),
            pasos_antes + 1,
            "mover la selección dejó más de un paso en el historial"
        );
    }

    /// Cambiar de herramienta a mitad de una curva dejaba el estado armado: el
    /// primer clic del lápiz lo consumía la curva, y el `begin_stroke` abierto
    /// se comía el trazo siguiente.
    #[test]
    fn cambiar_de_herramienta_cierra_la_curva_a_medias() {
        let mut d = Doc::new(60, 60);
        d.tool = Tool::Shape;
        d.shape = Shape::Curve;
        d.down((5.0, 5.0), false, 3.0);
        d.drag_to((40.0, 40.0), false);
        d.up((40.0, 40.0));
        assert!(d.is_multistep_active(), "la curva no quedó armada");

        d.set_tool(Tool::Pencil);
        assert!(!d.is_multistep_active(), "la curva sobrevivió al cambio de herramienta");

        // Y ahora el lápiz pinta de verdad, sin que le roben el clic.
        let antes = d.canvas.pixels().to_vec();
        d.down((10.0, 50.0), false, 3.0);
        d.drag_to((30.0, 50.0), false);
        d.up((30.0, 50.0));
        assert_ne!(d.canvas.pixels(), &antes[..], "el lápiz no pintó tras el cambio");
    }

    /// Pegar tiene que reusar la maquinaria de selección flotante.
    #[test]
    fn pegar_crea_una_seleccion_flotante() {
        let mut d = Doc::new(50, 50);
        let px = vec![Color32::RED; 4 * 4];
        d.paste(4, 4, px);

        let sel = d.sel.as_ref().expect("pegar no creó selección");
        assert_eq!(sel.r.x, 0);
        assert_eq!(sel.r.y, 0);
        assert!(sel.px.is_some(), "lo pegado no quedó flotando");
        assert_eq!(d.tool, Tool::Select, "pegar debería activar Seleccionar");

        // Al confirmar, los píxeles bajan al lienzo.
        d.commit_selection();
        assert_eq!(d.canvas.geti(1, 1).unwrap(), Color32::RED, "lo pegado no bajó al lienzo");
    }

    /// Copiar y pegar dentro de la app, sin tocar el sistema operativo.
    #[test]
    fn copiar_y_pegar_interno() {
        let mut d = Doc::new(40, 40);
        d.canvas.set(5, 5, Color32::BLUE);

        d.tool = Tool::Select;
        d.down((4.0, 4.0), false, 3.0);
        d.drag_to((10.0, 10.0), false);
        d.up((10.0, 10.0));
        d.copy_selection();
        assert!(d.clipboard.is_some(), "no copió nada");

        d.sel = None;
        d.paste_from_clipboard();
        d.commit_selection();
        assert_eq!(d.canvas.geti(1, 1).unwrap(), Color32::BLUE, "no pegó en el origen");
    }

    /// Estirar por una manija remuestrea, y remuestrea **desde el original**.
    ///
    /// Lo segundo es lo que se rompe solo: si cada tirón reescalara sobre lo ya
    /// reescalado, agrandar y volver al tamaño de antes dejaría un borrón en
    /// vez de la imagen de partida. Mirando sólo la pantalla no se nota hasta
    /// que ya perdiste el dibujo.
    #[test]
    fn estirar_la_seleccion_y_volver_deja_los_mismos_pixeles() {
        let mut d = Doc::new(20, 20);
        d.canvas.set(3, 3, Color32::RED);

        d.tool = Tool::Select;
        d.down((2.0, 2.0), false, 1.0);
        d.drag_to((6.0, 6.0), false);
        d.up((6.0, 6.0));

        // La manija 4 es la esquina de abajo a la derecha.
        let esquina = d.sel.as_ref().unwrap().handle(4);
        d.down(esquina, false, 1.0);
        let (w0, h0) = {
            let r = d.sel.as_ref().unwrap().r;
            (r.w, r.h)
        };
        let antes = d.sel.as_ref().unwrap().pixels().expect("levantar dejó píxeles");

        d.drag_to((esquina.0 + w0 as f32, esquina.1 + h0 as f32), false);
        let estirada = d.sel.as_ref().unwrap().pixels().unwrap();
        assert_eq!(
            estirada.len(),
            w0 * 2 * h0 * 2,
            "al doble de ancho y de alto tiene que haber cuatro veces los píxeles"
        );

        d.drag_to(esquina, false);
        d.up(esquina);
        assert_eq!(
            d.sel.as_ref().unwrap().pixels().unwrap(),
            antes,
            "volver al tamaño de antes tiene que dar la imagen de antes"
        );
    }
}
