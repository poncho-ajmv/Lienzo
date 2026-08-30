#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Lienzo — un clon de Paint de Windows 10, en Rust.
//!
//! Este archivo es la capa de aplicación: ventana, textura, teclado, archivos y
//! portapapeles. La lógica de verdad vive debajo y no sabe que egui existe:
//! `canvas` (píxeles e historial), `shapes` (formas y pinceles) y `doc`
//! (herramientas). Esos tres se testean con `cargo test`, sin ventana.

mod canvas;
mod doc;
mod lang;
mod shapes;
mod text;
mod theme;
mod ui;

use doc::{Doc, Tool};
use ecolor::Color32;
use egui::{vec2, Pos2, Sense};
use text::TextBox;
use theme::Theme;
use ui::{Cmd, Ico, Icon, Tab, UiIn};

/// Los once niveles de zoom de Paint: duplica por debajo de 100%, suma 100 por
/// encima. Verificado contra el original — la serie de sólo duplicar es de la
/// versión de Windows 11.
const ZOOMS: [f32; 11] = [0.125, 0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];

fn zoom_step(index: usize, direction: f32) -> usize {
    if direction > 0.0 {
        (index + 1).min(ZOOMS.len() - 1)
    } else if direction < 0.0 {
        index.saturating_sub(1)
    } else {
        index
    }
}

/// Los tamaños de lienzo que se ofrecen al crear un dibujo. Sin esto hay que
/// crear, ir a Propiedades y escribir dos números para cada formato común.
const PRESETS: [(&str, usize, usize); 5] = [
    ("Full HD", 1920, 1080),
    ("Cuadrado", 1080, 1080),
    ("Historia", 1080, 1920),
    ("A4 a 150 ppp", 1240, 1754),
    ("HD", 1280, 720),
];

/// Qué muestra el panel derecho del menú Archivo. Cambia según el comando que
/// tengas encima: Paint desperdicia esa mitad en una lista fija de recientes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
enum Pane {
    #[default]
    Recent,
    New,
    Export,
    Settings,
}

#[derive(Clone, Debug)]
enum PendingAction {
    New(usize, usize),
    Open(std::path::PathBuf),
    Exit,
}

/// Lo que sobrevive a cerrar el programa.
///
/// El tema se guarda **por nombre y no por índice**: el índice se corre en
/// cuanto se agrega un tema, y quien vuelve a abrir se encuentra con otro.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Ajustes {
    tema: String,
    idioma: String,
    color1: [u8; 3],
    color2: [u8; 3],
    personalizados: Vec<Option<[u8; 3]>>,
}

/// La clave del almacén de eframe.
const AJUSTES: &str = "lienzo";

/// Los 48 colores básicos del diálogo de color de Windows, en su orden.
const BASIC_COLORS: [[u8; 3]; 48] = [
    [0xff, 0x80, 0x80],
    [0xff, 0xff, 0x80],
    [0x80, 0xff, 0x80],
    [0x00, 0xff, 0x80],
    [0x80, 0xff, 0xff],
    [0x00, 0x80, 0xff],
    [0xff, 0x80, 0xc0],
    [0xff, 0x80, 0xff],
    [0xff, 0x00, 0x00],
    [0xff, 0xff, 0x00],
    [0x80, 0xff, 0x00],
    [0x00, 0xff, 0x40],
    [0x00, 0xff, 0xff],
    [0x00, 0x80, 0xc0],
    [0x80, 0x80, 0xc0],
    [0xff, 0x00, 0xff],
    [0x80, 0x40, 0x40],
    [0xff, 0x80, 0x40],
    [0x00, 0xff, 0x00],
    [0x00, 0x80, 0x80],
    [0x00, 0x40, 0x80],
    [0x80, 0x80, 0xff],
    [0x80, 0x00, 0x40],
    [0xff, 0x00, 0x80],
    [0x80, 0x00, 0x00],
    [0xff, 0x80, 0x00],
    [0x00, 0x80, 0x00],
    [0x00, 0x80, 0x40],
    [0x00, 0x00, 0xff],
    [0x00, 0x00, 0xa0],
    [0x80, 0x00, 0x80],
    [0x80, 0x00, 0xff],
    [0x40, 0x00, 0x00],
    [0x80, 0x40, 0x00],
    [0x00, 0x40, 0x00],
    [0x00, 0x40, 0x40],
    [0x00, 0x00, 0x80],
    [0x00, 0x00, 0x40],
    [0x40, 0x00, 0x40],
    [0x40, 0x00, 0x80],
    [0x00, 0x00, 0x00],
    [0x80, 0x80, 0x00],
    [0x80, 0x80, 0x40],
    [0x80, 0x80, 0x80],
    [0x40, 0x80, 0x80],
    [0xc0, 0xc0, 0xc0],
    [0x40, 0x00, 0x40],
    [0xff, 0xff, 0xff],
];

/// El lienzo es opaco: al importar RGBA se compone sobre blanco, no se revelan
/// los canales RGB invisibles que pueda guardar un PNG transparente.
fn opaque_rgba(p: &[u8]) -> Color32 {
    let a = p[3] as u16;
    let mix = |c: u8| ((c as u16 * a + 255 * (255 - a) + 127) / 255) as u8;
    Color32::from_rgb(mix(p[0]), mix(p[1]), mix(p[2]))
}

/// Una muestra de color. `None` es un hueco libre de los personalizados: va
/// con borde punteado, porque con borde sólido parece un botón blanco.
fn swatch(ui: &mut egui::Ui, theme: &Theme, c: Option<Color32>, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(vec2(24.0, 19.0), Sense::click());
    match c {
        Some(c) => {
            ui.painter().rect_filled(rect, 0.0, c);
            ui.painter().rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                egui::StrokeKind::Inside,
            );
        }
        None => {
            ui.painter().rect_filled(rect, 0.0, Color32::WHITE);
            // Punteado a mano: `rect_stroke` no tiene borde discontinuo.
            let d = egui::Stroke::new(1.0, Color32::from(theme.border_strong));
            let mut x = rect.left();
            while x < rect.right() {
                let x2 = (x + 2.0).min(rect.right());
                ui.painter()
                    .line_segment([Pos2::new(x, rect.top()), Pos2::new(x2, rect.top())], d);
                ui.painter().line_segment(
                    [Pos2::new(x, rect.bottom()), Pos2::new(x2, rect.bottom())],
                    d,
                );
                x += 4.0;
            }
            let mut y = rect.top();
            while y < rect.bottom() {
                let y2 = (y + 2.0).min(rect.bottom());
                ui.painter()
                    .line_segment([Pos2::new(rect.left(), y), Pos2::new(rect.left(), y2)], d);
                ui.painter()
                    .line_segment([Pos2::new(rect.right(), y), Pos2::new(rect.right(), y2)], d);
                y += 4.0;
            }
        }
    }
    if selected {
        ui.painter().rect_stroke(
            rect.expand(2.0),
            0.0,
            egui::Stroke::new(2.0, Color32::from(theme.accent)),
            egui::StrokeKind::Outside,
        );
    } else if resp.hovered() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            0.0,
            egui::Stroke::new(1.0, Color32::from(theme.text_dim)),
            egui::StrokeKind::Outside,
        );
    }
    resp.clicked()
}

/// Etiqueta de sección: chica y atenuada, para que no compita con el contenido.
fn caption(ui: &mut egui::Ui, theme: &Theme, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .size(theme.font_size - 1.0)
            .color(Color32::from(theme.text_dim)),
    );
    ui.add_space(4.0);
}

/// El lienzo con el que abre Paint de Windows 10 en una pantalla de 1080p.
const DEFAULT_W: usize = 1152;
const DEFAULT_H: usize = 648;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([640.0, 480.0])
            // Explícitos: la franja negra de arriba era la zona de la barra de
            // título de macOS sin pintar. `clear_color` no llega ahí porque no
            // es área de egui.
            .with_decorations(true)
            .with_transparent(false)
            .with_titlebar_shown(true)
            .with_title_shown(true)
            .with_fullsize_content_view(false)
            .with_title("Sin título - Lienzo"),
        ..Default::default()
    };
    eframe::run_native("Lienzo", options, Box::new(|cc| Ok(Box::new(App::new(cc)))))
}

#[derive(Default)]
struct Dialogs {
    resize: bool,
    properties: bool,
    color: bool,
    about: bool,
    file_menu: bool,
    qat_menu: bool,
    paste_menu: bool,
    outline_menu: bool,
    fill_menu: bool,
    rotate_menu: bool,
    size_menu: bool,
    theme_menu: bool,
    brushes: bool,
    select_menu: bool,
    /// Campos del diálogo Cambiar tamaño y sesgar.
    by_percent: bool,
    keep_ratio: bool,
    rw: f32,
    rh: f32,
    skew_x: f32,
    skew_y: f32,
    /// Ancho y alto del diálogo Propiedades. Tienen que persistir entre frames:
    /// `DragValue` no guarda estado propio, así que re-leerlos del lienzo en
    /// cada frame descartaba lo que el usuario acababa de escribir.
    pw: f32,
    ph: f32,
    /// Campos del editor de color: matiz, saturación y valor en 0..1.
    hsv: [f32; 3],
}

impl Dialogs {
    /// Los catorce emergentes en un solo lugar. Antes la lista estaba escrita a
    /// mano en el guardia de atajos, así que un menú nuevo nacía sin Escape y
    /// sin bloquear los atajos hasta que alguien se acordaba de agregarlo.
    fn popups(&mut self) -> [&mut bool; 14] {
        [
            &mut self.resize,
            &mut self.properties,
            &mut self.color,
            &mut self.about,
            &mut self.file_menu,
            &mut self.qat_menu,
            &mut self.paste_menu,
            &mut self.outline_menu,
            &mut self.fill_menu,
            &mut self.rotate_menu,
            &mut self.size_menu,
            &mut self.theme_menu,
            &mut self.brushes,
            &mut self.select_menu,
        ]
    }

    fn any_open(&mut self) -> bool {
        self.popups().iter().any(|f| **f)
    }

    /// Cierra todo lo abierto. Devuelve si había algo que cerrar.
    fn close_all(&mut self) -> bool {
        let mut any = false;
        for f in self.popups() {
            any |= *f;
            *f = false;
        }
        any
    }
}

struct App {
    doc: Doc,
    themes: Vec<Theme>,
    theme_idx: usize,
    /// La textura del lienzo. Se crea una vez y después sólo se actualizan los
    /// rectángulos sucios: una subida completa asigna una textura de GPU nueva
    /// en cada llamada, que a 60 fps es medio giga por segundo de basura.
    tex: Option<egui::TextureHandle>,
    /// Textura aparte para la selección flotante, que ya no está en el lienzo.
    sel_tex: Option<egui::TextureHandle>,
    sel_dirty: bool,
    /// Si el trazo actual empezó sobre el lienzo.
    drawing: bool,
    zoom_idx: usize,
    path: Option<std::path::PathBuf>,
    dialogs: Dialogs,
    show_grid: bool,
    show_rulers: bool,
    show_status: bool,
    show_thumbnail: bool,
    tab: Tab,
    fullscreen: bool,
    /// Qué botones muestra la barra de acceso rápido, en el orden de `ALL_QAT`.
    qat: [bool; ui::ALL_QAT.len()],
    qat_below: bool,
    ribbon_min: bool,
    /// Dónde quedó el bitmap en pantalla este frame. El cuadro de texto se
    /// posiciona contra esto y no contra el panel: el lienzo vive dentro de un
    /// área con scroll, así que el panel no dice dónde está de verdad.
    canvas_rect: egui::Rect,
    /// Dónde colgar el menú que se acaba de abrir.
    menu_anchor: Pos2,
    /// Las nueve muestras de pincel, pintadas una vez con el motor de verdad.
    brush_tex: Option<egui::TextureHandle>,
    /// Las siete muestras de contorno. Se rehacen si cambia el Color 1, porque
    /// el original las dibuja con el color elegido.
    stroke_tex: Option<egui::TextureHandle>,
    stroke_tex_color: Color32,
    /// El campo de matiz por saturación del editor de color. Nunca cambia,
    /// así que se genera una vez.
    hs_tex: Option<egui::TextureHandle>,
    /// Los últimos archivos abiertos o guardados, el más nuevo primero.
    recent: Vec<std::path::PathBuf>,
    /// Qué muestra el panel derecho del menú Archivo.
    pane: Pane,
    /// Escala del exportado, en porcentaje.
    export_scale: u32,
    /// Los dieciséis huecos de colores personalizados del diálogo.
    custom_colors: [Option<Color32>; 16],
    /// Índice del idioma en `lang::LANGS`. La traducción vive en un global,
    /// pero el que se guarda en los ajustes es este.
    lang_idx: usize,
    /// Tamaño en curso mientras se arrastra un tirador del lienzo.
    resizing: Option<(usize, usize)>,
    /// Cuadro de texto flotante de la herramienta Texto.
    /// El cuadro de texto abierto, si hay uno. Mientras vive no tocó un solo
    /// píxel: recién al confirmar se vuelca al lienzo.
    text_box: Option<TextBox>,
    /// Qué se está arrastrando del cuadro, desde dónde, y cómo estaba.
    t_grab: Option<TGrab>,
    t_from: (f32, f32),
    t_orig: (f32, f32, f32, f32),
    /// Cuál de los dos colores recibe lo que se elija en la paleta.
    picking_c1: bool,
    pending_action: Option<PendingAction>,
    allow_close: bool,
    status: String,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let themes = theme::load_all();
        let mut app = Self {
            doc: Doc::new(DEFAULT_W, DEFAULT_H),
            themes,
            theme_idx: 0,
            tex: None,
            sel_tex: None,
            sel_dirty: false,
            drawing: false,
            zoom_idx: 3, // 100%
            path: None,
            dialogs: Dialogs {
                keep_ratio: true,
                by_percent: true,
                rw: 100.0,
                rh: 100.0,
                ..Default::default()
            },
            show_grid: false,
            show_rulers: false,
            show_status: true,
            show_thumbnail: false,
            tab: Tab::Home,
            fullscreen: false,
            // Los mismos tres que trae Paint de fábrica: guardar, deshacer, rehacer.
            qat: [false, false, true, false, false, true, true],
            qat_below: false,
            ribbon_min: false,
            canvas_rect: egui::Rect::ZERO,
            menu_anchor: Pos2::ZERO,
            brush_tex: None,
            stroke_tex: None,
            stroke_tex_color: Color32::BLACK,
            hs_tex: None,
            recent: Vec::new(),
            pane: Pane::Recent,
            export_scale: 100,
            custom_colors: [None; 16],
            lang_idx: 0,
            resizing: None,
            text_box: None,
            t_grab: None,
            t_from: (0.0, 0.0),
            t_orig: (0.0, 0.0, 0.0, 0.0),
            picking_c1: true,
            pending_action: None,
            allow_close: false,
            status: String::new(),
        };
        // Lo guardado pisa los valores de fábrica, antes de instalar el estilo.
        if let Some(st) = cc.storage {
            if let Some(a) = eframe::get_value::<Ajustes>(st, AJUSTES) {
                if let Some(i) = app.themes.iter().position(|t| t.name == a.tema) {
                    app.theme_idx = i;
                }
                if !a.idioma.is_empty() {
                    app.lang_idx = lang::index_of(&a.idioma);
                    lang::set(app.lang_idx);
                }
                let c = |v: [u8; 3]| Color32::from_rgb(v[0], v[1], v[2]);
                app.doc.color1 = c(a.color1);
                app.doc.color2 = c(a.color2);
                for (slot, saved) in app.custom_colors.iter_mut().zip(a.personalizados) {
                    *slot = saved.map(c);
                }
            }
        }

