//! Temas: un archivo JSON define todo el aspecto.
//!
//! Decisión de diseño: **no serializamos `egui::Style`.** Volcarlo daría un JSON
//! de cientos de campos que nadie edita a mano. En cambio definimos ~30 tokens
//! con nombre y construimos el `Style` a partir de ellos. Con `serde(default)`,
//! un tema puede traer sólo los seis campos que le importan.
//!
//! Lo que un tema **no** puede cambiar es el layout: egui no tiene layout por
//! datos. Por eso elige entre tres *chromes* que sí están en el código.

use ecolor::Color32;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Cómo se acomoda la misma tabla de comandos. Tres funciones, seis temas.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Chrome {
    /// Cinta arriba con grupos etiquetados. Windows 7, 10 y 11.
    Ribbon,
    /// Grilla de herramientas a la izquierda, paleta abajo. Windows XP y 98.
    Palette,
    /// Barra unificada con el título centrado, herramientas en una lateral y
    /// opciones en un inspector a la derecha. macOS.
    Mac,
    /// La barra de título **es** la barra de herramientas: botones enlazados,
    /// una sola acción sugerida y el menú en tres rayas. Lo del momento flota
    /// sobre el lienzo. GNOME.
    Gnome,
    /// Barra de menú, barra de iconos, panel de herramientas y panel de color.
    /// GNOME esconde y KDE muestra: son dos filosofías opuestas y por eso son
    /// dos chromes y no uno con distintos colores.
    Kde,
    /// Una sola barra al pie y el lienzo hasta los cuatro bordes. 2077.
    Neon,
    /// Nada pegado a los bordes: una consola y un lector que **flotan** sobre
    /// el lienzo, con dos esquinas cortadas a 45°. SW.
    Holo,
    /// El propio: riel de herramientas al costado, una barra que muestra sólo
    /// lo que la herramienta elegida sabe hacer, y el lienzo flotando con todo
    /// lo que sobra. Lienzo y Pigmento.
    Studio,
}

/// Un color que en JSON se ve como `"#rrggbb"`, para que un humano lo pueda
/// escribir. Es la razón por la que no usamos `Color32` directo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Col(pub Color32);

impl Col {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self(Color32::from_rgb(r, g, b))
    }
}

impl From<Col> for Color32 {
    fn from(c: Col) -> Self {
        c.0
    }
}

impl Serialize for Col {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let c = self.0;
        s.serialize_str(&format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b()))
    }
}

impl<'de> Deserialize<'de> for Col {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        parse_hex(&s).map(Col).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "color inválido {s:?}: se espera \"#rrggbb\" o \"#rgb\""
            ))
        })
    }
}

