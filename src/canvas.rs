//! Motor de píxeles. Cero UI.
//!
//! La única dependencia es `ecolor`, por el tipo de color: guardar el buffer como
//! `Vec<Color32>` premultiplicado es lo que permite subirlo a la GPU con
//! `bytemuck::cast_slice`, sin recorrer píxel por píxel.
//!
//! La idea central es el historial: en vez de guardar una copia del lienzo por
//! paso (2,9 MB a 1152×648), guarda sólo el rectángulo que el trazo ensució.

use ecolor::Color32;

/// Presupuesto del historial, en bytes. Se cuenta en memoria y no en cantidad de
/// pasos: así unos trazos chicos dan miles de niveles de deshacer, y unas pocas
/// operaciones que tocan todo el lienzo degradan con gracia.
pub const MAX_UNDO_BYTES: usize = 256 << 20;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        Self { x, y, w, h }
    }

    /// Rectángulo entre dos puntos cualesquiera, ya ordenado y recortado.
    pub fn from_corners(a: (i32, i32), b: (i32, i32), w: usize, h: usize) -> Option<Self> {
        let x0 = a.0.min(b.0).max(0) as usize;
        let y0 = a.1.min(b.1).max(0) as usize;
        let x1 = (a.0.max(b.0) + 1).max(0) as usize;
        let y1 = (a.1.max(b.1) + 1).max(0) as usize;
        let x1 = x1.min(w);
        let y1 = y1.min(h);
        if x1 <= x0 || y1 <= y0 {
            return None;
        }
        Some(Self {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        })
    }

    pub fn union(self, o: Self) -> Self {
        let x = self.x.min(o.x);
        let y = self.y.min(o.y);
        let x1 = (self.x + self.w).max(o.x + o.w);
        let y1 = (self.y + self.h).max(o.y + o.h);
        Self {
            x,
            y,
            w: x1 - x,
            h: y1 - y,
        }
    }

    pub fn area(self) -> usize {
        self.w * self.h
    }

    pub fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x as i32
            && y >= self.y as i32
            && x < (self.x + self.w) as i32
            && y < (self.y + self.h) as i32
    }
}

/// Los píxeles que había en `r` antes de una operación.
struct Patch {
    r: Rect,
    px: Vec<Color32>,
}

impl Patch {
    fn bytes(&self) -> usize {
        self.px.len() * std::mem::size_of::<Color32>()
    }
}

pub struct Canvas {
    pub w: usize,
    pub h: usize,
    px: Vec<Color32>,
    /// Copia tomada al empezar el trazo. Se reusa siempre: una sola asignación en
    /// toda la vida del programa, después es sólo un memcpy.
    scratch: Vec<Color32>,
    /// Lo ensuciado por el trazo en curso. Alimenta el historial.
    dirty: Option<Rect>,
    /// Si hay un `begin_stroke` sin su `end_stroke`.
    stroke: bool,
    /// Lo ensuciado desde el último frame. Alimenta `set_partial` en la GPU.
    upload: Option<Rect>,
    undo: Vec<Patch>,
    redo: Vec<Patch>,
    undo_bytes: usize,
    /// Se pone en true en cada cambio; el título de la ventana lo mira.
    pub dirty_file: bool,
}

impl Canvas {
    pub fn new(w: usize, h: usize) -> Self {
        assert!(w > 0 && h > 0, "el lienzo no puede tener lado cero");
        Self {
            w,
            h,
            px: vec![Color32::WHITE; w * h],
            scratch: Vec::new(),
            dirty: None,
            stroke: false,
            upload: None,
            undo: Vec::new(),
            redo: Vec::new(),
            undo_bytes: 0,
            dirty_file: false,
        }
    }

    pub fn pixels(&self) -> &[Color32] {
        &self.px
    }

    pub fn undo_bytes(&self) -> usize {
        self.undo_bytes
    }

    pub fn undo_len(&self) -> usize {
        self.undo.len()
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[inline]
    pub fn geti(&self, x: i32, y: i32) -> Option<Color32> {
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            None
        } else {
            Some(self.px[y as usize * self.w + x as usize])
        }
    }