        app.install_style(&cc.egui_ctx);
        // egui se queda con Ctrl + y Ctrl − para escalar *toda* la interfaz,
        // como un navegador. En un editor de imágenes esas teclas son del
        // lienzo, así que le sacamos el atajo.
        cc.egui_ctx.options_mut(|o| o.zoom_with_keyboard = false);
        app
    }

    /// egui no tiene `Context::set_style`: guarda un estilo por modo claro y
    /// otro por oscuro. Nuestro tema ya decide cuál es, así que se instala en
    /// los dos slots para que el conmutador del sistema no lo pise.
    fn install_style(&self, ctx: &egui::Context) {
        let style = self.theme().to_style();
        ctx.all_styles_mut(|s| *s = style.clone());
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_idx.min(self.themes.len() - 1)]
    }

    fn zoom(&self) -> f32 {
        ZOOMS[self.zoom_idx]
    }

    fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Sin título".into());
        let star = if self.doc.canvas.dirty_file { "*" } else { "" };
        format!("{star}{name} - Lienzo")
    }

    // -------------------------------------------------------------- textura

    fn sync_texture(&mut self, ctx: &egui::Context) {
        let (w, h) = (self.doc.canvas.w, self.doc.canvas.h);
        let stale = self.tex.as_ref().is_none_or(|t| t.size() != [w, h]);

        if stale {
            let img = egui::ColorImage::new([w, h], self.doc.canvas.pixels().to_vec());
            self.tex = Some(ctx.load_texture("lienzo", img, egui::TextureOptions::NEAREST));
            self.doc.canvas.take_upload();
            return;
        }

        // Sólo la zona sucia. Es el mismo rectángulo que alimenta el historial:
        // una sola cuenta sirve para las dos cosas.
        if let Some(r) = self.doc.canvas.take_upload() {
            let sub = self.doc.canvas.region(r);
            let img = egui::ColorImage::new([r.w, r.h], sub);
            if let Some(t) = self.tex.as_mut() {
                t.set_partial([r.x, r.y], img, egui::TextureOptions::NEAREST);
            }
        }
    }

    fn sync_sel_texture(&mut self, ctx: &egui::Context) {
        let Some(sel) = &self.doc.sel else {
            self.sel_tex = None;
            return;
        };
        let Some(px) = sel.pixels() else {
            self.sel_tex = None;
            return;
        };
        let size = [sel.r.w, sel.r.h];
        let stale = self.sel_dirty || self.sel_tex.as_ref().is_none_or(|t| t.size() != size);
        // Estirar cambia `r` sin tocar `px`; el tamaño distinto es justo lo que
        // hace que se rehaga la textura, así que no hace falta nada más.
        if stale {
            let img = egui::ColorImage::new(size, px);
            self.sel_tex = Some(ctx.load_texture("seleccion", img, egui::TextureOptions::NEAREST));
            self.sel_dirty = false;
        }
    }

    /// Pinta las nueve muestras con el motor de pinceles de verdad y las sube
    /// como una sola textura en tira. Es la única forma honesta de previsualizar
    /// un pincel: se ve exactamente lo que va a hacer.
    fn build_brush_previews(&mut self, ctx: &egui::Context) {
        if self.brush_tex.is_some() {
            return;
        }
        const T: usize = 34;
        let n = shapes::ALL_BRUSHES.len();
        let mut c = canvas::Canvas::new(T, T * n);
        let mut rng = shapes::Rng::new();
        for (i, b) in shapes::ALL_BRUSHES.iter().enumerate() {
            let y = (i * T) as f32;
            // Una curva corta, como la muestra del original.
            let pts = [
                (5.0, y + 24.0),
                (12.0, y + 11.0),
                (21.0, y + 22.0),
                (29.0, y + 9.0),
            ];
            for w in pts.windows(2) {
                shapes::brush_stroke(&mut c, *b, w[0], w[1], 5.0, Color32::BLACK, &mut rng);
            }
        }
        let img = egui::ColorImage::new([T, T * n], c.pixels().to_vec());
        self.brush_tex = Some(ctx.load_texture("pinceles", img, egui::TextureOptions::LINEAR));
    }

    /// Las siete muestras de contorno, con el mismo motor que dibuja en el
    /// lienzo. `Sin contorno` queda en blanco: la diagonal roja la pinta la fila.
    fn build_stroke_previews(&mut self, ctx: &egui::Context) {
        let col = self.doc.color1;
        if self.stroke_tex.is_some() && self.stroke_tex_color == col {
            return;
        }
        const W: usize = 40;
        const H: usize = 26;
        let n = doc::ALL_STROKES.len();
        let mut c = canvas::Canvas::new(W, H * n);
        let mut rng = shapes::Rng::new();

        for (i, st) in doc::ALL_STROKES.iter().enumerate() {
            let y = (i * H) as f32;
            match st.brush() {
                Some(b) => {
                    let pts = [
                        (6.0, y + 19.0),
                        (16.0, y + 8.0),
                        (26.0, y + 18.0),
                        (34.0, y + 7.0),
                    ];
                    for w in pts.windows(2) {
                        shapes::brush_stroke(&mut c, b, w[0], w[1], 6.0, col, &mut rng);
                    }
                }
                // Color sólido: un cuadrado lleno, como en el original.
                None if *st == doc::Stroke::Solid => {
                    for yy in (y as i32 + 5)..(y as i32 + 21) {
                        for xx in 12..28 {
                            c.set(xx, yy, col);
                        }
                    }
                }
                None => {}
            }
        }
        let img = egui::ColorImage::new([W, H * n], c.pixels().to_vec());
        self.stroke_tex = Some(ctx.load_texture("contornos", img, egui::TextureOptions::LINEAR));
        self.stroke_tex_color = col;
    }

    /// El campo de matiz (eje X) por saturación (eje Y) del editor de color.
    /// Se genera una sola vez: no depende de nada que cambie.
    fn build_hs_texture(&mut self, ctx: &egui::Context) {
        if self.hs_tex.is_some() {
            return;
        }
        const W: usize = 238;
        const H: usize = 216;
        let mut px = Vec::with_capacity(W * H);
        for y in 0..H {
            let sat = 1.0 - y as f32 / (H - 1) as f32;
            for x in 0..W {
                let hue = x as f32 / (W - 1) as f32;
                let c = ecolor::Hsva::new(hue, sat, 1.0, 1.0).to_srgb();
                px.push(Color32::from_rgb(c[0], c[1], c[2]));
            }
        }
        let img = egui::ColorImage::new([W, H], px);
        self.hs_tex = Some(ctx.load_texture("campo_color", img, egui::TextureOptions::LINEAR));
    }

    // ------------------------------------------------------------- comandos

    fn has_unsaved_changes(&self) -> bool {
        self.doc.canvas.dirty_file || self.text_box.as_ref().is_some_and(|t| !t.s.is_empty())
    }

    fn reset_document_ui(&mut self) {
        self.text_box = None;
        self.t_grab = None;
        self.resizing = None;
        self.drawing = false;
        self.sel_tex = None;
        self.sel_dirty = false;
        self.tex = None;
        self.dialogs.close_all();
    }

    fn request_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        if self.has_unsaved_changes() {
            self.dialogs.close_all();
            self.pending_action = Some(action);
        } else {
            self.perform_action(action, ctx);
        }
    }

    fn perform_action(&mut self, action: PendingAction, ctx: &egui::Context) {
        match action {
            PendingAction::New(w, h) => {
                self.doc = Doc::new(w.max(1), h.max(1));
                self.path = None;
                self.reset_document_ui();
            }
            PendingAction::Open(path) => self.open_path(&path),
            PendingAction::Exit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn save_current(&mut self, ctx: &egui::Context) -> bool {
        match self.path.clone() {
            Some(path) => {
                if self.text_box.is_some() {
                    self.commit_text(ctx);
                }
                self.save_to(&path)
            }
            None => self.save_as(ctx),
        }
    }

    fn apply(&mut self, cmd: Cmd, ctx: &egui::Context) {
        match cmd {
            Cmd::New => self.request_action(PendingAction::New(DEFAULT_W, DEFAULT_H), ctx),
            Cmd::NewSized(w, h) => self.request_action(PendingAction::New(w, h), ctx),
            Cmd::OpenRecent(i) => {
                if let Some(p) = self.recent.get(i).cloned() {
                    self.request_action(PendingAction::Open(p), ctx);
                }
            }
            Cmd::Export(scale) => self.export(scale),
            Cmd::CopyImage => {
                let (w, h) = (self.doc.canvas.w, self.doc.canvas.h);
                self.doc.commit_selection();
                self.doc.clipboard = Some((w, h, self.doc.canvas.pixels().to_vec()));
                self.copy_to_system();
            }
            Cmd::Reveal => self.reveal(),
            Cmd::Open => self.open_file(ctx),
            Cmd::Save => {
                self.save_current(ctx);
            }
            Cmd::SaveAs => {
                self.save_as(ctx);
            }
            Cmd::Undo => {
                self.doc.commit_selection();
                self.doc.canvas.undo();
            }
            Cmd::Redo => self.doc.canvas.redo(),
            Cmd::Cut | Cmd::Copy => {
                if self.doc.sel.is_none() {
                    self.status = "No hay nada seleccionado".into();
                } else {
                    if cmd == Cmd::Cut {
                        self.doc.cut_selection();
                    } else {
                        self.doc.copy_selection();
                    }
                    self.copy_to_system();
                }
            }
            Cmd::Paste => self.paste_from_system(),
            Cmd::PasteFrom => self.paste_from_file(),
            Cmd::SelectAll => self.doc.select_all(),
            Cmd::Delete => self.doc.delete_selection(),
            Cmd::Crop => self.doc.crop_to_selection(),
            Cmd::ResizeDialog => {
                self.dialogs.resize = true;
                self.dialogs.rw = 100.0;
                self.dialogs.rh = 100.0;
                self.dialogs.skew_x = 0.0;
                self.dialogs.skew_y = 0.0;
            }
            Cmd::PropertiesDialog => {
                self.dialogs.properties = true;
                self.dialogs.pw = self.doc.canvas.w as f32;
                self.dialogs.ph = self.doc.canvas.h as f32;
            }
            Cmd::Rotate(q) => {
                self.doc.commit_selection();
                self.doc.canvas.rotate(q);
            }
            Cmd::FlipH => {
                self.doc.commit_selection();
                self.doc.canvas.flip_horizontal();
            }
            Cmd::FlipV => {
                self.doc.commit_selection();
                self.doc.canvas.flip_vertical();
            }
            Cmd::InvertColors => self.doc.invert_selection_colors(),
            Cmd::ZoomIn => self.zoom_idx = zoom_step(self.zoom_idx, 1.0),
            Cmd::ZoomOut => self.zoom_idx = zoom_step(self.zoom_idx, -1.0),
            Cmd::Zoom100 => self.zoom_idx = 3,
            Cmd::ZoomTo(i) => self.zoom_idx = i.min(ZOOMS.len() - 1),
            Cmd::SetTheme(i) => {
                if i < self.themes.len() {
                    self.theme_idx = i;
                    self.install_style(ctx);
                }
            }
            Cmd::ToggleGrid => self.show_grid = !self.show_grid,
            Cmd::ToggleRulers => self.show_rulers = !self.show_rulers,
            Cmd::ToggleStatusBar => self.show_status = !self.show_status,
            Cmd::ToggleThumbnail => self.show_thumbnail = !self.show_thumbnail,
            Cmd::ToggleQatBelow => self.qat_below = !self.qat_below,
            Cmd::ToggleRibbonMin => self.ribbon_min = !self.ribbon_min,
            // `ponytail:` sin impresión todavía. El plan es componer un PDF de
            // una página con `printpdf` y entregárselo al sistema.
            Cmd::Print | Cmd::PrintPreview => {
                self.status = "Imprimir todavía no está implementado".into();
            }
            Cmd::FullScreen => {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen));
            }
            Cmd::About => self.dialogs.about = true,
            Cmd::Exit => self.request_action(PendingAction::Exit, ctx),
        }
    }

    // -------------------------------------------------------------- archivo

    /// Recuerda un archivo en la lista de recientes: el más nuevo primero, sin
    /// repetidos y con tope de ocho.
    fn remember(&mut self, path: &std::path::Path) {
        self.recent.retain(|p| p != path);
        self.recent.insert(0, path.to_path_buf());
        self.recent.truncate(8);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_file(&mut self, ctx: &egui::Context) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Imágenes",
                &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "ico"],
            )
            .pick_file()
        else {
            return;
        };
        self.request_action(PendingAction::Open(path), ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_path(&mut self, path: &std::path::Path) {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let px = rgba.pixels().map(|p| opaque_rgba(&p.0)).collect();
                self.doc.load(w, h, px);
                self.reset_document_ui();
                self.path = Some(path.to_path_buf());
                self.remember(path);
                self.status = "Abierto".into();
            }
            Err(e) => self.status = format!("No pude abrir: {e}"),
        }
    }

    /// Guarda una copia a otra escala. `Guardar como` sólo cambia el formato;
    /// esto saca el mismo dibujo al doble o a la mitad sin tocar el original.
    #[cfg(not(target_arch = "wasm32"))]
    fn export(&mut self, scale: u32) {
        let (w, h) = (
            (self.doc.canvas.w * scale as usize / 100).max(1),
            (self.doc.canvas.h * scale as usize / 100).max(1),
        );
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .set_file_name(format!("copia-{w}x{h}.png"))
            .save_file()
        else {
            return;
        };
        self.doc.commit_selection();
        let mut buf = image::RgbaImage::new(w as u32, h as u32);
        let (cw, chh) = (self.doc.canvas.w, self.doc.canvas.h);
        let px = self.doc.canvas.pixels();
        for y in 0..h {
            let sy = y * chh / h;
            for x in 0..w {
                let sx = x * cw / w;
                let p = px[sy * cw + sx];
                buf.put_pixel(x as u32, y as u32, image::Rgba([p.r(), p.g(), p.b(), 255]));
            }
        }
        match buf.save(&path) {
            Ok(()) => self.status = format!("Exportado a {w} × {h} px"),
            Err(e) => self.status = format!("No pude exportar: {e}"),
        }
    }

    /// Abre la carpeta del archivo y lo deja seleccionado.
    #[cfg(not(target_arch = "wasm32"))]
    fn reveal(&mut self) {
        let Some(path) = self.path.clone() else {
            self.status = "Guardá el dibujo primero".into();
            return;
        };
        let r = if cfg!(target_os = "macos") {
            std::process::Command::new("open")
                .arg("-R")
                .arg(&path)
                .spawn()
        } else if cfg!(target_os = "windows") {
            std::process::Command::new("explorer")
                .arg(format!("/select,{}", path.display()))
                .spawn()
        } else {
            let dir = path.parent().unwrap_or(&path).to_path_buf();
            std::process::Command::new("xdg-open").arg(dir).spawn()
        };
        if let Err(e) = r {
            self.status = format!("No pude abrir el explorador: {e}");
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn open_path(&mut self, _p: &std::path::Path) {}
    #[cfg(target_arch = "wasm32")]
    fn export(&mut self, _s: u32) {
        self.status = "Exportar en web todavía no está".into();
    }
    #[cfg(target_arch = "wasm32")]
    fn reveal(&mut self) {}

    /// "Pegar desde" de Paint: trae una imagen de disco y la deja flotando
    /// encima del dibujo, sin reemplazarlo — que es lo que la distingue de Abrir.
    #[cfg(not(target_arch = "wasm32"))]
    fn paste_from_file(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Imágenes",
                &["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "ico"],
            )
            .pick_file()
        else {
            return;
        };
        match image::open(&path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let px = rgba.pixels().map(|p| opaque_rgba(&p.0)).collect();
                self.doc.paste(w, h, px);
                self.sel_dirty = true;
                self.status = format!("Pegado desde {}", path.display());
            }
            Err(e) => self.status = format!("No pude abrir: {e}"),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn paste_from_file(&mut self) {
        self.status = "Pegar desde archivo en web todavía no está".into();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_as(&mut self, ctx: &egui::Context) -> bool {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("PNG", &["png"])
            .add_filter("JPEG", &["jpg", "jpeg"])
            .add_filter("Mapa de bits", &["bmp"])
            .add_filter("GIF", &["gif"])
            .add_filter("TIFF", &["tif", "tiff"])
            .set_file_name("dibujo.png")
            .save_file()
        else {
            return false;
        };
        if self.text_box.is_some() {
            self.commit_text(ctx);
        }
        if self.save_to(&path) {
            self.path = Some(path);
            true
        } else {
            false
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_to(&mut self, path: &std::path::Path) -> bool {
        self.doc.commit_selection();
        let (w, h) = (self.doc.canvas.w, self.doc.canvas.h);
        let mut buf = image::RgbaImage::new(w as u32, h as u32);
        for (i, p) in self.doc.canvas.pixels().iter().enumerate() {
            buf.put_pixel(
                (i % w) as u32,
                (i / w) as u32,
                image::Rgba([p.r(), p.g(), p.b(), 255]),
            );
        }
        match buf.save(path) {
            Ok(()) => {
                self.doc.canvas.dirty_file = false;
                self.remember(path);
                self.status = format!("Guardado en {}", path.display());
                true
            }
            Err(e) => {
                self.status = format!("No pude guardar: {e}");
                false
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn open_file(&mut self, _ctx: &egui::Context) {
        self.status = "Abrir archivos en web todavía no está".into();
    }
    #[cfg(target_arch = "wasm32")]
    fn save_as(&mut self, _ctx: &egui::Context) -> bool {
        self.status = "Guardar en web todavía no está".into();
        false
    }
    #[cfg(target_arch = "wasm32")]
    fn save_to(&mut self, _path: &std::path::Path) -> bool {
        false
    }

    // --------------------------------------------------------- portapapeles

    /// Copiar hacia afuera. egui tiene `OutputCommand::CopyImage`, pero acá
    /// vamos directo a arboard para que sea el mismo camino en las dos
    /// direcciones.
    #[cfg(not(target_arch = "wasm32"))]
    fn copy_to_system(&mut self) {
        let Some((w, h, px)) = self.doc.clipboard.clone() else {
            return;
        };
        let mut bytes = Vec::with_capacity(w * h * 4);
        for p in &px {
            bytes.extend_from_slice(&[p.r(), p.g(), p.b(), 255]);
        }
        let img = arboard::ImageData {
            width: w,
            height: h,
            bytes: std::borrow::Cow::Owned(bytes),
        };
        match arboard::Clipboard::new().and_then(|mut c| c.set_image(img)) {
            Ok(()) => self.status = "Copiado".into(),
            Err(e) => self.status = format!("No pude copiar: {e}"),
        }
    }

    /// Pegar desde afuera. egui **no tiene** evento de pegar imagen —el issue
    /// #2108 lleva abierto desde 2022—, así que se llama a arboard directo.
    /// Lo pegado se convierte en una selección flotante, que es la misma
    /// maquinaria de la herramienta Seleccionar: pegar no es una función nueva.
    #[cfg(not(target_arch = "wasm32"))]
    fn paste_from_system(&mut self) {
        match arboard::Clipboard::new().and_then(|mut c| c.get_image()) {
            Ok(img) => {
                let (w, h) = (img.width, img.height);
                // arboard entrega RGBA sin premultiplicar; el lienzo es opaco.
                let px: Vec<Color32> = img
                    .bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| opaque_rgba(c))
                    .collect();
                if px.len() == w * h {
                    self.doc.paste(w, h, px);
                    self.sel_dirty = true;
                    self.status = "Pegado".into();
                }
            }
            // Sin imagen en el portapapeles, se cae al portapapeles interno.
            Err(_) => self.doc.paste_from_clipboard(),
        }
        self.sel_dirty = true;
    }

    #[cfg(target_arch = "wasm32")]
    fn copy_to_system(&mut self) {}
    #[cfg(target_arch = "wasm32")]
    fn paste_from_system(&mut self) {
        self.doc.paste_from_clipboard();
        self.sel_dirty = true;
    }

    // -------------------------------------------------------------- teclado

    fn keyboard(&mut self, ctx: &egui::Context) -> Vec<Cmd> {
        let mut cmds = Vec::new();
        if self.pending_action.is_some() {
            if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                self.pending_action = None;
                self.allow_close = false;
            }
            return cmds;
        }
        // Si hay un cuadro de texto o un diálogo abierto, las teclas son suyos.
        // Antes esto miraba `memory().focused()`, que devuelve `Some` en cuanto
        // *cualquier* widget toma el foco —un botón recién clickeado incluido—
        // y dejaba todos los atajos muertos.
        // Escape sale de lo que sea que esté encima: diálogo, menú o cuadro de
        // texto. Va antes que todo y corta acá, para que la misma tecla no
        // cierre además el polígono a medio hacer que hay debajo.
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.text_box.take().is_some() {
                self.t_grab = None;
                self.tab = Tab::Home;
                return cmds;
            }
            if self.dialogs.close_all() {
                return cmds;
            }
        }
        // Ctrl+Enter confirma sin sacar la mano del teclado. `Enter` solo hace
        // un salto de línea, como en cualquier cuadro de varias líneas.
        if self.text_box.is_some()
            && ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Enter))
        {
            self.commit_text(ctx);
            return cmds;
        }
        if self.text_box.is_some() || self.dialogs.any_open() {
            return cmds;
        }
        // Ctrl en Windows/Linux y Cmd en macOS + rueda: el gesto habitual de
        // Paint. Sólo responde sobre el lienzo para no cambiar el zoom mientras
        // se recorre un menú o una galería. `zoom_delta` también acepta el gesto
        // de pellizcar del trackpad sin añadir otro camino de entrada.
        let wheel_zoom = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .filter(|p| self.canvas_rect.contains(*p))
                .map(|_| i.zoom_delta() - 1.0)
                .filter(|delta| delta.abs() > f32::EPSILON)
        });
        if let Some(direction) = wheel_zoom {
            cmds.push(if direction > 0.0 {
                Cmd::ZoomIn
            } else {
                Cmd::ZoomOut
            });
        }
        ctx.input(|i| {
            let m = i.modifiers;
            let cmd = m.command;
            use egui::Key as K;
            for (key, c) in [
                (K::N, Cmd::New),
                (K::O, Cmd::Open),
                (K::Z, Cmd::Undo),
                (K::Y, Cmd::Redo),
                (K::A, Cmd::SelectAll),
                (K::W, Cmd::ResizeDialog),
                (K::E, Cmd::PropertiesDialog),
            ] {
                if cmd && i.key_pressed(key) {
                    cmds.push(c);
                }
            }

            // Cortar, copiar y pegar **no llegan como tecla**: egui-winit los
            // intercepta antes y los convierte en eventos propios.
            for ev in &i.events {
                match ev {
                    egui::Event::Cut => cmds.push(Cmd::Cut),
                    egui::Event::Copy => cmds.push(Cmd::Copy),
                    _ => {}
                }
            }

            // Y pegar es peor: egui sólo emite `Event::Paste` cuando el
            // portapapeles tiene **texto**. Con una imagen no emite nada, y
            // además se come la tecla — por eso Ctrl+V estaba muerto.
            //
            // El resquicio: ese filtro corre sólo en la pulsación. Al *soltar*
            // la tecla el evento sí se emite, así que se escucha la soltada.
            if cmd && i.key_released(K::V) {
                cmds.push(Cmd::Paste);
            }
            if cmd && i.key_pressed(K::S) {
                cmds.push(if m.shift { Cmd::SaveAs } else { Cmd::Save });
            }
            if cmd && m.shift && i.key_pressed(K::I) {
                cmds.push(Cmd::InvertColors);
            }
            // El `+` llega como `Plus` o como `Equals` según el teclado y la
            // distribución; en el numérico, además, por otra tecla física.
            if cmd && (i.key_pressed(K::Plus) || i.key_pressed(K::Equals)) {
                cmds.push(Cmd::ZoomIn);
            }
            if cmd && i.key_pressed(K::Minus) {
                cmds.push(Cmd::ZoomOut);
            }
            if cmd && i.key_pressed(K::Num0) {
                cmds.push(Cmd::Zoom100);
            }
            if i.key_pressed(K::Delete) || i.key_pressed(K::Backspace) {
                cmds.push(Cmd::Delete);
            }
        });
        // Escape y Enter cierran una curva o un polígono a medio hacer.
        let close =
            ctx.input(|i| i.key_pressed(egui::Key::Escape) || i.key_pressed(egui::Key::Enter));
        if close {
            if self.doc.is_multistep_active() {
                self.doc.finish_multistep();
            } else {
                self.doc.commit_selection();
            }
        }
        cmds
    }

    // --------------------------------------------------------------- lienzo

    fn canvas_area(&mut self, ui: &mut egui::Ui) {
        let theme = self.theme().clone();
        let zoom = self.zoom();
        let (cw, ch) = (self.doc.canvas.w, self.doc.canvas.h);

        ui.painter()
            .rect_filled(ui.max_rect(), 0.0, Color32::from(theme.workspace));

        // Con reglas hay que reservarles el lugar: antes se dibujaban 8 px por
        // encima del lienzo con sólo 6 px de margen, así que quedaban
        // recortadas y la casilla parecía no hacer nada.
        const RULER: f32 = 22.0;
        let pad = if self.show_rulers { RULER + 4.0 } else { 6.0 };

        egui::ScrollArea::both()
            .id_salt("lienzo")
            // Siempre visibles, aunque el lienzo entre entero: así se ve de un
            // vistazo si hay algo fuera de cuadro.
            .scroll_bar_visibility(
                egui::containers::scroll_area::ScrollBarVisibility::AlwaysVisible,
            )
            // Sin esto el área se encoge al tamaño del contenido, así que la
            // barra vertical quedaba flotando en el medio de la ventana con
            // gris a la derecha, en vez de pegada al borde como en Paint.
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(pad);
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let size = vec2(cw as f32 * zoom, ch as f32 * zoom);
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
                    self.canvas_rect = rect;

                    // SW: corchetes de puntería en las cuatro esquinas. Marcan
                    // el área sin encerrarla, que es la diferencia entre un visor
                    // y una ventana.
                    if theme.chrome == theme::Chrome::Holo {
                        let p = ui.painter();
                        let s = egui::Stroke::new(1.0, Color32::from(theme.accent_hot));
                        let l = 18.0;
                        let o = rect.expand(6.0);
                        for (esq, dx, dy) in [
                            (o.left_top(), 1.0, 1.0),
                            (o.right_top(), -1.0, 1.0),
                            (o.left_bottom(), 1.0, -1.0),
                            (o.right_bottom(), -1.0, -1.0),
                        ] {
                            p.line_segment([esq, esq + vec2(l * dx, 0.0)], s);
                            p.line_segment([esq, esq + vec2(0.0, l * dy)], s);
                        }
                    }

                    // 2077: los canales separados en los bordes, en vez de sombra.
                    // En un editor de píxeles la aberración cromática es una
                    // referencia que se entiende sola.
                    if theme.chrome == theme::Chrome::Neon {
                        let p = ui.painter();
                        for (dx, dy, c) in [
                            (-3.0, 0.0, theme.accent),
                            (3.0, 0.0, theme.accent_alt),
                            (0.0, 4.0, theme.accent_hot),
                        ] {
                            p.rect_stroke(
                                rect.translate(vec2(dx, dy)),
                                0.0,
                                egui::Stroke::new(2.0, Color32::from(c)),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }

                    // En el chrome propio el lienzo flota: papel sobre una mesa,
                    // no un widget dentro de otro. Los clones no llevan sombra —
                    // Paint no la tiene y ahí la gracia es parecerse.
                    // Marco hundido de Windows clásico alrededor del lienzo.
                    if theme.chrome == theme::Chrome::Palette {
                        let sombra = egui::Stroke::new(1.0, Color32::from_rgb(0x80, 0x80, 0x80));
                        let luz = egui::Stroke::new(1.0, Color32::WHITE);
                        let o = rect.expand(2.0);
                        let p = ui.painter();
                        p.line_segment([o.left_bottom(), o.left_top()], sombra);
                        p.line_segment([o.left_top(), o.right_top()], sombra);
                        p.line_segment([o.right_top(), o.right_bottom()], luz);
                        p.line_segment([o.right_bottom(), o.left_bottom()], luz);
                    }
                    if theme.chrome == theme::Chrome::Studio {
                        for (d, a) in [(10.0, 22u8), (6.0, 26), (3.0, 30), (1.0, 46)] {
                            ui.painter().rect_filled(
                                rect.translate(vec2(0.0, d * 0.45)).expand(d),
                                0.0,
                                Color32::from_black_alpha(a),
                            );
                        }
                    }

                    // El bitmap.
                    if let Some(t) = &self.tex {
                        ui.painter().image(
                            t.id(),
                            rect,
                            egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                            Color32::WHITE,
                        );
                    }
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                        egui::StrokeKind::Outside,
                    );

                    // Reglas: dos franjas graduadas con números, como en Paint.
                    if self.show_rulers {
                        let p = ui.painter();
                        let bg = Color32::from(theme.surface_alt);
                        let line = Color32::from(theme.border_strong);
                        let txt = Color32::from(theme.text_dim);

                        let top = egui::Rect::from_min_max(
                            Pos2::new(rect.left() - RULER, rect.top() - RULER),
                            Pos2::new(rect.right(), rect.top()),
                        );
                        let left = egui::Rect::from_min_max(
                            Pos2::new(rect.left() - RULER, rect.top() - RULER),
                            Pos2::new(rect.left(), rect.bottom()),
                        );
                        for r in [top, left] {
                            p.rect_filled(r, 0.0, bg);
                            p.rect_stroke(
                                r,
                                0.0,
                                egui::Stroke::new(1.0, line),
                                egui::StrokeKind::Inside,
                            );
                        }

                        // Marca gruesa y número cada 100 px de lienzo; fina cada 10.
                        // Con poco zoom se ralea para que no quede un borrón.
                        let minor = if zoom >= 2.0 {
                            10
                        } else if zoom >= 0.5 {
                            50
                        } else {
                            200
                        };
                        let major = if zoom >= 0.5 { 100 } else { 500 };
                        let f = egui::FontId::proportional(9.0);

                        for x in (0..=cw).step_by(minor) {
                            let sx = rect.left() + x as f32 * zoom;
                            if sx > rect.right() {
                                break;
                            }
                            let big = x % major == 0;
                            let h = if big { 9.0 } else { 4.0 };
                            p.line_segment(
                                [
                                    Pos2::new(sx, rect.top() - h),
                                    Pos2::new(sx, rect.top() - 1.0),
                                ],
                                egui::Stroke::new(1.0, line),
                            );
                            if big && x > 0 {
                                p.text(
                                    Pos2::new(sx + 2.0, rect.top() - RULER + 2.0),
                                    egui::Align2::LEFT_TOP,
                                    x.to_string(),
                                    f.clone(),
                                    txt,
                                );
                            }
                        }
                        for y in (0..=ch).step_by(minor) {
                            let sy = rect.top() + y as f32 * zoom;
                            if sy > rect.bottom() {
                                break;
                            }
                            let big = y % major == 0;
                            let w = if big { 9.0 } else { 4.0 };
                            p.line_segment(
                                [
                                    Pos2::new(rect.left() - w, sy),
                                    Pos2::new(rect.left() - 1.0, sy),
                                ],
                                egui::Stroke::new(1.0, line),
                            );
                            if big && y > 0 {
                                p.text(
                                    Pos2::new(rect.left() - RULER + 2.0, sy + 1.0),
                                    egui::Align2::LEFT_TOP,
                                    y.to_string(),
                                    f.clone(),
                                    txt,
                                );
                            }
                        }

                        // Dónde está el cursor, como el original.
                        if let Some(m) = resp.hover_pos() {
                            let s = egui::Stroke::new(1.0, Color32::from(theme.accent));
                            p.line_segment(
                                [
                                    Pos2::new(m.x, rect.top() - RULER),
                                    Pos2::new(m.x, rect.top()),
                                ],
                                s,
                            );
                            p.line_segment(
                                [
                                    Pos2::new(rect.left() - RULER, m.y),
                                    Pos2::new(rect.left(), m.y),
                                ],
                                s,
                            );
                        }
                    }

                    // Cuadrícula. Antes tenía una compuerta a 400% de zoom, así que
                    // a tamaño normal la casilla no hacía nada visible.
                    if self.show_grid && zoom >= 1.0 {
                        let g = Color32::from_rgba_unmultiplied(0, 0, 0, 36);
                        let s = egui::Stroke::new(1.0, g);
                        // Con celdas de menos de 4 px la grilla es un borrón gris:
                        // se salta de a diez, como hace Paint.
                        let step = if zoom >= 4.0 { 1 } else { 10 };
                        for x in (0..=cw).step_by(step) {
                            let px = rect.left() + x as f32 * zoom;
                            ui.painter().line_segment(
                                [Pos2::new(px, rect.top()), Pos2::new(px, rect.bottom())],
                                s,
                            );
                        }
                        for y in (0..=ch).step_by(step) {
                            let py = rect.top() + y as f32 * zoom;
                            ui.painter().line_segment(
                                [Pos2::new(rect.left(), py), Pos2::new(rect.right(), py)],
                                s,
                            );
                        }
                    }

                    self.handle_pointer(ui, &resp, rect, zoom);
                    self.set_cursor(ui, &resp, rect, zoom);
                    self.draw_overlays(ui, rect, zoom, &theme);
                    self.resize_handles(ui, rect, zoom, &theme);
                });
                ui.add_space(6.0);
            });
    }

    /// El cursor cambia con la herramienta, como en Paint.
    ///
    /// Con dos criterios distintos a propósito:
    ///
    /// - Las herramientas que **cargan un tamaño** —lápiz, pincel y goma— se
    ///   dibujan solas: se esconde el cursor del sistema y se pinta el contorno
    ///   del grosor real. Paint usa un dibujito fijo y no te deja ver cuán
    ///   grande es la goma hasta que borrás algo; esto es mejor y sale gratis.
    /// - El resto usa un cursor del sistema. egui sólo puede pedir los treinta
    ///   y pico que trae el sistema operativo —no acepta mapas de bits—, y para
    ///   una cruz o una lupa el del sistema es el correcto igual.
    fn set_cursor(&self, ui: &egui::Ui, resp: &egui::Response, rect: egui::Rect, zoom: f32) {
        use egui::CursorIcon as C;
        if !resp.hovered() {
            return;
        }
        let ctx = ui.ctx();

        // Sobre una selección el cursor dice qué va a pasar al arrastrar:
        // estirar por la manija, o mover desde adentro.
        if self.doc.tool == Tool::Select {
            if let (Some(sel), Some(m)) = (&self.doc.sel, ui.input(|i| i.pointer.hover_pos())) {
                let c = ((m.x - rect.left()) / zoom, (m.y - rect.top()) / zoom);
                let grab = 7.0 / zoom;
                for i in 0..8 {
                    let h = sel.handle(i);
                    if (c.0 - h.0).abs() <= grab && (c.1 - h.1).abs() <= grab {
                        ctx.set_cursor_icon(match i {
                            0 | 4 => C::ResizeNwSe,
                            2 | 6 => C::ResizeNeSw,
                            1 | 5 => C::ResizeVertical,
                            _ => C::ResizeHorizontal,
                        });
                        return;
                    }
                }
                if sel.r.contains(c.0 as i32, c.1 as i32) {
                    ctx.set_cursor_icon(C::Move);
                    return;
                }
            }
        }

        match self.doc.tool {
            Tool::Pencil | Tool::Brush | Tool::Eraser => {
                ctx.set_cursor_icon(C::None);
                if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                    let w = (self.doc.width * zoom).max(6.0);
                    // Dos trazos, claro sobre oscuro: un contorno de un solo
                    // color desaparece sobre el dibujo justo cuando más falta.
                    let fuera = egui::Stroke::new(3.0, Color32::from_black_alpha(120));
                    let dentro = egui::Stroke::new(1.0, Color32::WHITE);
                    if self.doc.tool == Tool::Eraser {
                        // La goma de Paint es cuadrada, y se nota al borrar.
                        let q = egui::Rect::from_center_size(m, vec2(w, w));
                        for s in [fuera, dentro] {
                            ui.painter()
                                .rect_stroke(q, 0.0, s, egui::StrokeKind::Middle);
                        }
                    } else {
                        for s in [fuera, dentro] {
                            ui.painter().circle_stroke(m, w / 2.0, s);
                        }
                    }
                }
            }
            Tool::Text => ctx.set_cursor_icon(C::Text),
            Tool::Magnifier => ctx.set_cursor_icon(C::ZoomIn),
            // El cuentagotas se dibuja: lo que importa es **qué píxel** vas a
            // tomar, y una cruz del sistema no dice cuál ni qué color tiene.
            // La punta cae exacta sobre el píxel y al lado va la muestra de lo
            // que hay debajo — eso Paint no lo hace y acá sale gratis.
            Tool::Picker => {
                ctx.set_cursor_icon(C::None);
                if let Some(m) = ui.input(|i| i.pointer.hover_pos()) {
                    let p = ui.painter();
                    let fuera = egui::Stroke::new(3.0, Color32::from_black_alpha(140));
                    let dentro = egui::Stroke::new(1.4, Color32::WHITE);
                    // El cuerpo, en diagonal, con la punta en el puntero.
                    let cuerpo = [m + vec2(3.0, -3.0), m + vec2(13.0, -13.0)];
                    let bulbo = [m + vec2(11.0, -15.0), m + vec2(17.0, -9.0)];
                    for st in [fuera, dentro] {
                        p.line_segment(cuerpo, st);
                        p.line_segment(bulbo, st);
                        p.line_segment([m, m + vec2(4.0, -4.0)], st);
                    }
                    // La muestra de lo que hay debajo.
                    if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                        let cx = ((pos.x - rect.left()) / zoom) as i32;
                        let cy = ((pos.y - rect.top()) / zoom) as i32;
                        if let Some(c) = self.doc.canvas.geti(cx, cy) {
                            let sw =
                                egui::Rect::from_min_size(m + vec2(8.0, 4.0), vec2(16.0, 16.0));
                            p.rect_filled(sw, 0.0, c);
                            p.rect_stroke(
                                sw,
                                0.0,
                                egui::Stroke::new(1.0, Color32::WHITE),
                                egui::StrokeKind::Outside,
                            );
                            p.rect_stroke(
                                sw.expand(1.0),
                                0.0,
                                egui::Stroke::new(1.0, Color32::from_black_alpha(140)),
                                egui::StrokeKind::Outside,
                            );
                        }
                    }
                }
            }
            Tool::Fill => ctx.set_cursor_icon(C::Cell),
            Tool::Shape | Tool::Select => ctx.set_cursor_icon(C::Crosshair),
        }
    }

    /// Se maneja con el estado crudo del puntero, no con `Response::clicked` ni
    /// `dragged`. Con `Sense::click_and_drag` egui **posterga** la decisión de
    /// si fue clic o arrastre hasta que el cursor se mueve unos 6 px, así que
    /// `drag_started` llega tarde y todo trazo perdía su comienzo. Para una
    /// superficie de dibujo eso es inaceptable: acá el trazo empieza donde
    /// bajaste el botón.
    fn handle_pointer(
        &mut self,
        ui: &egui::Ui,
        resp: &egui::Response,
        rect: egui::Rect,
        zoom: f32,
    ) {
        let to_canvas = |p: Pos2| ((p.x - rect.left()) / zoom, (p.y - rect.top()) / zoom);

        let (l_down, r_down, released, pos, shift, held) = ui.input(|i| {
            (
                i.pointer.primary_pressed(),
                i.pointer.secondary_pressed(),
                i.pointer.any_released(),
                i.pointer.interact_pos(),
                i.modifiers.shift,
                i.pointer.primary_down(),
            )
        });
        let Some(pos) = pos else { return };
        let c = to_canvas(pos);

        if resp.hovered() {
            self.status = format!("{}, {} px", c.0 as i32, c.1 as i32);
        }

        // La lupa hace zoom acá, no en el documento: es cosa de la vista.
        if self.doc.tool == Tool::Magnifier {
            if resp.hovered() && l_down {
                self.zoom_idx = (self.zoom_idx + 1).min(ZOOMS.len() - 1);
            } else if resp.hovered() && r_down {
                self.zoom_idx = self.zoom_idx.saturating_sub(1);
            }
            return;
        }

        // El texto no pinta: abre un cuadro sobre el lienzo y se edita ahí.
        if self.doc.tool == Tool::Text {
            self.text_pointer(resp, c, l_down, held, released, zoom);
            return;
        }

        if (l_down || r_down) && resp.hovered() {
            self.drawing = true;
            // Las manijas miden lo mismo en pantalla a cualquier zoom, así
            // que el radio de agarre se mide en píxeles del lienzo.
            self.doc.down(c, r_down, 7.0 / zoom);
            self.sel_dirty = true;
        } else if self.drawing {
            if released {
                self.doc.up(c);
                self.drawing = false;
                self.sel_dirty = true;
            } else {
                self.doc.drag_to(c, shift);
            }
        }
    }

    /// Los tres tiradores blancos de Paint: derecha, abajo y esquina. Arrastran
    /// el **lienzo**, no la imagen: lo que aparece se rellena con el Color 2.
    /// Crear, mover y estirar el cuadro de texto.
    ///
    /// Un clic afuera **confirma** el cuadro anterior y empieza uno nuevo, que
    /// es lo que hace Paint: no hay botón de aceptar, salir es aceptar.
    fn text_pointer(
        &mut self,
        resp: &egui::Response,
        c: (f32, f32),
        l_down: bool,
        held: bool,
        released: bool,
        zoom: f32,
    ) {
        if released {
            // Un clic sin arrastre deja una caja de cero: se le da un tamaño
            // usable en vez de una raya invisible.
            if self.t_grab == Some(TGrab::New) {
                if let Some(tb) = self.text_box.as_mut() {
                    if tb.w < text::MIN_W || tb.h < text::MIN_H {
                        let (x, y) = (tb.x, tb.y);
                        *tb = TextBox::default_at(x, y);
                    }
                }
            }
            self.t_grab = None;
            return;
        }

        if l_down && resp.hovered() {
            // Las manijas miden lo mismo en pantalla a cualquier zoom, así que
            // el radio de agarre se mide en píxeles del lienzo.
            let grab = 7.0 / zoom;
            let start_new = match self.text_box.as_ref() {
                Some(tb) => {
                    let hit = T_HANDLES.iter().position(|(hx, hy)| {
                        let px = tb.x + hx * tb.w;
                        let py = tb.y + hy * tb.h;
                        (c.0 - px).abs() <= grab && (c.1 - py).abs() <= grab
                    });
                    let inside =
                        c.0 >= tb.x && c.0 <= tb.x + tb.w && c.1 >= tb.y && c.1 <= tb.y + tb.h;
                    match hit {
                        Some(i) => {
                            self.t_grab = Some(TGrab::Handle(i));
                            false
                        }
                        // Sobre el borde: se mueve entero. Adentro: el clic es
                        // del cuadro de edición, para poner el cursor.
                        None if inside => {
                            let borde = (c.0 - tb.x).abs() <= grab
                                || (c.0 - tb.x - tb.w).abs() <= grab
                                || (c.1 - tb.y).abs() <= grab
                                || (c.1 - tb.y - tb.h).abs() <= grab;
                            if borde {
                                self.t_grab = Some(TGrab::Move);
                            }
                            false
                        }
                        None => true,
                    }
                }
                None => true,
            };

            if start_new {
                let ctx = resp.ctx.clone();
                self.commit_text(&ctx);
                self.text_box = Some(TextBox::new(c.0, c.1, 0.0, 0.0));
                self.tab = Tab::Text;
                self.t_grab = Some(TGrab::New);
            }
            self.t_from = c;
            if let Some(tb) = self.text_box.as_ref() {
                self.t_orig = (tb.x, tb.y, tb.w, tb.h);
            }
        }

        if !held {
            return;
        }
        let Some(g) = self.t_grab else { return };
        let o = self.t_orig;
        let from = self.t_from;
        // Los tres se leen antes de pedir el préstamo mutable: `self` no puede
        // estar prestado de las dos formas en la misma expresión.
        let Some(tb) = self.text_box.as_mut() else {
            return;
        };
        match g {
            TGrab::New => {
                tb.x = from.0.min(c.0);
                tb.y = from.1.min(c.1);
                tb.w = (c.0 - from.0).abs();
                tb.h = (c.1 - from.1).abs();
            }
            TGrab::Move => {
                tb.x = o.0 + (c.0 - from.0);
                tb.y = o.1 + (c.1 - from.1);
            }
            TGrab::Handle(i) => {
                let (hx, hy) = T_HANDLES[i];
                let (mut l, mut t, mut r, mut b) = (o.0, o.1, o.0 + o.2, o.1 + o.3);
                // El eje con 0.5 no se toca; los otros se topan contra el
                // mínimo para que la caja no se dé vuelta al pasarse de largo.
                if hx == 0.0 {
                    l = c.0.min(r - text::MIN_W);
                } else if hx == 1.0 {
                    r = c.0.max(l + text::MIN_W);
                }
                if hy == 0.0 {
                    t = c.1.min(b - text::MIN_H);
                } else if hy == 1.0 {
                    b = c.1.max(t + text::MIN_H);
                }
                tb.x = l;
                tb.y = t;
                tb.w = r - l;
                tb.h = b - t;
            }
        }
    }

    /// Vuelca el cuadro al lienzo y lo cierra. Sin cuadro, o vacío, no hace
    /// nada: confirmar algo que no escribiste no debería ensuciar el historial.
    fn commit_text(&mut self, ctx: &egui::Context) {
        let Some(tb) = self.text_box.take() else {
            return;
        };
        self.t_grab = None;
        self.tab = Tab::Home;
        if !tb.s.is_empty() {
            self.doc.commit_selection();
            text::rasterize(
                ctx,
                &mut self.doc.canvas,
                &tb,
                self.doc.color1,
                self.doc.color2,
            );
        }
    }

    fn resize_handles(&mut self, ui: &mut egui::Ui, rect: egui::Rect, zoom: f32, theme: &Theme) {
        let (cw, ch) = (self.doc.canvas.w, self.doc.canvas.h);
        let spots = [
            (Pos2::new(rect.right(), rect.center().y), true, false),
            (Pos2::new(rect.center().x, rect.bottom()), false, true),
            (Pos2::new(rect.right(), rect.bottom()), true, true),
        ];
        for (i, (p, hx, hy)) in spots.iter().enumerate() {
            let hr = egui::Rect::from_center_size(*p, vec2(10.0, 10.0));
            let resp = ui.interact(hr, ui.id().with(("tirador", i)), Sense::drag());
            ui.painter()
                .rect_filled(hr.shrink(2.0), 0.0, Color32::WHITE);
            ui.painter().rect_stroke(
                hr.shrink(2.0),
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                egui::StrokeKind::Inside,
            );

            if resp.dragged() {
                if let Some(m) = resp.interact_pointer_pos() {
                    let nw = if *hx {
                        (((m.x - rect.left()) / zoom).round() as i64).max(1) as usize
                    } else {
                        cw
                    };
                    let nh = if *hy {
                        (((m.y - rect.top()) / zoom).round() as i64).max(1) as usize
                    } else {
                        ch
                    };
                    self.resizing = Some((nw.min(8000), nh.min(8000)));
                }
            }
            if resp.drag_stopped() {
                if let Some((nw, nh)) = self.resizing.take() {
                    let bg = self.doc.color2;
                    self.doc.commit_selection();
                    self.doc.canvas.resize_canvas(nw, nh, bg);
                    self.tex = None;
                }
            }
        }

        // Contorno de lo que va a quedar, mientras se arrastra.
        if let Some((nw, nh)) = self.resizing {
            let pv = egui::Rect::from_min_size(rect.min, vec2(nw as f32 * zoom, nh as f32 * zoom));
            ui.painter().rect_stroke(
                pv,
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.accent)),
                egui::StrokeKind::Outside,
            );
            self.status = format!("{nw} × {nh} px");
        }
    }

    fn draw_overlays(&self, ui: &egui::Ui, rect: egui::Rect, zoom: f32, theme: &Theme) {
        let to_screen = |x: f32, y: f32| Pos2::new(rect.left() + x * zoom, rect.top() + y * zoom);
        let p = ui.painter();

        // Vista previa de la forma en curso, encima del bitmap.
        if !self.doc.preview.is_empty() {
            let pts = &self.doc.preview;
            let s = egui::Stroke::new(1.0, self.doc.color1);
            for i in 0..pts.len().saturating_sub(1) {
                p.line_segment(
                    [
                        to_screen(pts[i].0, pts[i].1),
                        to_screen(pts[i + 1].0, pts[i + 1].1),
                    ],
                    s,
                );
            }
            if self.doc.preview_closed && pts.len() > 2 {
                let (a, b) = (pts[pts.len() - 1], pts[0]);
                p.line_segment([to_screen(a.0, a.1), to_screen(b.0, b.1)], s);
            }
        }

        // Selección: los píxeles flotantes y el marco.
        if let Some(sel) = &self.doc.sel {
            let sr = egui::Rect::from_min_size(
                to_screen(sel.r.x as f32, sel.r.y as f32),
                vec2(sel.r.w as f32 * zoom, sel.r.h as f32 * zoom),
            );
            if let Some(t) = &self.sel_tex {
                p.image(
                    t.id(),
                    sr,
                    egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    Color32::WHITE,
                );
            }
            // Doble marco para que se vea sobre cualquier fondo. egui no tiene
            // línea punteada de fábrica, y esto cumple igual.
            p.rect_stroke(
                sr,
                0.0,
                egui::Stroke::new(1.0, Color32::WHITE),
                egui::StrokeKind::Outside,
            );
            p.rect_stroke(
                sr.expand(1.0),
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.text)),
                egui::StrokeKind::Outside,
            );

            // Las ocho manijas para estirarla. Miden lo mismo en pantalla a
            // cualquier zoom: son para el mouse, no parte del dibujo.
            for (hx, hy) in doc::SEL_HANDLES {
                let c = Pos2::new(sr.left() + hx * sr.width(), sr.top() + hy * sr.height());
                let hr = egui::Rect::from_center_size(c, vec2(7.0, 7.0));
                p.rect_filled(hr, 0.0, Color32::WHITE);
                p.rect_stroke(
                    hr,
                    0.0,
                    egui::Stroke::new(1.0, Color32::from(theme.text)),
                    egui::StrokeKind::Inside,
                );
            }
        }
    }

    // ------------------------------------------------------------- diálogos

    /// Los nueve desplegables anclados, todos juntos.
    ///
    /// Van en una función propia a propósito: cuando estaban sueltos entre los
    /// diálogos, un reemplazo del menú Archivo se los llevó por delante y la
    /// mitad de la cinta dejó de responder sin que nada fallara al compilar.
    fn anchored_menus(&mut self, ctx: &egui::Context, theme: &Theme, pending: &mut Vec<Cmd>) {
        let anchor = self.menu_anchor;

        if self.dialogs.paste_menu {
            let keep = ui::menu_panel(ctx, theme, "m_pegar", anchor, 176.0, |ui| {
                let mut stay = true;
                if ui::menu_row(ui, theme, Icon::I(Ico::Paste), lang::t("Pegar"), None) {
                    pending.push(Cmd::Paste);
                    stay = false;
                }
                if ui::menu_row(ui, theme, Icon::I(Ico::Open), lang::t("Pegar desde…"), None) {
                    pending.push(Cmd::PasteFrom);
                    stay = false;
                }
                stay
            });
            self.dialogs.paste_menu = keep;
        }

        if self.dialogs.select_menu {
            let mode = self.doc.select_mode;
            let transp = self.doc.transparent_selection;
            let mut set_mode = None;
            let mut flip_transp = false;
            let keep = ui::menu_panel(ctx, theme, "m_sel", anchor, 224.0, |ui| {
                use doc::SelectMode::*;
                let mut stay = true;
                for (label, m) in [
                    ("Selección rectangular", Rectangular),
                    ("Selección de forma libre", FreeForm),
                ] {
                    if ui::menu_row(ui, theme, Icon::None, label, Some(mode == m)) {
                        set_mode = Some(m);
                        stay = false;
                    }
                }
                ui::menu_sep(ui, theme);
                if ui::menu_row(ui, theme, Icon::None, lang::t("Seleccionar todo"), None) {
                    pending.push(Cmd::SelectAll);
                    stay = false;
                }
                if ui::menu_row(ui, theme, Icon::None, lang::t("Eliminar"), None) {
                    pending.push(Cmd::Delete);
                    stay = false;
                }
                ui::menu_sep(ui, theme);
                if ui::menu_row(
                    ui,
                    theme,
                    Icon::None,
                    lang::t("Selección transparente"),
                    Some(transp),
                ) {
                    flip_transp = true;
                }
                stay
            });
            if let Some(m) = set_mode {
                self.doc.select_mode = m;
            }
            if flip_transp {
                self.doc.transparent_selection = !transp;
            }
            self.dialogs.select_menu = keep;
        }

        if self.dialogs.brushes {
            self.build_brush_previews(ctx);
            let cur = self.doc.brush;
            let tex = self.brush_tex.clone();
            let mut pick = None;
            let keep = ui::menu_panel(ctx, theme, "m_pinceles", anchor, 4.0 * 42.0 + 8.0, |ui| {
                let mut stay = true;
                let n = shapes::ALL_BRUSHES.len() as f32;
                for (row, chunk) in shapes::ALL_BRUSHES.chunks(4).enumerate() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = vec2(1.0, 1.0);
                        for (col, b) in chunk.iter().enumerate() {
                            let i = row * 4 + col;
                            let (r, resp) =
                                ui.allocate_exact_size(vec2(40.0, 40.0), Sense::click());
                            if *b == cur {
                                ui.painter().rect_filled(
                                    r,
                                    2.0,
                                    Color32::from(theme.button_active),
                                );
                                ui.painter().rect_stroke(
                                    r,
                                    2.0,
                                    egui::Stroke::new(1.0, Color32::from(theme.accent)),
                                    egui::StrokeKind::Inside,
                                );
                            } else if resp.hovered() {
                                ui.painter()
                                    .rect_filled(r, 2.0, Color32::from(theme.button_hover));
                            }
                            if let Some(t) = &tex {
                                ui.painter().image(
                                    t.id(),
                                    r.shrink(3.0),
                                    egui::Rect::from_min_max(
                                        Pos2::new(0.0, i as f32 / n),
                                        Pos2::new(1.0, (i + 1) as f32 / n),
                                    ),
                                    Color32::WHITE,
                                );
                            }
                            if resp.on_hover_text(b.label()).clicked() {
                                pick = Some(*b);
                                stay = false;
                            }
                        }
                    });
                }
                stay
            });
            if let Some(b) = pick {
                self.doc.brush = b;
                self.doc.set_tool(Tool::Brush);
            }
            self.dialogs.brushes = keep;
        }

        for is_outline in [true, false] {
            let open = if is_outline {
                self.dialogs.outline_menu
            } else {
                self.dialogs.fill_menu
            };
            if !open {
                continue;
            }
            self.build_stroke_previews(ctx);
            let tex = self.stroke_tex.clone();
            let cur = if is_outline {
                self.doc.outline
            } else {
                self.doc.fill_style
            };
            let id = if is_outline {
                "m_contorno"
            } else {
                "m_relleno"
            };
            let mut pick = None;
            let keep = ui::menu_panel(ctx, theme, id, anchor, 182.0, |ui| {
                let mut stay = true;
                let n = doc::ALL_STROKES.len() as f32;
                for (i, st) in doc::ALL_STROKES.iter().enumerate() {
                    let uv = egui::Rect::from_min_max(
                        Pos2::new(0.0, i as f32 / n),
                        Pos2::new(1.0, (i + 1) as f32 / n),
                    );
                    let label = if is_outline {
                        st.outline_label()
                    } else {
                        st.fill_label()
                    };
                    if ui::menu_row_sample(
                        ui,
                        theme,
                        tex.as_ref().map(|t| (t.id(), uv)),
                        *st == doc::Stroke::None,
                        label,
                        *st == cur,
                    ) {
                        pick = Some(*st);
                        stay = false;
                    }
                }
                stay
            });
            if let Some(st) = pick {
                if is_outline {
                    self.doc.outline = st;
                } else {
                    self.doc.fill_style = st;
                }
            }
            if is_outline {
                self.dialogs.outline_menu = keep;
            } else {
                self.dialogs.fill_menu = keep;
            }
        }

        if self.dialogs.rotate_menu {
            let keep = ui::menu_panel(ctx, theme, "m_girar", anchor, 236.0, |ui| {
                let mut stay = true;
                for (label, icon, cmd) in [
                    (
                        "Girar 90° a la derecha",
                        Icon::I(Ico::Rotate),
                        Cmd::Rotate(1),
                    ),
                    (
                        "Girar 90° a la izquierda",
                        Icon::I(Ico::Rotate),
                        Cmd::Rotate(3),
                    ),
                    ("Girar 180°", Icon::I(Ico::Rotate), Cmd::Rotate(2)),
                    ("Voltear horizontalmente", Icon::I(Ico::FlipH), Cmd::FlipH),
                    ("Voltear verticalmente", Icon::I(Ico::FlipV), Cmd::FlipV),
                ] {
                    if ui::menu_row(ui, theme, icon, label, None) {
                        pending.push(cmd);
                        stay = false;
                    }
                }
                ui::menu_sep(ui, theme);
                // No está en la cinta de Paint —vive en el menú contextual del
                // lienzo— pero acá queda alcanzable sin depender del atajo.
                if ui::menu_row(ui, theme, Icon::None, lang::t("Invertir color"), None) {
                    pending.push(Cmd::InvertColors);
                    stay = false;
                }
                stay
            });
            self.dialogs.rotate_menu = keep;
        }

        if self.dialogs.size_menu {
            let widths = if self.doc.tool == Tool::Eraser {
                [4.0f32, 6.0, 8.0, 10.0]
            } else {
                [1.0f32, 3.0, 5.0, 8.0]
            };
            let cur = self.doc.width;
            let limit = if self.doc.tool == Tool::Eraser {
                100.0
            } else {
                50.0
            };
            let mut custom = cur;
            let mut custom_changed = false;
            let mut pick = None;
            let keep = ui::menu_panel(ctx, theme, "m_tamano", anchor, 196.0, |ui| {
                let mut stay = true;
                for w in widths {
                    if ui::menu_row_width(ui, theme, w, (w - cur).abs() < 0.5) {
                        pick = Some(w);
                        stay = false;
                    }
                }
                ui::menu_sep(ui, theme);
                ui.label(lang::t("Grosor"));
                custom_changed = ui
                    .add(
                        egui::Slider::new(&mut custom, 1.0..=limit)
                            .step_by(1.0)
                            .suffix(" px")
                            .fixed_decimals(0),
                    )
                    .changed();
                stay
            });
            if let Some(w) = pick {
                self.doc.width = w;
            } else if custom_changed {
                self.doc.width = custom;
            }
            self.dialogs.size_menu = keep;
        }

        if self.dialogs.theme_menu {
            // Una fila por familia. Antes salían las dieciocho variantes y la
            // mitad de la lista era «… oscuro», que es justo lo que ya decide
            // el interruptor de Configuración.
            let names = theme::families(&self.themes, theme.dark);
            let cur = self.theme_idx;
            let mut pick = None;
            let keep = ui::menu_panel(ctx, theme, "m_tema", anchor, 194.0, |ui| {
                let mut stay = true;
                for (name, i) in &names {
                    if ui::menu_row(ui, theme, Icon::None, name, Some(*i == cur)) {
                        pick = Some(*i);
                        stay = false;
                    }
                }
                stay
            });
            if let Some(i) = pick {
                pending.push(Cmd::SetTheme(i));
            }
            self.dialogs.theme_menu = keep;
        }

        if self.dialogs.qat_menu {
            let flags = self.qat;
            let below = self.qat_below;
            let minimized = self.ribbon_min;
            let mut toggled = None;
            let keep = ui::menu_panel(ctx, theme, "m_qat", anchor, 272.0, |ui| {
                let mut stay = true;
                for (i, item) in ui::ALL_QAT.iter().enumerate() {
                    if ui::menu_row(ui, theme, Icon::None, item.label(), Some(flags[i])) {
                        toggled = Some(i);
                    }
                }
                ui::menu_sep(ui, theme);
                let mover = if below {
                    "Mostrar encima de la cinta"
                } else {
                    "Mostrar debajo de la cinta"
                };
                if ui::menu_row(ui, theme, Icon::None, mover, None) {
                    pending.push(Cmd::ToggleQatBelow);
                    stay = false;
                }
                let colapsar = if minimized {
                    "Expandir la cinta"
                } else {
                    "Minimizar la cinta"
                };
                if ui::menu_row(ui, theme, Icon::None, colapsar, None) {
                    pending.push(Cmd::ToggleRibbonMin);
                    stay = false;
                }
                stay
            });
            if let Some(i) = toggled {
                self.qat[i] = !self.qat[i];
            }
            self.dialogs.qat_menu = keep;
        }
    }

    fn dialogs(&mut self, ctx: &egui::Context) -> Vec<Cmd> {
        let mut pending: Vec<Cmd> = Vec::new();
        // Lo usan tanto los diálogos como los menús anclados.
        let theme = self.theme().clone();
        self.anchored_menus(ctx, &theme, &mut pending);
        let mut resize = self.dialogs.resize;
        if resize {
            const W: f32 = 340.0;
            const IW: f32 = W - 32.0;
            egui::Window::new("resize_dlg")
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .constrain(true)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(ui::dialog_frame(&theme))
                .show(ctx, |ui| {
                    ui.set_width(W);
                    if ui::dialog_header(ui, &theme, W, lang::t("Cambiar tamaño y sesgar")) {
                        resize = false;
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(16.0);
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 2.0;

                            caption(ui, &theme, lang::t("Cambiar tamaño"));
                            ui.horizontal(|ui| {
                                if ui::dlg_radio(
                                    ui,
                                    &theme,
                                    lang::t("Porcentaje"),
                                    self.dialogs.by_percent,
                                ) {
                                    self.dialogs.by_percent = true;
                                    self.dialogs.rw = 100.0;
                                    self.dialogs.rh = 100.0;
                                }
                                ui.add_space(10.0);
                                if ui::dlg_radio(
                                    ui,
                                    &theme,
                                    lang::t("Píxeles"),
                                    !self.dialogs.by_percent,
                                ) {
                                    // Al cambiar de unidad los campos pasan al
                                    // tamaño real: dejar «100» ahí querría decir
                                    // 100 píxeles, y eso encoge el dibujo sin
                                    // que nadie lo haya pedido.
                                    self.dialogs.by_percent = false;
                                    self.dialogs.rw = self.doc.canvas.w as f32;
                                    self.dialogs.rh = self.doc.canvas.h as f32;
                                }
                            });
                            ui.add_space(4.0);

                            let (tope, suf) = if self.dialogs.by_percent {
                                (1.0..=800.0, " %")
                            } else {
                                (1.0..=8000.0, " px")
                            };
                            // La proporción se guarda antes de tocar nada: en
                            // píxeles, atar el alto al ancho **con el mismo
                            // número** deformaría todo lo que no sea cuadrado.
                            let prop = self.doc.canvas.h as f32 / self.doc.canvas.w.max(1) as f32;

                            let mut rw = self.dialogs.rw;
                            let mut rh = self.dialogs.rh;
                            if ui::dlg_field(
                                ui,
                                &theme,
                                lang::t("Horizontal"),
                                &mut rw,
                                tope.clone(),
                                suf,
                            ) && self.dialogs.keep_ratio
                            {
                                rh = if self.dialogs.by_percent {
                                    rw
                                } else {
                                    (rw * prop).round()
                                };
                            }
                            if ui::dlg_field(ui, &theme, lang::t("Vertical"), &mut rh, tope, suf)
                                && self.dialogs.keep_ratio
                            {
                                rw = if self.dialogs.by_percent {
                                    rh
                                } else {
                                    (rh / prop).round()
                                };
                            }
                            self.dialogs.rw = rw;
                            self.dialogs.rh = rh;

                            ui.add_space(4.0);
                            if ui::dlg_check(
                                ui,
                                &theme,
                                lang::t("Mantener relación de aspecto"),
                                self.dialogs.keep_ratio,
                            ) {
                                self.dialogs.keep_ratio = !self.dialogs.keep_ratio;
                            }

                            ui::dlg_sep(ui, &theme, IW);

                            caption(ui, &theme, lang::t("Sesgar (grados)"));
                            ui.add_space(4.0);
                            let mut sx = self.dialogs.skew_x;
                            let mut sy = self.dialogs.skew_y;
                            ui::dlg_field(
                                ui,
                                &theme,
                                lang::t("Horizontal"),
                                &mut sx,
                                -89.0..=89.0,
                                "°",
                            );
                            ui::dlg_field(
                                ui,
                                &theme,
                                lang::t("Vertical"),
                                &mut sy,
                                -89.0..=89.0,
                                "°",
                            );
                            self.dialogs.skew_x = sx;
                            self.dialogs.skew_y = sy;
                            ui.add_space(12.0);
                        });
                    });

                    match ui::dialog_footer(
                        ui,
                        &theme,
                        W,
                        lang::t("Aceptar"),
                        Some(lang::t("Cancelar")),
                    ) {
                        Some(true) => {
                            self.doc.commit_selection();
                            let (w, h) = (self.doc.canvas.w as f32, self.doc.canvas.h as f32);
                            let (nw, nh) = if self.dialogs.by_percent {
                                (w * self.dialogs.rw / 100.0, h * self.dialogs.rh / 100.0)
                            } else {
                                (self.dialogs.rw, self.dialogs.rh)
                            };
                            let nw = nw.round().max(1.0) as usize;
                            let nh = nh.round().max(1.0) as usize;
                            if nw != self.doc.canvas.w || nh != self.doc.canvas.h {
                                self.doc.canvas.scale(nw, nh);
                            }
                            if self.dialogs.skew_x != 0.0 || self.dialogs.skew_y != 0.0 {
                                let bg = self.doc.color2;
                                self.doc
                                    .canvas
                                    .skew(self.dialogs.skew_x, self.dialogs.skew_y, bg);
                            }
                            self.tex = None;
                            resize = false;
                        }
                        Some(false) => resize = false,
                        None => {}
                    }
                });
        }
        self.dialogs.resize = resize;

        let mut props = self.dialogs.properties;
        if props {
            const W: f32 = 372.0;
            let (mut w, mut h) = (self.dialogs.pw, self.dialogs.ph);
            let (cw, chh) = (self.doc.canvas.w, self.doc.canvas.h);
            let mut accept = false;

            egui::Window::new("props_dlg")
                // Sin esto egui los deja donde quedaron la última vez, y salían
                // fuera de la ventana.
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .constrain(true)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(ui::dialog_frame(&theme))
                .show(ctx, |ui| {
                    ui.set_width(W);
                    if ui::dialog_header(ui, &theme, W, lang::t("Propiedades de la imagen")) {
                        props = false;
                    }

                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        ui.vertical(|ui| {
                            caption(ui, &theme, lang::t("Tamaño del lienzo"));
                            egui::Grid::new("props_size")
                                .num_columns(2)
                                .spacing(vec2(10.0, 8.0))
                                .show(ui, |ui| {
                                    ui.label(lang::t("Ancho"));
                                    ui.add(
                                        egui::DragValue::new(&mut w)
                                            .range(1.0..=8000.0)
                                            .suffix(" px"),
                                    );
                                    ui.end_row();
                                    ui.label(lang::t("Alto"));
                                    ui.add(
                                        egui::DragValue::new(&mut h)
                                            .range(1.0..=8000.0)
                                            .suffix(" px"),
                                    );
                                    ui.end_row();
                                });
                            // Es lo que más confunde de este diálogo en Paint:
                            // recorta o agranda el lienzo, no escala el dibujo.
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(
                                    "Recorta o agranda el lienzo. Para escalar el dibujo, usá Cambiar tamaño.",
                                )
                                .size(theme.font_size - 1.0)
                                .color(Color32::from(theme.text_dim)),
                            );

                            ui.add_space(14.0);
                            caption(ui, &theme, lang::t("Información"));
                            let iw = W - 28.0;
                            let bytes = cw * chh * 4;
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Actual"),
                                &format!("{cw} × {chh} px"),
                            );
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("En memoria"),
                                &if bytes >= 1 << 20 {
                                    format!("{:.1} MB", bytes as f32 / (1 << 20) as f32)
                                } else {
                                    format!("{} KB", bytes / 1024)
                                },
                            );
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Deshacer"),
                                &format!(
                                    "{} pasos · {:.0} KB",
                                    self.doc.canvas.undo_len(),
                                    self.doc.canvas.undo_bytes() as f32 / 1024.0
                                ),
                            );
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Archivo"),
                                &self
                                    .path
                                    .as_ref()
                                    .and_then(|p| p.file_name())
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| "sin guardar".into()),
                            );
                        });
                        ui.add_space(14.0);
                    });

                    ui.add_space(14.0);
                    match ui::dialog_footer(ui, &theme, W, lang::t("Aceptar"), Some(lang::t("Cancelar"))) {
                        Some(true) => {
                            accept = true;
                            props = false;
                        }
                        Some(false) => props = false,
                        None => {}
                    }
                });

            self.dialogs.pw = w;
            self.dialogs.ph = h;
            if accept {
                let bg = self.doc.color2;
                self.doc.commit_selection();
                self.doc.canvas.resize_canvas(w as usize, h as usize, bg);
                self.tex = None;
            }
        }
        self.dialogs.properties = props;

        let mut color = self.dialogs.color;
        if color {
            self.build_hs_texture(ctx);
            let hs = self.hs_tex.clone();
            let mut hsv = self.dialogs.hsv;
            let target = if self.picking_c1 {
                "Color 1"
            } else {
                "Color 2"
            };
            let mut add_custom = false;
            let mut accept = false;

            // Cabecera propia: la barra de título de egui mide el doble y centra
            // el texto como un cartel.
            egui::Window::new(format!("color_{target}"))
                // Sin esto egui los deja donde quedaron la última vez, y salían
                // fuera de la ventana.
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .constrain(true)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(Color32::from(theme.surface))
                        .stroke(egui::Stroke::new(1.0, Color32::from(theme.border_strong)))
                        .shadow(egui::Shadow {
                            offset: [0, 8],
                            blur: 28,
                            spread: 0,
                            color: Color32::from_black_alpha(56),
                        }),
                )
                .show(ctx, |ui| {
                    ui.set_width(560.0);

                    // --- cabecera ---
                    let (hd, _) = ui.allocate_exact_size(vec2(560.0, 34.0), Sense::hover());
                    ui.painter()
                        .rect_filled(hd, 0.0, Color32::from(theme.surface_alt));
                    ui.painter().line_segment(
                        [
                            Pos2::new(hd.left(), hd.bottom() - 0.5),
                            Pos2::new(hd.right(), hd.bottom() - 0.5),
                        ],
                        egui::Stroke::new(1.0, Color32::from(theme.border)),
                    );
                    ui.painter().text(
                        Pos2::new(hd.left() + 12.0, hd.center().y),
                        egui::Align2::LEFT_CENTER,
                        format!("Modificar colores — {target}"),
                        egui::FontId::proportional(theme.font_size + 0.5),
                        theme.text.into(),
                    );
                    let xr = egui::Rect::from_center_size(
                        Pos2::new(hd.right() - 19.0, hd.center().y),
                        vec2(26.0, 26.0),
                    );
                    let xresp = ui.interact(xr, ui.id().with("cerrar_color"), Sense::click());
                    if xresp.hovered() {
                        ui.painter()
                            .rect_filled(xr, 2.0, Color32::from(theme.button_hover));
                    }
                    let st = egui::Stroke::new(1.2, Color32::from(theme.text_dim));
                    ui.painter().line_segment(
                        [xr.center() + vec2(-4.5, -4.5), xr.center() + vec2(4.5, 4.5)],
                        st,
                    );
                    ui.painter().line_segment(
                        [xr.center() + vec2(4.5, -4.5), xr.center() + vec2(-4.5, 4.5)],
                        st,
                    );
                    if xresp.clicked() {
                        color = false;
                    }

                    let srgb = ecolor::Hsva::new(hsv[0], hsv[1], hsv[2], 1.0).to_srgb();
                    let cur = Color32::from_rgb(srgb[0], srgb[1], srgb[2]);

                    // --- cuerpo ---
                    ui.add_space(12.0);
                    ui.horizontal_top(|ui| {
                        ui.add_space(12.0);

                        ui.vertical(|ui| {
                            caption(ui, &theme, lang::t("Colores básicos"));
                            for row in BASIC_COLORS.chunks(8) {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                                    for c in row {
                                        let col = Color32::from_rgb(c[0], c[1], c[2]);
                                        if swatch(ui, &theme, Some(col), col == cur) {
                                            let h = ecolor::Hsva::from_srgb(*c);
                                            hsv = [h.h, h.s, h.v];
                                        }
                                    }
                                });
                                ui.add_space(2.0);
                            }

                            ui.add_space(12.0);
                            caption(ui, &theme, lang::t("Colores personalizados"));
                            for row in 0..2 {
                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(2.0, 2.0);
                                    for col in 0..8 {
                                        let c = self.custom_colors[row * 8 + col];
                                        if swatch(ui, &theme, c, c == Some(cur)) {
                                            if let Some(c) = c {
                                                let h =
                                                    ecolor::Hsva::from_srgb([c.r(), c.g(), c.b()]);
                                                hsv = [h.h, h.s, h.v];
                                            }
                                        }
                                    }
                                });
                                ui.add_space(2.0);
                            }

                            ui.add_space(12.0);
                            caption(ui, &theme, lang::t("Elegido"));
                            let (pr, _) = ui.allocate_exact_size(vec2(74.0, 52.0), Sense::hover());
                            ui.painter().rect_filled(pr, 0.0, cur);
                            ui.painter().rect_stroke(
                                pr,
                                0.0,
                                egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                                egui::StrokeKind::Inside,
                            );
                            // El hex no está en el diálogo de Windows, y es lo
                            // primero que uno quiere copiar.
                            ui.painter().text(
                                Pos2::new(pr.left(), pr.bottom() + 4.0),
                                egui::Align2::LEFT_TOP,
                                format!("#{:02X}{:02X}{:02X}", srgb[0], srgb[1], srgb[2]),
                                egui::FontId::monospace(theme.font_size - 0.5),
                                theme.text_dim.into(),
                            );
                            ui.add_space(18.0);
                        });

                        ui.add_space(16.0);

                        ui.vertical(|ui| {
                            caption(ui, &theme, lang::t("Matiz y saturación"));
                            ui.horizontal(|ui| {
                                let (rect, resp) = ui.allocate_exact_size(
                                    vec2(232.0, 184.0),
                                    Sense::click_and_drag(),
                                );
                                if let Some(t) = &hs {
                                    ui.painter().image(
                                        t.id(),
                                        rect,
                                        egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                }
                                ui.painter().rect_stroke(
                                    rect,
                                    0.0,
                                    egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                                    egui::StrokeKind::Inside,
                                );
                                if resp.dragged() || resp.clicked() {
                                    if let Some(p) = resp.interact_pointer_pos() {
                                        hsv[0] =
                                            ((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                                        hsv[1] = 1.0
                                            - ((p.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                                    }
                                }
                                let c = Pos2::new(
                                    rect.left() + hsv[0] * rect.width(),
                                    rect.top() + (1.0 - hsv[1]) * rect.height(),
                                );
                                // Cruz abierta al medio: tapada no dejaría ver
                                // el color que se está eligiendo.
                                let w = egui::Stroke::new(1.0, Color32::WHITE);
                                for dx in [-7.0f32, 3.0] {
                                    ui.painter().line_segment(
                                        [Pos2::new(c.x + dx, c.y), Pos2::new(c.x + dx + 4.0, c.y)],
                                        w,
                                    );
                                }
                                for dy in [-7.0f32, 3.0] {
                                    ui.painter().line_segment(
                                        [Pos2::new(c.x, c.y + dy), Pos2::new(c.x, c.y + dy + 4.0)],
                                        w,
                                    );
                                }

                                ui.add_space(7.0);
                                let (bar, bresp) = ui.allocate_exact_size(
                                    vec2(18.0, 184.0),
                                    Sense::click_and_drag(),
                                );
                                const BANDS: usize = 48;
                                for i in 0..BANDS {
                                    let v = 1.0 - i as f32 / (BANDS - 1) as f32;
                                    let c = ecolor::Hsva::new(hsv[0], hsv[1], v, 1.0).to_srgb();
                                    let y = bar.top() + i as f32 * bar.height() / BANDS as f32;
                                    ui.painter().rect_filled(
                                        egui::Rect::from_min_size(
                                            Pos2::new(bar.left(), y),
                                            vec2(bar.width(), bar.height() / BANDS as f32 + 1.0),
                                        ),
                                        0.0,
                                        Color32::from_rgb(c[0], c[1], c[2]),
                                    );
                                }
                                ui.painter().rect_stroke(
                                    bar,
                                    0.0,
                                    egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                                    egui::StrokeKind::Inside,
                                );
                                if bresp.dragged() || bresp.clicked() {
                                    if let Some(p) = bresp.interact_pointer_pos() {
                                        hsv[2] = 1.0
                                            - ((p.y - bar.top()) / bar.height()).clamp(0.0, 1.0);
                                    }
                                }
                                let py = bar.top() + (1.0 - hsv[2]) * bar.height();
                                ui.painter().add(egui::Shape::convex_polygon(
                                    vec![
                                        Pos2::new(bar.right() + 2.0, py),
                                        Pos2::new(bar.right() + 9.0, py - 5.0),
                                        Pos2::new(bar.right() + 9.0, py + 5.0),
                                    ],
                                    Color32::from(theme.text),
                                    egui::Stroke::NONE,
                                ));
                                ui.add_space(12.0);
                            });

                            // Los seis campos en dos columnas, cada etiqueta
                            // pegada a lo suyo. Sueltos no se leía cuál era cuál.
                            ui.add_space(12.0);
                            let mut rgb = [srgb[0] as f32, srgb[1] as f32, srgb[2] as f32];
                            let mut rgb_touched = false;
                            egui::Grid::new("campos_color")
                                .num_columns(4)
                                .spacing(vec2(8.0, 6.0))
                                .show(ui, |ui| {
                                    // Windows numera matiz, saturación y luz de
                                    // 0 a 240, no de 0 a 255. Se respeta.
                                    let hsl = ["Matiz", "Sat", "Lum"];
                                    let rgbl = ["Rojo", "Verde", "Azul"];
                                    for i in 0..3 {
                                        ui.label(hsl[i]);
                                        let mut n = (hsv[i] * 240.0).round();
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut n)
                                                    .range(0.0..=240.0)
                                                    .speed(1.0),
                                            )
                                            .changed()
                                        {
                                            hsv[i] = (n / 240.0).clamp(0.0, 1.0);
                                        }
                                        ui.label(rgbl[i]);
                                        rgb_touched |= ui
                                            .add(
                                                egui::DragValue::new(&mut rgb[i])
                                                    .range(0.0..=255.0)
                                                    .speed(1.0),
                                            )
                                            .changed();
                                        ui.end_row();
                                    }
                                });
                            if rgb_touched {
                                let h = ecolor::Hsva::from_srgb([
                                    rgb[0] as u8,
                                    rgb[1] as u8,
                                    rgb[2] as u8,
                                ]);
                                hsv = [h.h, h.s, h.v];
                            }

                            ui.add_space(10.0);
                            if ui
                                .add_sized(
                                    vec2(257.0, 26.0),
                                    egui::Button::new(lang::t("Guardar en personalizados")),
                                )
                                .clicked()
                            {
                                add_custom = true;
                            }
                        });
                        ui.add_space(12.0);
                    });

                    // --- pie ---
                    ui.add_space(10.0);
                    let (ft, _) = ui.allocate_exact_size(vec2(560.0, 48.0), Sense::hover());
                    ui.painter()
                        .rect_filled(ft, 0.0, Color32::from(theme.surface_alt));
                    ui.painter().line_segment(
                        [
                            Pos2::new(ft.left(), ft.top() + 0.5),
                            Pos2::new(ft.right(), ft.top() + 0.5),
                        ],
                        egui::Stroke::new(1.0, Color32::from(theme.border)),
                    );
                    // Aceptar en color pleno y a la derecha: antes los dos
                    // botones eran iguales y no se sabía cuál era la acción.
                    let ok = egui::Rect::from_min_size(
                        Pos2::new(ft.right() - 12.0 - 84.0, ft.center().y - 14.0),
                        vec2(84.0, 28.0),
                    );
                    let cancel = egui::Rect::from_min_size(
                        Pos2::new(ok.left() - 8.0 - 84.0, ft.center().y - 14.0),
                        vec2(84.0, 28.0),
                    );
                    for (r, label, primary) in [(ok, "Aceptar", true), (cancel, "Cancelar", false)]
                    {
                        let resp = ui.interact(r, ui.id().with(label), Sense::click());
                        let (bg, fg) = if primary {
                            (
                                Color32::from(theme.accent),
                                Color32::from(theme.accent_text),
                            )
                        } else if resp.hovered() {
                            (Color32::from(theme.button_hover), Color32::from(theme.text))
                        } else {
                            (Color32::from(theme.surface), Color32::from(theme.text))
                        };
                        ui.painter().rect_filled(r, theme.button_rounding, bg);
                        ui.painter().rect_stroke(
                            r,
                            theme.button_rounding,
                            egui::Stroke::new(
                                1.0,
                                if primary {
                                    Color32::from(theme.accent)
                                } else {
                                    Color32::from(theme.border)
                                },
                            ),
                            egui::StrokeKind::Inside,
                        );
                        ui.painter().text(
                            r.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(theme.font_size),
                            fg,
                        );
                        if resp.clicked() {
                            accept = primary;
                            color = false;
                        }
                    }
                });

            self.dialogs.hsv = hsv;
            let srgb = ecolor::Hsva::new(hsv[0], hsv[1], hsv[2], 1.0).to_srgb();
            let cur = Color32::from_rgb(srgb[0], srgb[1], srgb[2]);
            if add_custom && !self.custom_colors.contains(&Some(cur)) {
                match self.custom_colors.iter().position(|c| c.is_none()) {
                    Some(slot) => self.custom_colors[slot] = Some(cur),
                    // Llenos: corren todos una posición y el nuevo entra al
                    // final. Antes pisaba el primero, que es arbitrario — así
                    // sale siempre el más viejo y se ve cómo se hace el lugar.
                    None => {
                        self.custom_colors.rotate_left(1);
                        self.custom_colors[15] = Some(cur);
                    }
                }
            }
            if accept {
                if self.picking_c1 {
                    self.doc.color1 = cur;
                } else {
                    self.doc.color2 = cur;
                }
            }
        }
        self.dialogs.color = color;
        let mut fm = self.dialogs.file_menu;
        if fm {
            let mut pane = self.pane;
            let recent = self.recent.clone();
            let (cw, ch) = (self.doc.canvas.w, self.doc.canvas.h);
            let mut scale = self.export_scale;
            let mut new_size: Option<(usize, usize)> = None;
            let mut new_lang: Option<usize> = None;
            let mut switch_mode: Option<bool> = None;

            egui::Window::new("archivo_backstage")
                // Pegado bajo la pestaña Archivo, como en Paint.
                .anchor(egui::Align2::LEFT_TOP, [0.0, 50.0])
                .constrain(true)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(
                    egui::Frame::NONE
                        .fill(Color32::from(theme.surface))
                        .stroke(egui::Stroke::new(1.0, Color32::from(theme.border_strong)))
                        .shadow(egui::Shadow {
                            offset: [0, 10],
                            blur: 32,
                            spread: 0,
                            color: Color32::from_black_alpha(60),
                        }),
                )
                .show(ctx, |ui| {
                    ui.set_width(660.0);

                    // Cabecera del color de la pestaña Archivo.
                    let (hd, _) = ui.allocate_exact_size(vec2(660.0, 34.0), Sense::hover());
                    ui.painter()
                        .rect_filled(hd, 0.0, Color32::from(theme.file_tab));
                    ui.painter().text(
                        Pos2::new(hd.left() + 12.0, hd.center().y),
                        egui::Align2::LEFT_CENTER,
                        "Archivo",
                        egui::FontId::proportional(theme.font_size + 0.5),
                        theme.file_tab_text.into(),
                    );
                    let xr = egui::Rect::from_center_size(
                        Pos2::new(hd.right() - 19.0, hd.center().y),
                        vec2(26.0, 26.0),
                    );
                    let xresp = ui.interact(xr, ui.id().with("cerrar_archivo"), Sense::click());
                    let st = egui::Stroke::new(1.3, Color32::from(theme.file_tab_text));
                    ui.painter().line_segment(
                        [xr.center() + vec2(-4.5, -4.5), xr.center() + vec2(4.5, 4.5)],
                        st,
                    );
                    ui.painter().line_segment(
                        [xr.center() + vec2(4.5, -4.5), xr.center() + vec2(-4.5, 4.5)],
                        st,
                    );
                    if xresp.clicked() {
                        fm = false;
                    }

                    ui.horizontal_top(|ui| {
                        // --- columna de comandos ---
                        ui.allocate_ui_with_layout(
                            vec2(250.0, 404.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add_space(6.0);

                                let mut item = |ui: &mut egui::Ui,
                                                icon: Icon,
                                                label: &str,
                                                sub: Option<Pane>,
                                                enabled: bool|
                                 -> bool {
                                    let (rect, resp) =
                                        ui.allocate_exact_size(vec2(250.0, 34.0), Sense::click());
                                    let active = sub.is_some() && sub == Some(pane);
                                    if active {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            Color32::from(theme.button_active),
                                        );
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(rect.min, vec2(2.0, 34.0)),
                                            0.0,
                                            Color32::from(theme.accent),
                                        );
                                    } else if resp.hovered() && enabled {
                                        ui.painter().rect_filled(
                                            rect,
                                            0.0,
                                            Color32::from(theme.button_hover),
                                        );
                                    }
                                    // Pasar por encima ya cambia el panel: es lo
                                    // que hace que la mitad derecha sirva.
                                    if resp.hovered() {
                                        if let Some(pn) = sub {
                                            pane = pn;
                                        }
                                    }
                                    let col: Color32 = if enabled {
                                        theme.icon.into()
                                    } else {
                                        theme.text_dim.into()
                                    };
                                    ui::draw_icon(
                                        ui,
                                        egui::Rect::from_center_size(
                                            Pos2::new(rect.left() + 21.0, rect.center().y),
                                            vec2(18.0, 18.0),
                                        ),
                                        icon,
                                        col,
                                    );
                                    ui.painter().text(
                                        Pos2::new(rect.left() + 41.0, rect.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        label,
                                        egui::FontId::proportional(theme.font_size + 0.5),
                                        if enabled { theme.text.into() } else { col },
                                    );
                                    if sub.is_some() {
                                        ui.painter().text(
                                            Pos2::new(rect.right() - 14.0, rect.center().y),
                                            egui::Align2::CENTER_CENTER,
                                            "›",
                                            egui::FontId::proportional(theme.font_size + 2.0),
                                            theme.text_dim.into(),
                                        );
                                    }
                                    enabled && resp.clicked()
                                };

                                let sep = |ui: &mut egui::Ui| {
                                    let (r, _) =
                                        ui.allocate_exact_size(vec2(250.0, 13.0), Sense::hover());
                                    ui.painter().line_segment(
                                        [
                                            Pos2::new(r.left() + 12.0, r.center().y),
                                            Pos2::new(r.right() - 12.0, r.center().y),
                                        ],
                                        egui::Stroke::new(1.0, Color32::from(theme.border)),
                                    );
                                };

                                if item(
                                    ui,
                                    Icon::I(Ico::New),
                                    lang::t("Nuevo"),
                                    Some(Pane::New),
                                    true,
                                ) {
                                    pending.push(Cmd::New);
                                    fm = false;
                                }
                                if item(ui, Icon::I(Ico::Open), lang::t("Abrir…"), None, true) {
                                    pending.push(Cmd::Open);
                                    fm = false;
                                }
                                item(
                                    ui,
                                    Icon::I(Ico::Clock),
                                    lang::t("Recientes"),
                                    Some(Pane::Recent),
                                    true,
                                );

                                sep(ui);

                                if item(ui, Icon::I(Ico::Save), lang::t("Guardar"), None, true) {
                                    pending.push(Cmd::Save);
                                    fm = false;
                                }
                                if item(
                                    ui,
                                    Icon::I(Ico::Save),
                                    lang::t("Guardar como…"),
                                    None,
                                    true,
                                ) {
                                    pending.push(Cmd::SaveAs);
                                    fm = false;
                                }
                                item(
                                    ui,
                                    Icon::I(Ico::Export),
                                    lang::t("Exportar"),
                                    Some(Pane::Export),
                                    true,
                                );

                                sep(ui);

                                if item(
                                    ui,
                                    Icon::I(Ico::Copy),
                                    lang::t("Copiar imagen"),
                                    None,
                                    true,
                                ) {
                                    pending.push(Cmd::CopyImage);
                                    fm = false;
                                }
                                let has_path = self.path.is_some();
                                if item(
                                    ui,
                                    Icon::I(Ico::Reveal),
                                    lang::t("Mostrar en el explorador"),
                                    None,
                                    has_path,
                                ) {
                                    pending.push(Cmd::Reveal);
                                    fm = false;
                                }

                                sep(ui);

                                item(
                                    ui,
                                    Icon::I(Ico::Settings),
                                    lang::t("Configuración"),
                                    Some(Pane::Settings),
                                    true,
                                );

                                if item(ui, Icon::I(Ico::Info), lang::t("Propiedades…"), None, true)
                                {
                                    pending.push(Cmd::PropertiesDialog);
                                    fm = false;
                                }
                                if item(
                                    ui,
                                    Icon::I(Ico::Palette),
                                    lang::t("Acerca de Lienzo"),
                                    None,
                                    true,
                                ) {
                                    pending.push(Cmd::About);
                                    fm = false;
                                }
                                if item(ui, Icon::I(Ico::Exit), lang::t("Salir"), None, true) {
                                    pending.push(Cmd::Exit);
                                }
                            },
                        );

                        // --- panel derecho, contextual ---
                        let (pr, _) = ui.allocate_exact_size(vec2(410.0, 404.0), Sense::hover());
                        ui.painter()
                            .rect_filled(pr, 0.0, Color32::from(theme.surface_alt));
                        ui.painter().line_segment(
                            [
                                Pos2::new(pr.left() + 0.5, pr.top()),
                                Pos2::new(pr.left() + 0.5, pr.bottom()),
                            ],
                            egui::Stroke::new(1.0, Color32::from(theme.border)),
                        );

                        let mut inner = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(pr.shrink2(vec2(16.0, 14.0)))
                                .layout(egui::Layout::top_down(egui::Align::Min)),
                        );
                        let ui = &mut inner;

                        match pane {
                            Pane::Settings => {
                                // --- claro / oscuro ---
                                caption(ui, &theme, lang::t("Apariencia"));
                                ui.horizontal(|ui| {
                                    for (label, want_dark) in
                                        [(lang::t("Claro"), false), (lang::t("Oscuro"), true)]
                                    {
                                        let (r, resp) = ui
                                            .allocate_exact_size(vec2(182.0, 40.0), Sense::click());
                                        let on = theme.dark == want_dark;
                                        ui.painter().rect_filled(
                                            r,
                                            3.0,
                                            Color32::from(if on {
                                                theme.accent_soft
                                            } else {
                                                theme.surface
                                            }),
                                        );
                                        ui.painter().rect_stroke(
                                            r,
                                            3.0,
                                            egui::Stroke::new(
                                                1.0,
                                                Color32::from(if on {
                                                    theme.accent
                                                } else {
                                                    theme.border
                                                }),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                        // Una muestra del papel y la tinta de
                                        // ese modo, para no leer sólo la palabra.
                                        let sw = egui::Rect::from_center_size(
                                            Pos2::new(r.left() + 24.0, r.center().y),
                                            vec2(22.0, 22.0),
                                        );
                                        let (paper, ink) = if want_dark {
                                            (
                                                Color32::from_rgb(0x2b, 0x2b, 0x2b),
                                                Color32::from_rgb(0xe8, 0xe8, 0xe8),
                                            )
                                        } else {
                                            (Color32::WHITE, Color32::from_rgb(0x1a, 0x1a, 0x1a))
                                        };
                                        ui.painter().rect_filled(sw, 2.0, paper);
                                        ui.painter().rect_stroke(
                                            sw,
                                            2.0,
                                            egui::Stroke::new(
                                                1.0,
                                                Color32::from(theme.border_strong),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                        ui.painter().line_segment(
                                            [
                                                Pos2::new(sw.left() + 5.0, sw.center().y),
                                                Pos2::new(sw.right() - 5.0, sw.center().y),
                                            ],
                                            egui::Stroke::new(2.0, ink),
                                        );
                                        ui.painter().text(
                                            Pos2::new(r.left() + 46.0, r.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            label,
                                            egui::FontId::proportional(theme.font_size + 0.5),
                                            theme.text.into(),
                                        );
                                        if resp.clicked() && !on {
                                            switch_mode = Some(want_dark);
                                        }
                                    }
                                });

                                ui.add_space(16.0);

                                // --- idioma ---
                                caption(ui, &theme, lang::t("Idioma"));
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                    for (i, (_, native)) in lang::LANGS.iter().enumerate() {
                                        let (r, resp) = ui
                                            .allocate_exact_size(vec2(88.0, 30.0), Sense::click());
                                        let on = i == self.lang_idx;
                                        ui.painter().rect_filled(
                                            r,
                                            3.0,
                                            Color32::from(if on {
                                                theme.accent_soft
                                            } else {
                                                theme.surface
                                            }),
                                        );
                                        ui.painter().rect_stroke(
                                            r,
                                            3.0,
                                            egui::Stroke::new(
                                                1.0,
                                                Color32::from(if on {
                                                    theme.accent
                                                } else {
                                                    theme.border
                                                }),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                        ui.painter().text(
                                            r.center(),
                                            egui::Align2::CENTER_CENTER,
                                            *native,
                                            egui::FontId::proportional(theme.font_size + 0.5),
                                            theme.text.into(),
                                        );
                                        if resp.clicked() {
                                            new_lang = Some(i);
                                        }
                                    }
                                });

                                ui.add_space(16.0);

                                // --- temas ---
                                caption(ui, &theme, lang::t("Temas"));
                                // Una fila por familia, armada con la pareja
                                // declarada. Recortar « oscuro» del nombre no
                                // sirve: Lienzo y Lienzo Tinta son pareja y no
                                // comparten ni una palabra.
                                let names = theme::families(&self.themes, theme.dark);
                                let aqui = self.theme_idx;
                                let here = names
                                    .iter()
                                    .find(|(_, i)| {
                                        *i == aqui
                                            || self.themes[aqui].pair.as_deref()
                                                == Some(self.themes[*i].name.as_str())
                                    })
                                    .map(|(n, _)| n.clone())
                                    .unwrap_or_default();
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                                    for (name, i) in names.iter() {
                                        let i = *i;
                                        let (r, resp) = ui
                                            .allocate_exact_size(vec2(182.0, 32.0), Sense::click());
                                        let on = *name == here;
                                        ui.painter().rect_filled(
                                            r,
                                            3.0,
                                            Color32::from(if on {
                                                theme.accent_soft
                                            } else {
                                                theme.surface
                                            }),
                                        );
                                        ui.painter().rect_stroke(
                                            r,
                                            3.0,
                                            egui::Stroke::new(
                                                1.0,
                                                Color32::from(if on {
                                                    theme.accent
                                                } else {
                                                    theme.border
                                                }),
                                            ),
                                            egui::StrokeKind::Inside,
                                        );
                                        // Punto del acento de *ese* tema: la
                                        // lista se lee de un vistazo sin probarlos.
                                        let dot = Pos2::new(r.left() + 15.0, r.center().y);
                                        ui.painter().circle_filled(
                                            dot,
                                            6.0,
                                            Color32::from(self.themes[i].accent),
                                        );
                                        ui.painter().text(
                                            Pos2::new(r.left() + 30.0, r.center().y),
                                            egui::Align2::LEFT_CENTER,
                                            name,
                                            egui::FontId::proportional(theme.font_size + 0.5),
                                            theme.text.into(),
                                        );
                                        if resp.clicked() {
                                            pending.push(Cmd::SetTheme(i));
                                        }
                                    }
                                });
                            }
                            Pane::New => {
                                caption(ui, &theme, lang::t("Tamaño del lienzo"));
                                let all: Vec<(String, usize, usize)> =
                                    std::iter::once(("Igual que ahora".to_string(), cw, ch))
                                        .chain(
                                            PRESETS.iter().map(|(n, w, h)| (n.to_string(), *w, *h)),
                                        )
                                        .collect();
                                for row in all.chunks(2) {
                                    ui.horizontal(|ui| {
                                        for (name, w, h) in row {
                                            let (r, resp) = ui.allocate_exact_size(
                                                vec2(182.0, 42.0),
                                                Sense::click(),
                                            );
                                            let on = *w == cw && *h == ch;
                                            ui.painter().rect_filled(
                                                r,
                                                3.0,
                                                Color32::from(theme.surface),
                                            );
                                            ui.painter().rect_stroke(
                                                r,
                                                3.0,
                                                egui::Stroke::new(
                                                    1.0,
                                                    if on || resp.hovered() {
                                                        Color32::from(theme.accent)
                                                    } else {
                                                        Color32::from(theme.border)
                                                    },
                                                ),
                                                egui::StrokeKind::Inside,
                                            );
                                            ui.painter().text(
                                                Pos2::new(r.left() + 10.0, r.top() + 12.0),
                                                egui::Align2::LEFT_CENTER,
                                                name,
                                                egui::FontId::proportional(theme.font_size),
                                                theme.text.into(),
                                            );
                                            ui.painter().text(
                                                Pos2::new(r.left() + 10.0, r.bottom() - 12.0),
                                                egui::Align2::LEFT_CENTER,
                                                format!("{w} × {h}"),
                                                egui::FontId::proportional(theme.font_size - 1.0),
                                                theme.text_dim.into(),
                                            );
                                            if resp.clicked() {
                                                new_size = Some((*w, *h));
                                            }
                                        }
                                    });
                                    ui.add_space(6.0);
                                }
                            }

                            Pane::Recent => {
                                caption(ui, &theme, lang::t("Archivos recientes"));
                                if recent.is_empty() {
                                    ui.label(
                                        egui::RichText::new(
                                            "Todavía no abriste ni guardaste ningún dibujo.",
                                        )
                                        .size(theme.font_size)
                                        .color(Color32::from(theme.text_dim)),
                                    );
                                }
                                for (i, path) in recent.iter().enumerate() {
                                    let name = path
                                        .file_name()
                                        .map(|n| n.to_string_lossy().to_string())
                                        .unwrap_or_default();
                                    let (r, resp) =
                                        ui.allocate_exact_size(vec2(378.0, 30.0), Sense::click());
                                    if resp.hovered() {
                                        ui.painter().rect_filled(
                                            r,
                                            2.0,
                                            Color32::from(theme.button_hover),
                                        );
                                    }
                                    ui.painter().text(
                                        Pos2::new(r.left() + 8.0, r.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        format!("{}", i + 1),
                                        egui::FontId::proportional(theme.font_size - 1.0),
                                        theme.text_dim.into(),
                                    );
                                    ui.painter().text(
                                        Pos2::new(r.left() + 26.0, r.center().y),
                                        egui::Align2::LEFT_CENTER,
                                        name,
                                        egui::FontId::proportional(theme.font_size),
                                        theme.text.into(),
                                    );
                                    if resp.clicked() {
                                        pending.push(Cmd::OpenRecent(i));
                                        fm = false;
                                    }
                                }
                            }

                            Pane::Export => {
                                caption(ui, &theme, lang::t("Escala de exportación"));
                                ui.horizontal(|ui| {
                                    for p in [50u32, 100, 200, 400] {
                                        if ui
                                            .selectable_label(scale == p, format!("{p} %"))
                                            .clicked()
                                        {
                                            scale = p;
                                        }
                                    }
                                });
                                ui.add_space(8.0);
                                let (w, h) = (cw * scale as usize / 100, ch * scale as usize / 100);
                                ui.label(
                                    egui::RichText::new(format!("Saldrá en {w} × {h} px"))
                                        .size(theme.font_size)
                                        .color(Color32::from(theme.text_dim)),
                                );
                                ui.add_space(12.0);
                                // Guardar como sólo cambia el formato. Esto saca
                                // una copia a otra escala sin tocar el original.
                                if ui
                                    .add_sized(
                                        vec2(180.0, 30.0),
                                        egui::Button::new(lang::t("Exportar una copia…")),
                                    )
                                    .clicked()
                                {
                                    pending.push(Cmd::Export(scale));
                                    fm = false;
                                }
                            }
                        }
                    });
                });

            self.pane = pane;
            self.export_scale = scale;
            if let Some((w, h)) = new_size {
                pending.push(Cmd::NewSized(w, h));
                fm = false;
            }
            if let Some(i) = new_lang {
                self.lang_idx = i;
                lang::set(i);
            }
            // Claro/oscuro salta al tema pareja del que está puesto. Si ese tema
            // no declaró pareja, se cae al primero del modo pedido: es preferible
            // a que el interruptor no haga nada.
            // Claro/oscuro salta a la pareja **del tema puesto**, y a ninguna
            // otra. Antes, si el tema no declaraba pareja, caía en el primer
            // tema oscuro de la lista y te sacaba del estilo que elegiste.
            if let Some(want_dark) = switch_mode {
                let par = self.theme().pair.clone();
                if let Some(i) = par.and_then(|n| {
                    self.themes
                        .iter()
                        .position(|t| t.name == n && t.dark == want_dark)
                }) {
                    pending.push(Cmd::SetTheme(i));
                }
            }
        }
        self.dialogs.file_menu = fm;

        let mut about = self.dialogs.about;
        if about {
            const W: f32 = 360.0;
            egui::Window::new("about_dlg")
                // Sin esto egui los deja donde quedaron la última vez, y salían
                // fuera de la ventana.
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .constrain(true)
                .title_bar(false)
                .collapsible(false)
                .resizable(false)
                .frame(ui::dialog_frame(&theme))
                .show(ctx, |ui| {
                    ui.set_width(W);
                    if ui::dialog_header(ui, &theme, W, lang::t("Acerca de Lienzo")) {
                        about = false;
                    }

                    ui.add_space(20.0);
                    ui.vertical_centered(|ui| {
                        let (r, _) = ui.allocate_exact_size(vec2(56.0, 56.0), Sense::hover());
                        ui::app_icon(ui, r, theme.icon.into());
                        ui.add_space(10.0);
                        ui.label(
                            egui::RichText::new("Lienzo")
                                .size(theme.font_size + 6.0)
                                .color(Color32::from(theme.text)),
                        );
                        ui.label(
                            egui::RichText::new(format!("Versión {}", env!("CARGO_PKG_VERSION")))
                                .size(theme.font_size)
                                .color(Color32::from(theme.text_dim)),
                        );
                    });

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.add_space(18.0);
                        ui.vertical(|ui| {
                            let iw = W - 36.0;
                            // Salen de Cargo.toml, no escritos acá: así el
                            // diálogo no puede desfasarse del paquete.
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Autor"),
                                env!("CARGO_PKG_AUTHORS"),
                            );
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Repositorio"),
                                env!("CARGO_PKG_REPOSITORY").trim_start_matches("https://"),
                            );
                            ui::info_row(ui, &theme, iw, lang::t("Licencia"), "MIT");
                            ui::info_row(ui, &theme, iw, lang::t("Tema"), &theme.name);
                            ui::info_row(
                                ui,
                                &theme,
                                iw,
                                lang::t("Temas instalados"),
                                &self.themes.len().to_string(),
                            );
                            ui::info_row(ui, &theme, iw, lang::t("Interfaz"), "Rust y egui");
                        });
                        ui.add_space(18.0);
                    });

                    ui.add_space(16.0);
                    if ui::dialog_footer(ui, &theme, W, "Cerrar", None).is_some() {
                        about = false;
                    }
                });
        }
        self.dialogs.about = about;

        pending
    }

    fn unsaved_dialog(&mut self, ctx: &egui::Context) {
        const W: f32 = 360.0;
        let Some(action) = self.pending_action.clone() else {
            return;
        };
        let theme = self.theme().clone();
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Sin título".into());
        let mut choice = None;

        egui::Modal::new(egui::Id::new("cambios_sin_guardar"))
            .frame(ui::dialog_frame(&theme))
            .show(ctx, |ui| {
                ui.set_width(W);
                if ui::dialog_header(ui, &theme, W, "Cambios sin guardar") {
                    choice = Some(2);
                }

                ui.add_space(18.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width(W - 32.0);
                        ui.label(format!("¿Querés guardar los cambios de «{name}»?"));
                    });
                    ui.add_space(16.0);
                });
                ui.add_space(18.0);

                if let Some(i) = ui::dialog_footer_buttons(
                    ui,
                    &theme,
                    W,
                    &[
                        (lang::t("Guardar"), true),
                        ("No guardar", false),
                        (lang::t("Cancelar"), false),
                    ],
                ) {
                    choice = Some(i);
                }
            });

        match choice {
            Some(0) if self.save_current(ctx) => {
                self.pending_action = None;
                self.perform_action(action, ctx);
            }
            Some(1) => {
                self.pending_action = None;
                self.perform_action(action, ctx);
            }
            Some(2) => {
                self.pending_action = None;
                self.allow_close = false;
            }
            _ => {}
        }
    }

    fn thumbnail(&mut self, ctx: &egui::Context) {
        if !self.show_thumbnail {
            return;
        }
        let theme = self.theme().clone();
        let Some(tex) = self.tex.as_ref() else { return };
        let id = tex.id();
        let (w, h) = (self.doc.canvas.w as f32, self.doc.canvas.h as f32);
        const W: f32 = 272.0;
        const PW: f32 = 240.0;
        const PH: f32 = 180.0;
        let scale = (PW / w).min(PH / h).min(1.0);
        let image_size = vec2((w * scale).max(1.0), (h * scale).max(1.0));
        let mut close = false;

        egui::Window::new("thumbnail_panel")
            .default_pos(Pos2::new(
                ctx.content_rect().right() - W - 18.0,
                ctx.content_rect().top() + 132.0,
            ))
            .constrain(true)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .frame(ui::dialog_frame(&theme))
            .show(ctx, |ui| {
                ui.set_width(W);
                if ui::dialog_header(ui, &theme, W, lang::t("Miniatura")) {
                    close = true;
                }

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    ui.add_space(16.0);
                    let (well, _) = ui.allocate_exact_size(vec2(PW, PH), Sense::hover());
                    ui.painter()
                        .rect_filled(well, 1.0, Color32::from(theme.workspace));
                    ui.painter().rect_stroke(
                        well,
                        1.0,
                        egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                        egui::StrokeKind::Inside,
                    );
                    let rect = egui::Rect::from_center_size(well.center(), image_size);
                    ui.painter().image(
                        id,
                        rect,
                        egui::Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(1.0, Color32::from(theme.border)),
                        egui::StrokeKind::Inside,
                    );
                    ui.add_space(16.0);
                });

                ui.add_space(8.0);
                let (footer, _) = ui.allocate_exact_size(vec2(W, 28.0), Sense::hover());
                ui.painter()
                    .rect_filled(footer, 0.0, Color32::from(theme.surface_alt));
                ui.painter().line_segment(
                    [footer.left_top(), footer.right_top()],
                    egui::Stroke::new(1.0, Color32::from(theme.border)),
                );
                ui.painter().text(
                    Pos2::new(footer.left() + 12.0, footer.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{} × {} px", self.doc.canvas.w, self.doc.canvas.h),
                    egui::FontId::proportional(theme.font_size - 0.5),
                    Color32::from(theme.text_dim),
                );
                ui.painter().text(
                    Pos2::new(footer.right() - 12.0, footer.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}%", (self.zoom() * 100.0).round() as i32),
                    egui::FontId::proportional(theme.font_size - 0.5),
                    Color32::from(theme.text),
                );
            });
        if close {
            self.show_thumbnail = false;
        }
    }

    /// El cuadro de texto flotante. Va sobre un `TextEdit` de verdad y no sobre
    /// eventos crudos: egui sólo activa el IME cuando un `TextEdit` con foco lo
    /// pide, y sin IME macOS **no entrega ningún evento de tecla muerta**, así
    /// que las tildes dejarían de funcionar.
    /// El cuadro sobre el lienzo: el texto en vivo, el marco de hormigas y las
    /// ocho manijas.
    ///
    /// El campo de edición es un `TextEdit` de egui **sin marco y sin fondo**,
    /// no un control gris flotando: así se ve el dibujo debajo mientras
    /// escribís, y de paso heredamos cursor, selección, flechas, Ctrl+A y —lo
    /// que más importa acá— las teclas muertas de los acentos y la ñ.
    fn text_overlay(&mut self, ctx: &egui::Context, origin: egui::Rect, zoom: f32) {
        let Some(mut tb) = self.text_box.take() else {
            return;
        };
        let theme = self.theme().clone();
        let scr = |x: f32, y: f32| Pos2::new(origin.left() + x * zoom, origin.top() + y * zoom);
        let r = egui::Rect::from_min_max(scr(tb.x, tb.y), scr(tb.x + tb.w, tb.y + tb.h));

        let ink = self.doc.color1;
        let paper = self.doc.color2;

        egui::Area::new(egui::Id::new("cuadro_texto"))
            .order(egui::Order::Foreground)
            .fixed_pos(r.min)
            .show(ctx, |ui| {
                if tb.opaque {
                    ui.painter().rect_filled(r, 0.0, paper);
                }
                ui.set_max_width(r.width());
                // La fuente se resuelve antes: `multiline` toma prestado el
                // texto, y `tb` no puede estar prestado de las dos formas.
                let font = text::font_id(&tb, ctx, zoom);
                let resp = ui.add(
                    egui::TextEdit::multiline(&mut tb.s)
                        // Sin marco ni relleno: el fondo lo pintamos nosotros
                        // sólo cuando el cuadro es opaco, así el dibujo se ve
                        // debajo mientras escribís.
                        .frame(egui::Frame::NONE)
                        .margin(egui::Margin::ZERO)
                        .desired_width(r.width())
                        .desired_rows(1)
                        .font(font)
                        .text_color(ink),
                );
                resp.request_focus();
            });

        // Marco y manijas van en su propia capa, encima del campo.
        let p = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            egui::Id::new("marco_texto"),
        ));
        // Hormigas en marcha: el guion corre con el reloj, así que el borde se
        // distingue de cualquier cosa que haya pintada debajo.
        let t = ctx.input(|i| i.time) as f32;
        let offset = (t * 16.0) % 8.0;
        let s_ants = egui::Stroke::new(1.0, Color32::from(theme.text));
        let corners = [
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
        ];
        for i in 0..4 {
            p.extend(egui::Shape::dashed_line_with_offset(
                &[corners[i], corners[(i + 1) % 4]],
                s_ants,
                &[4.0],
                &[4.0],
                offset,
            ));
        }
        ctx.request_repaint();

        for (hx, hy) in T_HANDLES {
            let c = Pos2::new(r.left() + hx * r.width(), r.top() + hy * r.height());
            let hr = egui::Rect::from_center_size(c, vec2(7.0, 7.0));
            p.rect_filled(hr, 0.0, Color32::WHITE);
            p.rect_stroke(
                hr,
                0.0,
                egui::Stroke::new(1.0, Color32::from(theme.border_strong)),
                egui::StrokeKind::Inside,
            );
        }

        self.text_box = Some(tb);
    }
}