pub fn parse_hex(s: &str) -> Option<Color32> {
    let h = s.trim().trim_start_matches('#');
    // Sin este guardia, cortar por índice de byte revienta con cualquier
    // carácter multibyte, y esto lee archivos que escribe el usuario.
    if !h.is_ascii() {
        return None;
    }
    let v = |i: usize, n: usize| u8::from_str_radix(&h[i..i + n], 16).ok();
    match h.len() {
        6 => Some(Color32::from_rgb(v(0, 2)?, v(2, 2)?, v(4, 2)?)),
        3 => {
            let d = |i: usize| v(i, 1).map(|x| x * 17);
            Some(Color32::from_rgb(d(0)?, d(1)?, d(2)?))
        }
        _ => None,
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    pub name: String,
    pub chrome: Chrome,
    pub dark: bool,
    /// El nombre del tema del otro modo. Es lo que hace que el interruptor
    /// claro/oscuro de Configuración sepa a dónde saltar sin adivinar.
    #[serde(default)]
    pub pair: Option<String>,

    // --- superficies ---
    pub window: Col,
    pub surface: Col,
    pub surface_alt: Col,
    /// El gris alrededor del lienzo.
    pub workspace: Col,

    // --- bordes ---
    pub border: Col,
    pub border_strong: Col,

    // --- texto ---
    pub text: Col,
    pub text_dim: Col,
    pub font_size: f32,

    /// Los iconos de herramientas y formas. En Paint son azules, no negros:
    /// es la diferencia que más se nota de lejos.
    pub icon: Col,

    // --- acento y selección ---
    pub accent: Col,
    /// El acento corrido al otro canal. Sólo lo usa 2077, donde lo elegido no
    /// se rellena sino que se desalinea; el resto de los temas lo deja igual al
    /// acento y no cambia nada.
    pub accent_alt: Col,
    /// Lo que está por pasar: el paso del cursor. En 2077 es el tercer canal;
    /// en los demás vale lo mismo que el acento.
    pub accent_hot: Col,
    pub accent_soft: Col,
    pub accent_text: Col,

    // --- botones ---
    pub button: Col,
    pub button_hover: Col,
    pub button_active: Col,
    pub button_border: Col,

    // --- cinta ---
    pub ribbon: Col,
    pub ribbon_tab_active: Col,
    pub ribbon_group_label: Col,
    pub ribbon_separator: Col,
    /// La pestaña Archivo, que en Windows es de color pleno.
    pub file_tab: Col,
    pub file_tab_text: Col,

    // --- barra de título ---
    /// egui no tiene degradés: `bar_top` y `bar_bottom` los pintamos a mano.
    /// El Aero de Win7 y el Luna de XP son degradés, no colores planos.
    pub bar_top: Col,
    pub bar_bottom: Col,

    // --- geometría ---
    pub rounding: f32,
    pub button_rounding: f32,
    pub item_spacing: f32,
    pub button_padding: f32,
    /// Alto de la banda de la cinta. Fija a propósito: la cinta se dibuja con
    /// geometría precalculada, no con reflow (ver PLAN.md, riesgo de layout).
    pub ribbon_height: f32,
    /// Ancho de la barra de desplazamiento. Windows XP la quiere cuadrada y
    /// ancha, un Mac fina y flotante: subirla de contraste para un tema se la
    /// subía a todos.
    pub scroll_width: f32,
}

impl Default for Theme {
    /// El tema base es Lienzo, el propio. Además de ser el de fábrica, es de
    /// donde saca los valores cualquier tema al que le falte un token.
    fn default() -> Self {
        Self {
            name: "Lienzo".into(),
            chrome: Chrome::Ribbon,
            dark: false,
            pair: Some("Lienzo Tinta".into()),

            window: Col::rgb(0xe8, 0xe6, 0xe1),
            surface: Col::rgb(0xfd, 0xfd, 0xfc),
            surface_alt: Col::rgb(0xf5, 0xf6, 0xf7),
            workspace: Col::rgb(0x80, 0x80, 0x80),

            border: Col::rgb(0xd6, 0xd6, 0xd6),
            border_strong: Col::rgb(0x9a, 0x9a, 0x9a),

            text: Col::rgb(0x1a, 0x1a, 0x1a),
            text_dim: Col::rgb(0x6b, 0x6b, 0x6b),
            font_size: 12.0,

            icon: Col::rgb(0x1e, 0x6f, 0xc4),

            accent: Col::rgb(0x00, 0x78, 0xd7),
            accent_alt: Col::rgb(0x00, 0x78, 0xd7),
            accent_hot: Col::rgb(0x00, 0x78, 0xd7),
            accent_soft: Col::rgb(0xcc, 0xe4, 0xf7),
            accent_text: Col::rgb(0xff, 0xff, 0xff),

            button: Col::rgb(0xf0, 0xf0, 0xf0),
            button_hover: Col::rgb(0xe5, 0xf1, 0xfb),
            button_active: Col::rgb(0xcc, 0xe4, 0xf7),
            button_border: Col::rgb(0xd6, 0xd6, 0xd6),

            ribbon: Col::rgb(0xff, 0xff, 0xff),
            ribbon_tab_active: Col::rgb(0xff, 0xff, 0xff),
            ribbon_group_label: Col::rgb(0x6b, 0x6b, 0x6b),
            ribbon_separator: Col::rgb(0xe0, 0xe0, 0xe0),
            file_tab: Col::rgb(0x00, 0x63, 0xb1),
            file_tab_text: Col::rgb(0xff, 0xff, 0xff),

            bar_top: Col::rgb(0xff, 0xff, 0xff),
            bar_bottom: Col::rgb(0xff, 0xff, 0xff),

            rounding: 2.0,
            button_rounding: 2.0,
            item_spacing: 4.0,
            button_padding: 6.0,
            ribbon_height: 118.0,
            scroll_width: 12.0,
        }
    }
}

impl Theme {
    /// Carga un tema de un JSON. Los campos que falten toman el valor por
    /// defecto, así que un tema válido puede tener tres líneas.
    pub fn from_json(s: &str) -> Result<Self, String> {
        serde_json::from_str(s).map_err(|e| format!("tema inválido: {e}"))
    }

    /// Traduce los tokens al `Style` de egui. Los widgets estándar (botones,
    /// deslizadores, cuadros de texto) quedan vestidos con esto solo.
    pub fn to_style(&self) -> egui::Style {
        // `Style::default()` en egui es el tema **oscuro**. Arrancar de ahí para
        // seis temas claros dejaba el cursor de texto, las sombras y los bordes
        // de ventana con valores pensados para fondo negro.
        let mut s = egui::Style {
            visuals: if self.dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            },
            ..Default::default()
        };
        let v = &mut s.visuals;

        v.dark_mode = self.dark;
        v.panel_fill = self.surface.into();
        v.window_fill = self.surface.into();
        v.extreme_bg_color = self.surface_alt.into();
        v.faint_bg_color = self.surface_alt.into();
        // Nada de `override_text_color`: pisa el `fg_stroke` de cada estado, así
        // que el texto atenuado, el deshabilitado y el resaltado saldrían todos
        // del mismo color y `text_dim` no aparecería nunca.
        v.window_stroke = egui::Stroke::new(1.0, Color32::from(self.border));
        v.selection.bg_fill = self.accent_soft.into();
        v.selection.stroke = egui::Stroke::new(1.0, Color32::from(self.accent));
        v.hyperlink_color = self.accent.into();

        let cr = egui::CornerRadius::same(self.button_rounding as u8);

        // Los cinco estados que egui expone. No hay estilos por tipo de widget:
        // todo botón, casilla y deslizador comparte estos cinco.
        v.widgets.noninteractive.bg_fill = self.surface.into();
        v.widgets.noninteractive.weak_bg_fill = self.surface.into();
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, Color32::from(self.border));
        v.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, Color32::from(self.text_dim));
        v.widgets.noninteractive.corner_radius = cr;

        v.widgets.inactive.bg_fill = self.button.into();
        v.widgets.inactive.weak_bg_fill = self.button.into();
        v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, Color32::from(self.button_border));
        v.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, Color32::from(self.text));
        v.widgets.inactive.corner_radius = cr;

        v.widgets.hovered.bg_fill = self.button_hover.into();
        v.widgets.hovered.weak_bg_fill = self.button_hover.into();
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, Color32::from(self.accent));
        v.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, Color32::from(self.text));
        v.widgets.hovered.corner_radius = cr;

        v.widgets.active.bg_fill = self.button_active.into();
        v.widgets.active.weak_bg_fill = self.button_active.into();
        v.widgets.active.bg_stroke = egui::Stroke::new(1.0, Color32::from(self.accent));
        v.widgets.active.fg_stroke = egui::Stroke::new(1.0, Color32::from(self.text));
        v.widgets.active.corner_radius = cr;

        v.widgets.open.bg_fill = self.button_active.into();
        v.widgets.open.weak_bg_fill = self.button_active.into();
        v.widgets.open.bg_stroke = egui::Stroke::new(1.0, Color32::from(self.border_strong));
        v.widgets.open.fg_stroke = egui::Stroke::new(1.0, Color32::from(self.text));
        v.widgets.open.corner_radius = cr;

        // Barras de desplazamiento fijas, no flotantes. Las de egui se
        // esconden hasta que pasás el mouse por encima; las de Paint están
        // siempre, y sin verlas no se sabe cuánto lienzo queda fuera de vista.
        s.spacing.scroll = egui::style::ScrollStyle {
            floating: false,
            bar_width: self.scroll_width,
            bar_inner_margin: 2.0,
            bar_outer_margin: 0.0,
            handle_min_length: 24.0,
            dormant_background_opacity: 1.0,
            active_background_opacity: 1.0,
            interact_background_opacity: 1.0,
            dormant_handle_opacity: 1.0,
            active_handle_opacity: 1.0,
            interact_handle_opacity: 1.0,
            // El tirador con el color del texto, no con el de relleno: de
            // fábrica es un gris claro sobre fondo claro y con sesenta formas
            // en la galería nadie se entera de que hay más abajo.
            //
            // Menos en el chrome de paleta: la barra de Windows 98 y XP es un
            // bloque **claro** con relieve sobre un canal más oscuro, y con el
            // color del texto salía una viga negra cruzando la ventana.
            foreground_color: !matches!(self.chrome, Chrome::Palette),
            ..egui::style::ScrollStyle::solid()
        };
        // El canal por donde corre. `extreme_bg_color` es lo que egui pinta de
        // fondo de la barra, y de fábrica es blanco.
        s.visuals.extreme_bg_color = if self.chrome == Chrome::Palette {
            // El canal hundido de XP: más oscuro que la cara, no más claro.
            Color32::from(self.surface_alt)
        } else if self.dark {
            Color32::from(self.surface_alt)
        } else {
            Color32::from(self.border)
        };

        s.spacing.item_spacing = egui::vec2(self.item_spacing, self.item_spacing);
        s.spacing.button_padding = egui::vec2(self.button_padding, self.button_padding * 0.6);

        for f in s.text_styles.values_mut() {
            f.size = self.font_size * (f.size / 12.0).max(0.8);
        }
        s.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::proportional(self.font_size),
        );
        s.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::proportional(self.font_size),
        );

        s
    }
}