    #[inline]
    pub fn set(&mut self, x: i32, y: i32, c: Color32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        self.px[y * self.w + x] = c;
        self.mark(Rect { x, y, w: 1, h: 1 });
    }

    /// Mezcla `c` sobre el píxel con opacidad `a` (0..=1). Para pinceles suaves:
    /// crayón, acuarela, marcador, aerógrafo.
    #[inline]
    pub fn blend(&mut self, x: i32, y: i32, c: Color32, a: f32) {
        if a <= 0.0 {
            return;
        }
        if a >= 1.0 {
            self.set(x, y, c);
            return;
        }
        let Some(d) = self.geti(x, y) else { return };
        let mix = |s: u8, d: u8| {
            (s as f32 * a + d as f32 * (1.0 - a))
                .round()
                .clamp(0.0, 255.0) as u8
        };
        let out = Color32::from_rgb(mix(c.r(), d.r()), mix(c.g(), d.g()), mix(c.b(), d.b()));
        self.set(x, y, out);
    }

    /// Recorta acá y no en cada llamador: es el único punto por donde pasan
    /// todos, y un rectángulo que se sale del lienzo termina indexando fuera de
    /// rango en `extract` cuando lo consume el historial o la subida a la GPU.
    fn mark(&mut self, r: Rect) {
        let r = clip(r, self.w, self.h);
        if r.area() == 0 {
            return;
        }
        self.dirty = Some(self.dirty.map_or(r, |d| d.union(r)));
        self.upload = Some(self.upload.map_or(r, |u| u.union(r)));
        self.dirty_file = true;
    }

    /// Las operaciones que cambian `w`/`h` no se pueden representar con un
    /// patch, así que cortan el historial. Y tienen que tirar también el trazo
    /// en curso: `scratch` quedaría con el stride viejo y `extract` leería
    /// fuera de rango.
    fn reset_history(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.undo_bytes = 0;
        self.dirty = None;
        self.stroke = false;
        self.scratch.clear();
    }

    /// Fuerza que el próximo frame suba el lienzo entero. Se usa tras cargar,
    /// redimensionar o rotar, donde cambió todo.
    pub fn mark_all(&mut self) {
        self.upload = Some(Rect {
            x: 0,
            y: 0,
            w: self.w,
            h: self.h,
        });
    }

    /// La UI lo consume cada frame para subir sólo esa zona a la textura.
    pub fn take_upload(&mut self) -> Option<Rect> {
        self.upload.take()
    }

    /// Si hay un trazo abierto. Sirve para no abrir uno adentro de otro y
    /// terminar con dos pasos de historial donde el usuario hizo una sola cosa.
    pub fn stroke_open(&self) -> bool {
        self.stroke
    }

    pub fn begin_stroke(&mut self) {
        // Abrir dos veces descartaba en silencio todo lo pintado desde la
        // primera: el trazo anterior se cierra antes.
        if self.stroke {
            self.end_stroke();
        }
        self.scratch.clear();
        self.scratch.extend_from_slice(&self.px);
        self.dirty = None;
        self.stroke = true;
    }

    pub fn end_stroke(&mut self) {
        self.stroke = false;
        let Some(r) = self.dirty.take() else { return };
        if self.scratch.len() != self.px.len() {
            // El lienzo cambió de tamaño en el medio: no hay patch coherente.
            return;
        }
        let patch = Patch {
            r,
            px: extract(&self.scratch, self.w, r),
        };
        self.push_undo(patch);
        self.redo.clear();
    }

    fn push_undo(&mut self, p: Patch) {
        self.undo_bytes += p.bytes();
        self.undo.push(p);
        // Se descarta lo más viejo, nunca el último paso: quedarse sin ningún
        // deshacer sorprende más que quedarse sin memoria.
        while self.undo_bytes > MAX_UNDO_BYTES && self.undo.len() > 1 {
            let old = self.undo.remove(0);
            self.undo_bytes -= old.bytes();
        }
    }

