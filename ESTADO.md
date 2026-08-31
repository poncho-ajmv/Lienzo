# Estado actual de Lienzo

La versión 0.1.3 está publicada. La rama principal prepara 0.1.4 con impresión
nativa y la selección libre terminada.

## Validación

```sh
cargo test --locked --offline
cargo clippy --locked --offline --all-targets -- -D warnings
cargo fmt --check
```

- 29 tests aprobados.
- Clippy sin advertencias tratadas como error.
- Formato de Rust verificado.
- Validado en macOS sobre `aarch64-apple-darwin`.
- Probado correctamente en Windows x86_64.
- Compilado para Linux x86_64; todavía requiere validación en una distribución
  real.
- DMG, ZIP portable, Setup.exe, AppImage y DEB publicados para la versión 0.1.3.
- Integridad de ZIP, DMG, AppImage, DEB y sumas SHA-256 verificada.

## Funciona

- Nueve herramientas, nueve pinceles y setenta y tres formas.
- Selección rectangular y libre con contorno real, movimiento, escalado,
  recorte e inversión.
- Texto rasterizado con las fuentes integradas de egui.
- Historial por rectángulos con deshacer y rehacer.
- Apertura y guardado de PNG, JPEG, BMP, GIF y TIFF; apertura de ICO.
- Importación RGBA compuesta sobre el lienzo opaco.
- Portapapeles de imágenes en los dos sentidos.
- Veinte temas embebidos, temas JSON adicionales y diez idiomas.
- Ocho chromes: Ribbon, Palette, Mac, GNOME, KDE, Studio, Neon y Holo.
- Once niveles de zoom, miniatura, reglas, cuadrícula y pantalla completa.
- Impresión a la impresora predeterminada y vista previa en el visor nativo.

## Alcance

Lienzo se distribuye exclusivamente como aplicación nativa de escritorio para
macOS, Windows y Linux. La versión para navegador queda cancelada y no forma
parte de la planificación del proyecto.

El sitio oficial, <https://lienzo.surge.sh/>, presenta la aplicación y dirige
las descargas a GitHub Releases; no ejecuta el editor en el navegador.

## Protecciones de datos

Nuevo, Abrir, Abrir reciente, Salir y el botón de cierre del sistema muestran
Guardar / No guardar / Cancelar cuando hay cambios. Una acción pendiente sólo
continúa después de guardar correctamente o de elegir explícitamente no
guardar. Un error o cancelación de Guardar como conserva el documento y su ruta
anterior.

Crear o abrir un documento cancela selección, curva o polígono multietapa,
previsualización, texto flotante y arrastres del documento anterior.

## Distribución 0.1.3

| Sistema | Portable | Instalador |
|---|---|---|
| macOS ARM64 | ZIP con `Lienzo.app` | DMG |
| Windows x86_64 | ZIP con `Lienzo.exe` | Setup.exe por usuario |
| Linux x86_64 | AppImage | DEB para Debian/Ubuntu |

GitHub Actions genera los seis paquetes desde la etiqueta `v0.1.3` y los
publica con la versión en el nombre y su archivo `SHA256SUMS.txt`.

## Cambios preparados para 0.1.4

- La selección libre dibuja el marco animado sobre el recorrido real del lazo
  y calcula su caja usando todos sus puntos.
- Imprimir usa el servicio nativo y Vista previa abre el lienzo en el visor del
  sistema, con una alternativa segura si no hay servicio de impresión.
- Se eliminó el soporte web incompleto: Lienzo es una aplicación nativa
  descargable para los tres sistemas.
- README en inglés y español, política de seguridad, reportes privados y firma
  SSH de commits.
- Sitio oficial integrado en los README, el diálogo Acerca de y los metadatos
  de los paquetes de Windows y Linux.

## Cambios de 0.1.3

- La rueda usa el evento original en lugar del desplazamiento suavizado: una
  muesca cambia un nivel de zoom y no se repite durante varios cuadros.
- `Ctrl + rueda` funciona sobre toda el área de trabajo en los tres sistemas;
  macOS conserva además `Cmd + rueda` y el gesto de pellizcar.
- El instalador de Windows usa MUI2 y detecta si debe actualizar, reparar o
  advertir sobre una versión más reciente ya instalada.
- El nuevo logo oficial aparece en el README y en los paquetes de macOS,
  Windows y Linux; el ICO incluye siete resoluciones nativas.
- La barra y el diálogo Acerca de usan la marca oficial sin fondo, mientras la
  ventana del sistema usa el logo completo.
- Los veinte temas se muestran en galerías separadas por modo claro y oscuro.

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
| Linux | Compila para x86_64; falta probarlo en una distribución real |
| Firma y notarización | Faltan certificados comerciales de Apple y Microsoft |

## Arquitectura breve

`main.rs` controla ventana, archivos y diálogos; `ui.rs` dibuja los ocho
chromes; `doc.rs` gobierna herramientas y selección; `canvas.rs` conserva los
píxeles y el historial; `shapes.rs` rasteriza formas y pinceles; `text.rs`
rasteriza texto; `theme.rs` y `lang.rs` manejan apariencia e idioma.

El detalle de uso, instalación y decisiones técnicas vive en `README.md`.