/// Pinta un degradé vertical de dos paradas. egui no los tiene, así que las
/// barras de XP y de Windows 7 se dibujan con esto.
///
/// `ponytail:` dos paradas, vertical. Si algún tema pide radial o multiparada,
/// acá se cambia — no hace falta antes.
pub fn gradient_bar(painter: &egui::Painter, rect: egui::Rect, top: Color32, bottom: Color32) {
    if top == bottom {
        painter.rect_filled(rect, 0.0, top);
        return;
    }
    // 24 bandas alcanzan para que no se vea escalonado a estas alturas.
    const BANDS: usize = 24;
    let h = rect.height() / BANDS as f32;
    for i in 0..BANDS {
        let t = i as f32 / (BANDS - 1) as f32;
        let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
        let c = Color32::from_rgb(
            lerp(top.r(), bottom.r()),
            lerp(top.g(), bottom.g()),
            lerp(top.b(), bottom.b()),
        );
        let y = rect.top() + i as f32 * h;
        painter.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(rect.left(), y),
                egui::vec2(rect.width(), h + 1.0),
            ),
            0.0,
            c,
        );
    }
}

/// Los seis temas que vienen de fábrica, embebidos en el binario para que la
/// aplicación arranque aunque no encuentre la carpeta `themes/`.
/// Una entrada por familia: el nombre que se muestra y el índice del tema que
/// hay que poner.
///
/// La familia se arma con la **pareja declarada**, no recortando « oscuro» del
/// nombre. Lienzo y Lienzo Tinta son pareja y no comparten ni una palabra, así
/// que cualquier truco con el texto los partiría en dos familias.
pub fn families(themes: &[Theme], dark: bool) -> Vec<(String, usize)> {
    let mut out = Vec::new();
    let mut vistos: Vec<String> = Vec::new();
    for (i, t) in themes.iter().enumerate() {
        if vistos.contains(&t.name) {
            continue;
        }
        vistos.push(t.name.clone());
        let pareja = t
            .pair
            .as_ref()
            .and_then(|p| themes.iter().position(|o| &o.name == p));
        if let Some(j) = pareja {
            vistos.push(themes[j].name.clone());
        }
        // El nombre es el del claro; el índice, el de la variante del modo puesto.
        let claro = if t.dark {
            pareja.map_or_else(|| t.name.clone(), |j| themes[j].name.clone())
        } else {
            t.name.clone()
        };
        let idx = if t.dark == dark {
            i
        } else {
            pareja.unwrap_or(i)
        };
        out.push((claro, idx));
    }
    out
}