    pub fn undo(&mut self) {
        self.step(true);
    }

    pub fn redo(&mut self) {
        self.step(false);
    }

    fn step(&mut self, undoing: bool) {
        let popped = if undoing {
            self.undo.pop()
        } else {
            self.redo.pop()
        };
        let Some(p) = popped else { return };
        if undoing {
            self.undo_bytes -= p.bytes();
        }
        let inverse = Patch {
            r: p.r,
            px: extract(&self.px, self.w, p.r),
        };
        blit(&mut self.px, self.w, p.r, &p.px);
        self.mark(p.r);
        self.dirty = None; // deshacer no es un trazo
        if undoing {
            self.redo.push(inverse);
        } else {
            self.push_undo(inverse);
        }
    }

    /// Relleno por líneas de barrido, 4 vecinos, tolerancia cero — como Paint.
    /// La tolerancia cero es la razón por la que el lápiz y los contornos no
    /// llevan antialiasing: con bordes suavizados el balde deja halos.
    pub fn fill(&mut self, x: i32, y: i32, c: Color32) {
        if x < 0 || y < 0 {
            return;
        }
        let (x, y) = (x as usize, y as usize);
        if x >= self.w || y >= self.h {
            return;
        }
        let target = self.px[y * self.w + x];
        if target == c {
            return;
        }

        let (w, n) = (self.w, self.px.len());
        let (mut x0, mut y0, mut x1, mut y1) = (x, y, x + 1, y + 1);
        let mut stack = vec![y * w + x];

        while let Some(i) = stack.pop() {
            if self.px[i] != target {
                continue;
            }
            let row = i / w * w;
            let mut l = i;
            let mut r = i;
            while l > row && self.px[l - 1] == target {
                l -= 1;
            }
            while r + 1 < row + w && self.px[r + 1] == target {
                r += 1;
            }
            for k in l..=r {
                self.px[k] = c;
                if k >= w && self.px[k - w] == target {
                    stack.push(k - w);
                }
                if k + w < n && self.px[k + w] == target {
                    stack.push(k + w);
                }
            }
            let ry = row / w;
            x0 = x0.min(l - row);
            x1 = x1.max(r - row + 1);
            y0 = y0.min(ry);
            y1 = y1.max(ry + 1);
        }

        self.mark(Rect {
            x: x0,
            y: y0,
            w: x1 - x0,
            h: y1 - y0,
        });
    }

    /// Reemplaza sólo los píxeles que coinciden con `from`. Es el borrador
    /// selectivo del clic derecho, una de las funciones más queridas de Paint.
    pub fn replace_color_in(&mut self, r: Rect, from: Color32, to: Color32) {
        if from == to {
            return;
        }
        let mut touched = false;
        for y in r.y..(r.y + r.h).min(self.h) {
            for x in r.x..(r.x + r.w).min(self.w) {
                if self.px[y * self.w + x] == from {
                    self.px[y * self.w + x] = to;
                    touched = true;
                }
            }
        }
        // Sin este guardia, pasar el borrador selectivo por una zona donde no
        // hay nada del color objetivo ensuciaría el archivo y forzaría una
        // subida a la GPU para no cambiar ni un píxel.
        if touched {
            self.mark(r);
        }
    }

    // ---- región: copiar, pegar, limpiar ----

    pub fn region(&self, r: Rect) -> Vec<Color32> {
        extract(&self.px, self.w, r)
    }

    pub fn put_region(&mut self, r: Rect, src: &[Color32]) {
        blit_clipped(&mut self.px, self.w, self.h, r, src);
        self.mark(clip(r, self.w, self.h));
    }

    /// Igual que `put_region` pero saltea los píxeles que coinciden con `key`.
    /// Es la "selección transparente" de Paint: sólo composición, nunca alfa.
    pub fn put_region_keyed(&mut self, r: Rect, src: &[Color32], key: Color32) {
        for (row, y) in (r.y as i32..r.y as i32 + r.h as i32).enumerate() {
            for (col, x) in (r.x as i32..r.x as i32 + r.w as i32).enumerate() {
                let c = src[row * r.w + col];
                if c != key {
                    self.set(x, y, c);
                }
            }
        }
    }

