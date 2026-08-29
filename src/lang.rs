//! Idiomas.
//!
//! La clave de cada cadena es **el original en español**, no un identificador.
//! Eso tiene una consecuencia que vale más que la elegancia: una traducción que
//! falta no deja un hueco ni un `menu.file.open` en la pantalla, cae al español
//! y se sigue entendiendo. Agregar un idioma es agregar un archivo.
//!
//! `ponytail:` la lista son los diez que las fuentes incluidas saben dibujar
//! —latín y cirílico—. Chino, japonés, coreano e hindi necesitan una fuente
//! CJK de unos 16 MB; el día que se sume, esta lista y `TABLES` crecen y nada
//! más cambia.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

/// Código y nombre en su propio idioma. El primero es el original.
pub const LANGS: [(&str, &str); 10] = [
    ("es", "Español"),
    ("en", "English"),
    ("pt", "Português"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("it", "Italiano"),
    ("ru", "Русский"),
    ("pl", "Polski"),
    ("tr", "Türkçe"),
    ("nl", "Nederlands"),
];

/// Las tablas van dentro del binario: un idioma no puede faltar en tiempo de
/// ejecución, igual que los temas.
const TABLES: [&str; 10] = [
    include_str!("../lang/es.json"),
    include_str!("../lang/en.json"),
    include_str!("../lang/pt.json"),
    include_str!("../lang/fr.json"),
    include_str!("../lang/de.json"),
    include_str!("../lang/it.json"),
    include_str!("../lang/ru.json"),
    include_str!("../lang/pl.json"),
    include_str!("../lang/tr.json"),
    include_str!("../lang/nl.json"),
];

/// Global a propósito: `t()` se llama desde el fondo de la interfaz, donde
/// pasar un parámetro más significaría tocar cada función que dibuja algo.
static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PARSED: OnceLock<Vec<HashMap<String, String>>> = OnceLock::new();

fn parsed() -> &'static Vec<HashMap<String, String>> {
    PARSED.get_or_init(|| {
        TABLES
            .iter()
            .map(|t| serde_json::from_str(t).unwrap_or_default())
            .collect()
    })
}

pub fn current() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

pub fn set(i: usize) {
    CURRENT.store(i.min(LANGS.len() - 1), Ordering::Relaxed);
}

/// Busca el código guardado en los ajustes. Uno desconocido cae al español.
pub fn index_of(code: &str) -> usize {
    LANGS.iter().position(|(c, _)| *c == code).unwrap_or(0)
}

/// La cadena en el idioma puesto, o el original si no está traducida.
pub fn t(key: &'static str) -> &'static str {
    let i = current();
    if i == 0 {
        return key;
    }
    parsed()[i].get(key).map(|s| s.as_str()).unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Todas las tablas tienen que ser JSON válido. Si una se rompe al editarla,
    /// `unwrap_or_default` la deja vacía y el idioma se ve entero en español
    /// **sin que nada falle**: es justo el tipo de error que pasa desapercibido.
    #[test]
    fn todas_las_tablas_cargan() {
        for (i, (code, _)) in LANGS.iter().enumerate() {
            let t: Result<HashMap<String, String>, _> = serde_json::from_str(TABLES[i]);
            assert!(t.is_ok(), "la tabla de {code} no es JSON válido");
            if i > 0 {
                assert!(!t.unwrap().is_empty(), "la tabla de {code} está vacía");
            }
        }
    }

    /// Ninguna traducción puede quedar vacía: una cadena en blanco es un botón
    /// sin texto, y eso no se nota hasta que alguien cambia de idioma.
    #[test]
    fn ninguna_traduccion_esta_en_blanco() {
        for (i, (code, _)) in LANGS.iter().enumerate().skip(1) {
            let t: HashMap<String, String> = serde_json::from_str(TABLES[i]).unwrap();
            for (k, v) in &t {
                assert!(!v.trim().is_empty(), "{code}: «{k}» quedó sin traducir");
            }
        }
    }

    #[test]
    fn lo_que_falta_cae_al_original() {
        set(1);
        assert_eq!(t("una cadena que no existe"), "una cadena que no existe");
        set(0);
        assert_eq!(t("Guardar"), "Guardar");
    }
}