pub const BUILTIN: [(&str, &str); 20] = [
    // El orden es el que se ve en la grilla de Configuración, y el primero es
    // el que arranca puesto la primera vez. Cada familia entra con su par
    // claro/oscuro: sin el par, el interruptor de Configuración no tiene a
    // dónde saltar y te saca del estilo que elegiste.
    ("Lienzo", include_str!("../themes/lienzo.json")),
    ("Lienzo Tinta", include_str!("../themes/lienzo-tinta.json")),
    ("2077", include_str!("../themes/2077.json")),
    ("2077 nocturno", include_str!("../themes/2077-noche.json")),
    ("SW", include_str!("../themes/sw.json")),
    ("SW nocturno", include_str!("../themes/sw-noche.json")),
    ("Windows 10", include_str!("../themes/win10.json")),
    (
        "Windows 10 oscuro",
        include_str!("../themes/win10-dark.json"),
    ),
    ("Windows 11", include_str!("../themes/win11.json")),
    (
        "Windows 11 oscuro",
        include_str!("../themes/win11-dark.json"),
    ),
    ("Windows 7", include_str!("../themes/win7.json")),
    ("Windows 7 oscuro", include_str!("../themes/win7-dark.json")),
    ("Windows XP", include_str!("../themes/winxp.json")),
    (
        "Windows XP oscuro",
        include_str!("../themes/winxp-dark.json"),
    ),
    ("GNOME", include_str!("../themes/linux.json")),
    ("GNOME oscuro", include_str!("../themes/linux-dark.json")),
    ("KDE", include_str!("../themes/kde.json")),
    ("KDE oscuro", include_str!("../themes/kde-dark.json")),
    ("macOS", include_str!("../themes/macos.json")),
    ("macOS oscuro", include_str!("../themes/macos-dark.json")),
];