    pub fn clear_region(&mut self, r: Rect, c: Color32) {
        let r = clip(r, self.w, self.h);
        if r.area() == 0 {
            return;
        }
        for y in r.y..r.y + r.h {
            self.px[y * self.w + r.x..y * self.w + r.x + r.w].fill(c);
        }
        self.mark(r);
    }

    pub fn invert_region(&mut self, r: Rect) {
        let r = clip(r, self.w, self.h);
        for y in r.y..r.y + r.h {
            for x in r.x..r.x + r.w {
                let c = self.px[y * self.w + x];
                self.px[y * self.w + x] = Color32::from_rgb(255 - c.r(), 255 - c.g(), 255 - c.b());
            }
        }
        self.mark(r);
    }

    // ---- operaciones sobre todo el lienzo ----
    // Cambian `w`/`h`, así que se registran con `snapshot_all` y no con trazos.

    fn snapshot_all(&mut self) {
        let r = Rect {
            x: 0,
            y: 0,
            w: self.w,
            h: self.h,
        };
        let patch = Patch {
            r,
            px: self.px.clone(),
        };
        self.push_undo(patch);
        self.redo.clear();
    }

    /// Cambia el tamaño del lienzo sin escalar la imagen (recorta o agranda).
    /// El área nueva se rellena con `bg`, como los tiradores de Paint.
    pub fn resize_canvas(&mut self, w: usize, h: usize, bg: Color32) {
        if w == 0 || h == 0 || (w == self.w && h == self.h) {
            return;
        }
        // El historial no puede representar un cambio de tamaño con un patch,
        // así que acá se corta: es el único caso donde se limpia.
        self.reset_history();

        let mut out = vec![bg; w * h];
        let cw = w.min(self.w);
        let ch = h.min(self.h);
        for y in 0..ch {
            out[y * w..y * w + cw].copy_from_slice(&self.px[y * self.w..y * self.w + cw]);
        }
        self.px = out;
        self.w = w;
        self.h = h;
        self.mark_all();
        self.dirty_file = true;
    }

    /// Escala la imagen (esto sí es "Cambiar tamaño" de Paint). Vecino más
    /// cercano, igual que Paint: mantiene el pixel art nítido.
    /// `ponytail:` vecino más cercano. Si alguien pide reducciones suaves,
    /// acá va un promedio de área.
    pub fn scale(&mut self, w: usize, h: usize) {
        if w == 0 || h == 0 || (w == self.w && h == self.h) {
            return;
        }
        self.reset_history();

        let mut out = vec![Color32::WHITE; w * h];
        for y in 0..h {
            let sy = y * self.h / h;
            for x in 0..w {
                let sx = x * self.w / w;
                out[y * w + x] = self.px[sy * self.w + sx];
            }
        }
        self.px = out;
        self.w = w;
        self.h = h;
        self.mark_all();
        self.dirty_file = true;
    }

    pub fn flip_horizontal(&mut self) {
        self.snapshot_all();
        for y in 0..self.h {
            self.px[y * self.w..(y + 1) * self.w].reverse();
        }
        self.mark_all();
        self.dirty_file = true;
    }

    pub fn flip_vertical(&mut self) {
        self.snapshot_all();
        let (w, h) = (self.w, self.h);
        for y in 0..h / 2 {
            for x in 0..w {
                self.px.swap(y * w + x, (h - 1 - y) * w + x);
            }
        }
        self.mark_all();
        self.dirty_file = true;
    }

    /// `quarters` en sentido horario: 1 = 90°, 2 = 180°, 3 = 270°.
    pub fn rotate(&mut self, quarters: u32) {
        let q = quarters % 4;
        if q == 0 {
            return;
        }
        self.reset_history();

        let (w, h) = (self.w, self.h);
        let (nw, nh) = if q == 2 { (w, h) } else { (h, w) };
        let mut out = vec![Color32::WHITE; nw * nh];
        for y in 0..h {
            for x in 0..w {
                let c = self.px[y * w + x];
                let (dx, dy) = match q {
                    1 => (h - 1 - y, x),
                    2 => (w - 1 - x, h - 1 - y),
                    _ => (y, w - 1 - x),
                };
                out[dy * nw + dx] = c;
            }
        }
        self.px = out;
        self.w = nw;
        self.h = nh;
        self.mark_all();
        self.dirty_file = true;
    }

