# Estado actual de Lienzo

Actualizado para la versión 0.1.1 después de la revisión integral y la
corrección de los flujos de archivo, selección y vista.

## Validación

```sh
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --check
```

- 27 tests aprobados.
- Clippy sin advertencias tratadas como error.
- Formato de Rust verificado.
- Validado en macOS sobre `aarch64-apple-darwin`.
- Probado correctamente en Windows x86_64.
- Compilado para Linux x86_64; todavía requiere validación en una distribución
  real.
- DMG, ZIP portable, Setup.exe, AppImage y DEB generados para la versión 0.1.0.
- Integridad de ZIP, DMG, AppImage, DEB y sumas SHA-256 verificada.

## Funciona

- Nueve herramientas, nueve pinceles y setenta y tres formas.
- Selección rectangular y libre, movimiento, escalado, recorte e inversión.
- Texto rasterizado con las fuentes integradas de egui.
- Historial por rectángulos con deshacer y rehacer.
- Apertura y guardado de PNG, JPEG, BMP, GIF y TIFF; apertura de ICO.
- Importación RGBA compuesta sobre el lienzo opaco.
- Portapapeles de imágenes en los dos sentidos.
- Veinte temas embebidos, temas JSON adicionales y diez idiomas.
- Ocho chromes: Ribbon, Palette, Mac, GNOME, KDE, Studio, Neon y Holo.
- Once niveles de zoom, miniatura, reglas, cuadrícula y pantalla completa.

## Protecciones de datos

Nuevo, Abrir, Abrir reciente, Salir y el botón de cierre del sistema muestran
Guardar / No guardar / Cancelar cuando hay cambios. Una acción pendiente sólo
continúa después de guardar correctamente o de elegir explícitamente no
guardar. Un error o cancelación de Guardar como conserva el documento y su ruta
anterior.

Crear o abrir un documento cancela selección, curva o polígono multietapa,
previsualización, texto flotante y arrastres del documento anterior.

## Distribución 0.1.1

| Sistema | Portable | Instalador |
|---|---|---|
| macOS ARM64 | ZIP con `Lienzo.app` | DMG |
| Windows x86_64 | ZIP con `Lienzo.exe` | Setup.exe por usuario |
| Linux x86_64 | AppImage | DEB para Debian/Ubuntu |

GitHub Actions genera los seis paquetes desde la etiqueta `v0.1.1` y los
publica con la versión en el nombre y su archivo `SHA256SUMS.txt`.

## Cambios de 0.1.1

- Zoom del lienzo con `Ctrl/Cmd + rueda` y con el gesto de pellizcar del
  trackpad.
- Menú Tamaño habilitado sólo para herramientas con grosor, con cuatro medidas
  rápidas y ajuste personalizado de 1–50 px; el borrador llega hasta 100 px.
- Miniatura rediseñada como panel flotante integrado al tema, con marco,
  dimensiones y zoom actual.
- Aplicación validada en Windows x86_64.

## Pendiente

| Área | Estado |
|---|---|
| Imprimir y vista previa | Sólo muestran un mensaje; falta integración de impresión |
| Web (WASM) | Abrir, guardar, exportar y pegar desde archivo no están implementados |
| Linux | Compila para x86_64; falta probarlo en una distribución real |
| Firma y notarización | Los instaladores funcionan, pero no tienen firma comercial |
| Selección libre | La máscara funciona; el marco visible sigue siendo rectangular |

## Arquitectura breve

`main.rs` controla ventana, archivos y diálogos; `ui.rs` dibuja los ocho
chromes; `doc.rs` gobierna herramientas y selección; `canvas.rs` conserva los
píxeles y el historial; `shapes.rs` rasteriza formas y pinceles; `text.rs`
rasteriza texto; `theme.rs` y `lang.rs` manejan apariencia e idioma.

El detalle de uso, instalación y decisiones técnicas vive en `README.md`.