/// Carga los temas de fábrica más los `.json` que haya en `themes/`, para que
/// alguien pueda dejar el suyo al lado del ejecutable.
pub fn load_all() -> Vec<Theme> {
    let mut out: Vec<Theme> = BUILTIN
        .iter()
        .filter_map(|(name, src)| match Theme::from_json(src) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("tema de fábrica {name} inválido: {e}");
                None
            }
        })
        .collect();

    if let Ok(dir) = std::fs::read_dir("themes") {
        for entry in dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path)
                .map_err(|e| e.to_string())
                .and_then(|s| Theme::from_json(&s))
            {
                Ok(t) => {
                    if !out.iter().any(|e| e.name == t.name) {
                        out.push(t);
                    }
                }
                // Un tema roto del usuario avisa y se ignora; no tira la app.
                Err(e) => eprintln!("no pude cargar {}: {e}", path.display()),
            }
        }
    }

    if out.is_empty() {
        out.push(Theme::default());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cada tema tiene que tener pareja del otro modo, y esa pareja tiene que
    /// volver a él. Sin esto, el interruptor claro/oscuro cae en el primer tema
    /// del modo pedido y te saca del estilo que elegiste — que es exactamente
    /// lo que pasaba con Windows 7, XP y Pigmento.
    #[test]
    fn todos_los_temas_tienen_pareja_de_ida_y_vuelta() {
        let ts: Vec<Theme> = BUILTIN
            .iter()
            .map(|(n, src)| Theme::from_json(src).unwrap_or_else(|e| panic!("{n}: {e}")))
            .collect();
        for t in &ts {
            let par = t
                .pair
                .as_ref()
                .unwrap_or_else(|| panic!("{} no tiene pareja", t.name));
            let otro = ts
                .iter()
                .find(|o| &o.name == par)
                .unwrap_or_else(|| panic!("{}: la pareja {par:?} no existe", t.name));
            assert_ne!(
                otro.dark, t.dark,
                "{} y {par} son los dos del mismo modo",
                t.name
            );
            assert_eq!(
                otro.pair.as_deref(),
                Some(t.name.as_str()),
                "{} apunta a {par}, pero {par} no vuelve",
                t.name
            );
        }
    }

    #[test]
    fn los_colores_van_y_vuelven_en_hexadecimal() {
        assert_eq!(parse_hex("#ff8000"), Some(Color32::from_rgb(255, 128, 0)));
        assert_eq!(parse_hex("f80"), Some(Color32::from_rgb(255, 136, 0)));
        assert_eq!(parse_hex("nope"), None);
        assert_eq!(parse_hex("#12345"), None);

        let c = Col::rgb(0x12, 0x34, 0x56);
        let json = serde_json::to_string(&c).unwrap();
        assert_eq!(json, "\"#123456\"");
        let back: Col = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
    }

    /// Lo importante de `serde(default)`: un tema puede traer tres líneas y el
    /// resto se completa solo. Si esto se rompe, los temas de usuario dejan de
    /// funcionar sin aviso.
    #[test]
    fn un_tema_parcial_hereda_el_resto() {
        // Ojo con las almohadillas: r#"…"# termina en el "# de "#ff0000".
        let t = Theme::from_json(r##"{"name":"Mío","accent":"#ff0000"}"##).unwrap();
        assert_eq!(t.name, "Mío");
        assert_eq!(Color32::from(t.accent), Color32::from_rgb(255, 0, 0));
        // No especificado: viene del default de Windows 10.
        assert_eq!(t.chrome, Chrome::Ribbon);
        assert_eq!(
            Color32::from(t.workspace),
            Color32::from_rgb(0x80, 0x80, 0x80)
        );
    }

    #[test]
    fn un_tema_invalido_da_error_y_no_panic() {
        assert!(Theme::from_json("{").is_err());
        assert!(Theme::from_json(r#"{"accent":"no-es-color"}"#).is_err());
        assert!(Theme::from_json(r#"{"chrome":"inventado"}"#).is_err());
    }

    /// Todos los temas de fábrica tienen que parsear. Van embebidos con
    /// `include_str!`, así que un JSON roto rompería el arranque.
    ///
    /// Antes esto exigía que fueran seis y quedó en rojo al llegar a veinte: un
    /// número fijo sólo mide cuántos había el día que se escribió. Lo que sí
    /// vale la pena es que **ningún nombre se repita** — la línea de
    /// `include_str!` se agrega copiando la de arriba, y con el nombre sin
    /// cambiar el tema nuevo queda invisible detrás del viejo.
    #[test]
    fn todos_los_temas_de_fabrica_parsean_y_no_se_repiten() {
        let mut nombres = Vec::new();
        for (name, src) in BUILTIN {
            let t =
                Theme::from_json(src).unwrap_or_else(|e| panic!("el tema {name} no parsea: {e}"));
            assert!(!t.name.is_empty(), "{name} no tiene nombre");
            assert!(
                !nombres.contains(&t.name),
                "el nombre «{}» está dos veces",
                t.name
            );
            nombres.push(t.name);
        }
    }
}