    /// Sesga la imagen en grados. Agranda el lienzo para contener el
    /// paralelogramo y rellena las esquinas con `bg`, como Paint.
    pub fn skew(&mut self, deg_x: f32, deg_y: f32, bg: Color32) {
        let tx = deg_x.to_radians().tan();
        let ty = deg_y.to_radians().tan();
        if tx == 0.0 && ty == 0.0 {
            return;
        }
        self.reset_history();

        let (w, h) = (self.w as f32, self.h as f32);
        let nw = (w + (tx * h).abs()).ceil() as usize;
        let nh = (h + (ty * w).abs()).ceil() as usize;
        let ox = if tx < 0.0 { (tx * h).abs() } else { 0.0 };
        let oy = if ty < 0.0 { (ty * w).abs() } else { 0.0 };

        let mut out = vec![bg; nw * nh];
        for dy in 0..nh {
            for dx in 0..nw {
                // Transformada inversa: de destino a origen.
                let fx = dx as f32 - ox;
                let fy = dy as f32 - oy;
                let sy = fy - ty * fx;
                let sx = fx - tx * sy;
                if sx >= 0.0 && sy >= 0.0 && (sx as usize) < self.w && (sy as usize) < self.h {
                    out[dy * nw + dx] = self.px[sy as usize * self.w + sx as usize];
                }
            }
        }
        self.px = out;
        self.w = nw;
        self.h = nh;
        self.mark_all();
        self.dirty_file = true;
    }

    /// Reemplaza todo el contenido (abrir un archivo).
    pub fn load(&mut self, w: usize, h: usize, px: Vec<Color32>) {
        debug_assert_eq!(px.len(), w * h);
        self.reset_history();
        self.px = px;
        self.w = w;
        self.h = h;
        self.mark_all();
        self.dirty_file = false;
    }
}

fn clip(r: Rect, w: usize, h: usize) -> Rect {
    let x = r.x.min(w);
    let y = r.y.min(h);
    Rect {
        x,
        y,
        w: (r.x + r.w).min(w).saturating_sub(x),
        h: (r.y + r.h).min(h).saturating_sub(y),
    }
}

fn extract(src: &[Color32], stride: usize, r: Rect) -> Vec<Color32> {
    let mut out = Vec::with_capacity(r.area());
    for y in r.y..r.y + r.h {
        let a = y * stride + r.x;
        out.extend_from_slice(&src[a..a + r.w]);
    }
    out
}

fn blit(dst: &mut [Color32], stride: usize, r: Rect, src: &[Color32]) {
    for (row, y) in (r.y..r.y + r.h).enumerate() {
        let a = y * stride + r.x;
        dst[a..a + r.w].copy_from_slice(&src[row * r.w..(row + 1) * r.w]);
    }
}