/// Las ocho manijas del cuadro de texto, en coordenadas de la caja: 0 es un
/// borde y 1 el opuesto. Con 0.5 ese eje no se mueve, así que redimensionar es
/// una sola cuenta para las ocho en vez de ocho casos distintos.
const T_HANDLES: [(f32, f32); 8] = [
    (0.0, 0.0),
    (0.5, 0.0),
    (1.0, 0.0),
    (1.0, 0.5),
    (1.0, 1.0),
    (0.5, 1.0),
    (0.0, 1.0),
    (0.0, 0.5),
];

/// Qué se está arrastrando del cuadro de texto.
#[derive(Clone, Copy, PartialEq, Debug)]
enum TGrab {
    /// Dibujando uno nuevo.
    New,
    /// Moviéndolo entero por el borde.
    Move,
    /// Estirándolo por una de las ocho manijas.
    Handle(usize),
}

impl eframe::App for App {
    /// eframe lo llama cada tanto y al cerrar.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        let c = |v: Color32| [v.r(), v.g(), v.b()];
        eframe::set_value(
            storage,
            AJUSTES,
            &Ajustes {
                tema: self.theme().name.clone(),
                idioma: lang::LANGS[self.lang_idx].0.to_string(),
                color1: c(self.doc.color1),
                color2: c(self.doc.color2),
                personalizados: self.custom_colors.iter().map(|o| o.map(c)).collect(),
            },
        );
    }

    /// El de fábrica es `(12, 12, 12, 180)` — casi negro. Con un tema claro,
    /// cualquier zona que no cubra un panel se ve como una banda negra.
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        Color32::from(self.theme().window).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.has_unsaved_changes()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.dialogs.close_all();
            self.pending_action = Some(PendingAction::Exit);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.title()));

        for c in self.keyboard(&ctx) {
            self.apply(c, &ctx);
        }

        // Cambiar de herramienta con el cuadro abierto lo confirma. Si no, la
        // caja se quedaría flotando y la pestaña contextual también, mostrando
        // controles de fuente mientras dibujás con el lápiz.
        if self.text_box.is_some() && self.doc.tool != Tool::Text {
            self.commit_text(&ctx);
        }

        self.sync_texture(&ctx);
        self.sync_sel_texture(&ctx);

        // Los necesita la barra de estado, que ahora va antes que todo.
        let zoom = self.zoom();
        let theme = self.theme().clone();

        // La barra de estado se arma antes que el resto: en egui el primer
        // panel de abajo es el que queda pegado al borde de la ventana. Yendo
        // después, la paleta de XP le quedaba *debajo*, al revés que en Paint.
        if self.show_status {
            egui::Panel::bottom("estado")
                .exact_size(28.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.set_min_height(22.0);
                        ui.add_space(4.0);
                        ui.label(&self.status);
                        if let Some(s) = &self.doc.sel {
                            ui.separator();
                            ui.label(format!("{} × {} px", s.r.w, s.r.h));
                        }
                        // El grosor, a mano y con **todos** los valores: arriba la
                        // cinta ofrece cuatro y nada más. Repetir esos cuatro acá
                        // abajo no agregaría nada; el motor acepta cualquier ancho.
                        //
                        // Siempre a la vista, apagado con las herramientas que no
                        // llevan grosor —bote, cuentagotas, lupa, selección—, igual
                        // que Paint apaga su botón Tamaño. Escondiéndolo del todo no
                        // se encuentra nunca: quien tomó el bote no sabe que existe.
                        //
                        // En XP ya vive en la caja de opciones y en SW en el lector,
                        // así que ahí sería el tercer sitio para lo mismo.
                        if !matches!(theme.chrome, theme::Chrome::Palette | theme::Chrome::Holo) {
                            let usa = matches!(
                                self.doc.tool,
                                Tool::Pencil | Tool::Brush | Tool::Eraser | Tool::Shape
                            );
                            ui.separator();
                            ui.add_enabled(usa, egui::Label::new(lang::t("Grosor")));
                            // La muestra al ancho real, topada al alto de la barra.
                            let (r, _) = ui.allocate_exact_size(vec2(30.0, 20.0), Sense::hover());
                            ui.painter().line_segment(
                                [
                                    Pos2::new(r.left() + 5.0, r.center().y),
                                    Pos2::new(r.right() - 5.0, r.center().y),
                                ],
                                egui::Stroke::new(
                                    self.doc.width.min(16.0),
                                    Color32::from(if usa { theme.text } else { theme.text_dim }),
                                ),
                            );
                            // La goma llega más lejos: borrar de a 50 px es lento.
                            let tope = if self.doc.tool == Tool::Eraser {
                                100.0
                            } else {
                                50.0
                            };
                            ui.add_enabled_ui(usa, |ui| {
                                ui.add_sized(
                                    vec2(128.0, 18.0),
                                    egui::Slider::new(&mut self.doc.width, 1.0..=tope)
                                        .step_by(1.0)
                                        .suffix(" px")
                                        .fixed_decimals(0),
                                );
                            });
                        }
                        // En el chrome de macOS la derecha de la barra queda vacía:
                        // el tamaño ya está en el título y el zoom en la barra de
                        // herramientas, y repetirlos abajo es decirlos dos veces.
                        if theme.chrome == theme::Chrome::Mac {
                            return;
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(4.0);
                            if ui.small_button("+").clicked() {
                                self.zoom_idx = (self.zoom_idx + 1).min(ZOOMS.len() - 1);
                            }
                            // Deslizador sobre el índice, no sobre el porcentaje:
                            // los once niveles no son lineales.
                            let mut idx = self.zoom_idx as f32;
                            if ui
                                .add_sized(
                                    vec2(110.0, 18.0),
                                    egui::Slider::new(&mut idx, 0.0..=(ZOOMS.len() - 1) as f32)
                                        .show_value(false),
                                )
                                .changed()
                            {
                                self.zoom_idx = idx.round() as usize;
                            }
                            if ui.small_button("−").clicked() {
                                self.zoom_idx = self.zoom_idx.saturating_sub(1);
                            }
                            ui.label(format!("{}%", (zoom * 100.0).round() as i32));
                            ui.separator();
                            // Lo que va a pesar el PNG, aproximado por los píxeles
                            // crudos: da la escala sin tener que comprimir nada.
                            let bytes = self.doc.canvas.w * self.doc.canvas.h * 4;
                            ui.label(if bytes >= 1 << 20 {
                                format!("{:.1} MB", bytes as f32 / (1 << 20) as f32)
                            } else {
                                format!("{} KB", bytes / 1024)
                            });
                            ui.separator();
                            ui.label(format!("{} × {} px", self.doc.canvas.w, self.doc.canvas.h));
                        });
                    });
                });
        }

        let names = theme::families(&self.themes, theme.dark);

        let out = ui::chrome(
            ui,
            &mut self.doc,
            &theme,
            &names,
            UiIn {
                tab: self.tab,
                picking_c1: self.picking_c1,
                show_rulers: self.show_rulers,
                show_grid: self.show_grid,
                show_status: self.show_status,
                show_thumbnail: self.show_thumbnail,
                zoom,
                qat: self.qat,
                qat_below: self.qat_below,
                ribbon_min: self.ribbon_min,
                theme_idx: self.theme_idx,
                // La cinta tiene lugar para diez; el resto vive sólo en el diálogo.
                custom: std::array::from_fn(|i| self.custom_colors[i]),
            },
            self.text_box.as_mut(),
        );
        self.picking_c1 = out.picking_c1;
        // Todos los que se anclan a un botón. El de acceso rápido faltaba acá,
        // así que se abría con el ancla del menú anterior —o en el origen— y
        // caía en medio del lienzo.
        if out.open_paste_menu
            || out.open_select_menu
            || out.open_brushes
            || out.open_outline_menu
            || out.open_fill_menu
            || out.open_rotate_menu
            || out.open_size_menu
            || out.open_theme_menu
            || out.open_qat_menu
        {
            self.menu_anchor = out.menu_anchor;
        }
        if let Some(t) = out.set_tab {
            self.tab = t;
        }
        if out.open_qat_menu {
            self.dialogs.qat_menu = true;
        }
        if out.open_settings {
            self.pane = Pane::Settings;
            self.dialogs.file_menu = true;
        }
        if out.open_paste_menu {
            self.dialogs.paste_menu = true;
        }
        if out.open_brushes {
            self.dialogs.brushes = true;
        }
        if out.open_outline_menu {
            self.dialogs.outline_menu = true;
        }
        if out.open_fill_menu {
            self.dialogs.fill_menu = true;
        }
        if out.open_rotate_menu {
            self.dialogs.rotate_menu = true;
        }
        if out.open_size_menu {
            self.dialogs.size_menu = true;
        }
        if out.open_theme_menu {
            self.dialogs.theme_menu = true;
        }
        if out.open_select_menu {
            self.dialogs.select_menu = true;
        }
        if out.open_file_menu {
            self.dialogs.file_menu = true;
        }
        if out.open_color_dialog {
            let c = if self.picking_c1 {
                self.doc.color1
            } else {
                self.doc.color2
            };
            let h = ecolor::Hsva::from_srgb([c.r(), c.g(), c.b()]);
            self.dialogs.hsv = [h.h, h.s, h.v];
            self.dialogs.color = true;
        }
        for c in out.cmds {
            self.apply(c, &ctx);
        }

        // Barra de estado.
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ui, |ui| {
                self.canvas_area(ui);
                if self.text_box.is_some() {
                    let origin = self.canvas_rect;
                    self.text_overlay(&ctx, origin, zoom);
                }
            });

        // Lo que va **encima** del lienzo y no en un panel: la píldora de
        // GNOME y la consola de SW. Si les diéramos un panel les comería
        // espacio al lienzo, que es justo lo que los dos temas evitan.
        if matches!(theme.chrome, theme::Chrome::Gnome | theme::Chrome::Holo) {
            let mut pill = ui::UiOut::new(ui::UiIn {
                tab: self.tab,
                picking_c1: self.picking_c1,
                show_rulers: self.show_rulers,
                show_grid: self.show_grid,
                show_status: self.show_status,
                show_thumbnail: self.show_thumbnail,
                zoom,
                qat: self.qat,
                qat_below: self.qat_below,
                ribbon_min: self.ribbon_min,
                theme_idx: self.theme_idx,
                custom: std::array::from_fn(|i| self.custom_colors[i]),
            });
            match theme.chrome {
                theme::Chrome::Gnome => ui::gnome_pill(&ctx, &theme, &mut self.doc, &mut pill),
                _ => ui::holo_overlay(&ctx, &theme, &mut self.doc, &mut pill),
            }
            self.picking_c1 = pill.picking_c1;
            if pill.open_color_dialog {
                self.dialogs.color = true;
            }
            // Archivo y Configuración también salen por acá. Estaban puestos y
            // nadie los leía: el mismo error que dejó muerto el menú de acceso
            // rápido hace unas semanas —una bandera que se enciende y se tira—.
            if pill.open_file_menu {
                self.dialogs.file_menu = true;
            }
            if pill.open_settings {
                self.pane = Pane::Settings;
                self.dialogs.file_menu = true;
            }
            for c in pill.cmds {
                self.apply(c, &ctx);
            }
        }

        for c in self.dialogs(&ctx) {
            self.apply(c, &ctx);
        }
        self.thumbnail(&ctx);
        self.unsaved_dialog(&ctx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_se_compone_sobre_blanco() {
        assert_eq!(opaque_rgba(&[255, 0, 0, 0]), Color32::WHITE);
        assert_eq!(opaque_rgba(&[0, 0, 0, 255]), Color32::BLACK);
        assert_eq!(
            opaque_rgba(&[255, 0, 0, 128]),
            Color32::from_rgb(255, 127, 127)
        );
    }

    #[test]
    fn zoom_por_rueda_respeta_los_limites() {
        assert_eq!(zoom_step(3, 1.0), 4);
        assert_eq!(zoom_step(3, -1.0), 2);
        assert_eq!(zoom_step(0, -1.0), 0);
        assert_eq!(zoom_step(ZOOMS.len() - 1, 1.0), ZOOMS.len() - 1);
        assert_eq!(zoom_step(3, 0.0), 3);
    }
}
