# Estado actual de Lienzo

La versión 0.1.6 es la última publicada y dispone de los seis paquetes y sus
sumas SHA-256 en GitHub Releases. La 0.1.7 está preparada localmente: su
ejecutable de Windows x86_64 se compiló y validó, pero todavía no se hizo commit
ni push. Por eso aún no existe el tag, el release ni los paquetes 0.1.7.

## Validación

```sh
cargo fmt --check
cargo +stable-x86_64-pc-windows-gnu test --locked --target-dir target-gnu
cargo +stable-x86_64-pc-windows-gnu clippy --locked --all-targets --target-dir target-gnu -- -D warnings
cargo +stable-x86_64-pc-windows-gnu build --release --locked --target-dir target-gnu
```

- La suite completa está aprobada, incluidas las regresiones del bloqueo de
  egui, los cursores de redimensionado y los límites del zoom configurable.
- Clippy sin advertencias tratadas como error.
- Formato de Rust verificado.
- El único ejecutable local, `target-gnu/release/lienzo.exe`, informa versión
  0.1.7.
- Probado correctamente en Windows x86_64, incluido el cambio repetido entre
  los temas Windows 7, 10 y 11.
- La validación visual de 0.1.7 sigue pendiente en Linux y macOS; el flujo de
  publicación los compila de forma aislada al subir la versión.
- DMG, ZIP portable, Setup.exe, AppImage y DEB están publicados y verificados
  para 0.1.6.

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
- Zoom editable de 12,5 % a 800 %, paso configurable y persistente, miniatura,
  reglas, cuadrícula, pantalla completa y vista Solo lienzo.
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

## Distribución 0.1.7

| Sistema | Portable | Instalador |
|---|---|---|
| macOS ARM64 | ZIP con `Lienzo.app` | DMG |
| Windows x86_64 | ZIP con `Lienzo.exe` | Setup.exe por usuario |
| Linux x86_64 | AppImage | DEB para Debian/Ubuntu |

Al hacer push a `main`, GitHub Actions detecta la versión de `Cargo.toml`.
Siempre ejecuta calidad; sólo si el tag todavía no existe genera los seis
paquetes y publica el tag y los archivos con su `SHA256SUMS.txt`. No se crea un
tag manual.

## Cambios listos para 0.1.7

- `Ctrl + V` detecta imágenes de forma fiable en Windows, incluso cuando el
  atajo procede de un botón programable del mouse.
- Pegar agranda cada eje del lienzo sólo cuando la imagen lo necesita y nunca
  reduce un documento mayor; la imagen queda como selección flotante en (0, 0).
- El paso de zoom se muestra en el botón `±25 %`. Su campo recibe foco y
  selecciona el valor completo al abrirse, admite teclado y no se cierra por
  hacer clic dentro; Backspace y `Ctrl + A` editan el valor. Sólo `Esc` o un
  clic exterior lo cierran.
- La ventana descarta tamaños persistidos inválidos y arranca con dimensiones
  útiles y un mínimo de 640 × 480.

## Cambios de 0.1.6

- Los tres tiradores de cambio de tamaño del lienzo anuncian su dirección con
  cursores horizontal, vertical o diagonal.
- Solo lienzo permite trabajar sin cinta, barras ni miniatura y se cierra con
  `Esc`; pantalla completa permanece como una opción independiente.
- La miniatura elimina el pozo fijo que causaba franjas vacías y se integra con
  los colores, bordes y proporción del dibujo actual.
- Las nueve interfaces traducidas incluyen el nuevo control Solo lienzo.
- El porcentaje de zoom se puede editar directamente y el paso de `− / +` se
  configura y conserva en las preferencias; funciona entre 12,5 % y 800 %.
- Acerca de Lienzo muestra la versión compilada desde `Cargo.toml` tanto bajo
  el logo como en la ficha de información.

## Cambios de 0.1.4

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