/// Como `blit` pero tolera que `r` se salga del lienzo (pegar cerca del borde).
fn blit_clipped(dst: &mut [Color32], stride: usize, h: usize, r: Rect, src: &[Color32]) {
    for row in 0..r.h {
        let y = r.y + row;
        if y >= h {
            break;
        }
        for col in 0..r.w {
            let x = r.x + col;
            if x >= stride {
                break;
            }
            dst[y * stride + x] = src[row * r.w + col];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Las dos cosas que pueden romperse en silencio: que deshacer no devuelva
    /// el bitmap exacto, y que el relleno se desborde o deje bordes sin pintar.
    #[test]
    fn undo_es_exacto_y_el_relleno_respeta_los_bordes() {
        let mut c = Canvas::new(64, 48);
        let limpio = c.pixels().to_vec();

        // Una caja hueca de borde negro, de (10,10) a (30,30).
        c.begin_stroke();
        for x in 10..=30 {
            c.set(x, 10, Color32::BLACK);
            c.set(x, 30, Color32::BLACK);
        }
        for y in 10..=30 {
            c.set(10, y, Color32::BLACK);
            c.set(30, y, Color32::BLACK);
        }
        c.end_stroke();
        let con_caja = c.pixels().to_vec();

        // El rectángulo sucio es la caja, no el lienzo entero. Ese es el punto.
        assert_eq!(c.undo_bytes(), 21 * 21 * 4, "el historial guardó de más");

        // Rellenar adentro no debe escaparse por ningún lado.
        c.begin_stroke();
        c.fill(20, 20, Color32::RED);
        c.end_stroke();

        assert_eq!(
            c.geti(20, 20).unwrap(),
            Color32::RED,
            "no rellenó el interior"
        );
        assert_eq!(
            c.geti(11, 11).unwrap(),
            Color32::RED,
            "no llegó a la esquina interior"
        );
        assert_eq!(c.geti(10, 10).unwrap(), Color32::BLACK, "se comió el borde");
        assert_eq!(
            c.geti(31, 20).unwrap(),
            Color32::WHITE,
            "se filtró hacia afuera"
        );
        assert_eq!(
            c.geti(0, 0).unwrap(),
            Color32::WHITE,
            "se filtró hasta el origen"
        );

        // Ida y vuelta: byte a byte, no "parecido".
        c.undo();
        assert_eq!(
            c.pixels(),
            &con_caja[..],
            "deshacer el relleno no fue exacto"
        );
        c.undo();
        assert_eq!(c.pixels(), &limpio[..], "deshacer la caja no fue exacto");

        c.redo();
        assert_eq!(c.pixels(), &con_caja[..], "rehacer la caja no fue exacto");
        c.redo();
        assert_eq!(
            c.geti(20, 20).unwrap(),
            Color32::RED,
            "rehacer el relleno no funcionó"
        );

        // Deshacer de más no debe romper ni hacer nada raro.
        for _ in 0..5 {
            c.undo();
        }
        assert_eq!(
            c.pixels(),
            &limpio[..],
            "deshacer de más corrompió el lienzo"
        );
        assert_eq!(c.undo_len(), 0);
    }

    #[test]
    fn rellenar_del_mismo_color_no_hace_nada() {
        let mut c = Canvas::new(8, 8);
        c.fill(0, 0, Color32::WHITE);
        assert!(c.take_upload().is_none(), "marcó trabajo que no hizo");
    }

    /// Rotar cuatro veces tiene que devolver exactamente la imagen original;
    /// si los índices están mal, esto lo caza enseguida.
    #[test]
    fn rotar_cuatro_veces_vuelve_al_origen() {
        let mut c = Canvas::new(7, 3); // no cuadrado, a propósito
        c.set(0, 0, Color32::RED);
        c.set(6, 0, Color32::from_rgb(0, 255, 0));
        c.set(0, 2, Color32::BLUE);
        let original = c.pixels().to_vec();

        for _ in 0..4 {
            c.rotate(1);
        }
        assert_eq!(c.w, 7);
        assert_eq!(c.h, 3);
        assert_eq!(
            c.pixels(),
            &original[..],
            "cuatro giros no volvieron al origen"
        );
    }

    #[test]
    fn voltear_dos_veces_vuelve_al_origen() {
        let mut c = Canvas::new(5, 4);
        c.set(0, 0, Color32::RED);
        c.set(4, 3, Color32::BLUE);
        let original = c.pixels().to_vec();

        c.flip_horizontal();
        assert_ne!(c.pixels(), &original[..], "voltear no hizo nada");
        c.flip_horizontal();
        assert_eq!(
            c.pixels(),
            &original[..],
            "voltear horizontal no es su propio inverso"
        );

        c.flip_vertical();
        c.flip_vertical();
        assert_eq!(
            c.pixels(),
            &original[..],
            "voltear vertical no es su propio inverso"
        );
    }
}
