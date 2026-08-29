# Estado — lo que hay que saber para seguir

Escrito al terminar la primera sesión de construcción. Léelo antes de tocar nada.

---

## Segunda pasada de revisión — lo que se corrigió

Tres revisiones independientes leyeron todo el código contra el fuente de egui.
Encontraron y se arreglaron:

**Un error de compilación real.** `r#"{"accent":"#ff0000"}"#` — la cadena cruda
termina en el `"#` de `"#ff0000"`, no al final. `cargo test` no compilaba.

**Cinco panics garantizados**, todos por indexar fuera de rango:

- El borrador con clic derecho cerca del borde inferior. El rectángulo sucio se
  marcaba sin recortar y después `extract` leía pasado el final del buffer.
  Arreglado en `mark()`, que es por donde pasan todos.
- Rotar o redimensionar con un trazo abierto: `scratch` quedaba con el ancho
  viejo y el historial leía con el stride equivocado.
- Recortar una selección flotante, o pegar una imagen más grande que el lienzo.
- `fill_polygon` sin acotar: arrastrar una forma muy afuera daba un bucle de
  miles de millones de filas — la aplicación se colgaba, no fallaba.

**Un test que ya estaba en rojo.** El corazón se salía un 6% de su caja porque
los divisores del paramétrico estaban mal.

**Ocho bugs de comportamiento**, de los cuales tres eran serios:

- **Cada trazo perdía su comienzo.** Con `Sense::click_and_drag`, egui posterga
  la decisión de clic-o-arrastre hasta que el cursor se mueve unos 6 px. Para
  una app de dibujo es inaceptable. Ahora se lee el estado crudo del puntero.
- **Borrar mientras escribías** en un campo numérico de un diálogo ejecutaba
  Borrar sobre el dibujo. Faltaba el guardia de foco.
- **Cambiar de herramienta a mitad de una curva** dejaba el estado armado: el
  primer clic del lápiz dibujaba una curva.

Y: los seis temas heredaban el `Style` **oscuro** de egui; `override_text_color`
anulaba todos los colores de texto por estado; las etiquetas de la cinta se
dibujaban encima de los botones y los separadores salían invertidos, porque
`ui.max_rect()` dentro de un `horizontal()` mide 18 px y no el alto del panel;
el diálogo Propiedades re-leía el ancho cada frame, así que no se podía cambiar;
la nube y la llamada ovalada se dibujaban cruzadas por la mitad.

**Se borraron** ~90 líneas: `Cmd` que nadie manejaba, campos escritos y nunca
leídos, los cuatro menús que eran la misma función, y el bloque de la paleta que
estaba copiado textual en los tres chromes.

Hay **18 tests** ahora, dos nuevos: que mover una selección deje *un* paso de
historial, y que ninguna forma cerrada se cruce a sí misma — el test anterior
sólo miraba que los puntos estuvieran dentro de la caja, y eso no dice nada del
orden, que en un polígono es la forma.

---

## Lo primero: esto no se compiló nunca

**No pude compilar el código.** Mi entorno no tiene Rust y todos los servidores
de distribución (`static.rust-lang.org`, conda, los espejos) están bloqueados
desde ahí. Lo único que corrió de verdad fue el motor de la primera entrega, en
tu máquina, y pasó.

Lo que sí hice, que es mucho mejor que adivinar: **leer el fuente de egui 0.36.1
que clonaste en `~/Desktop/egui`** y verificar contra él cada llamada. Eso
encontró dos cosas que habría escrito mal con total seguridad:

1. **`egui::TopBottomPanel` y `egui::SidePanel` ya no existen.** Se unificaron en
   un solo `egui::Panel` con `top()`, `bottom()`, `left()`, `right()`. Siete
   llamadas mías estaban mal.
2. **`Context::set_style` no existe.** egui guarda un estilo para modo claro y
   otro para oscuro; hay que usar `all_styles_mut`.

Además la firma del `trait App` cambió en la 0.34: es `fn ui(&mut self, ui, frame)`,
no `fn update`. Todo tutorial que encuentres en internet está desactualizado.

**Primer paso cuando vuelvas:**

```sh
cd ~/Desktop/lienzo
cargo test      # el motor: 16 tests que no necesitan ventana
cargo run       # la aplicación
```

Si algo no compila, pegame el error. Los tres módulos de abajo (`canvas`,
`shapes`, `doc`) no tocan egui, así que `cargo test` debería pasar aunque la
interfaz tenga problemas.

---

## Qué hay construido

```
src/canvas.rs   670 líneas   píxeles, historial por rectángulo, relleno,
                             rotar/voltear/escalar/sesgar          4 tests
src/shapes.rs   639 líneas   23 formas, 9 pinceles, 2 rasterizadores  3 tests
src/doc.rs      804 líneas   herramientas, selección, portapapeles    5 tests
src/theme.rs    405 líneas   temas desde JSON                         4 tests
src/ui.rs       811 líneas   los tres chromes y los widgets
src/main.rs     883 líneas   ventana, textura, teclado, archivos, diálogos
themes/*.json     6 archivos Win10, Win11, Win7, XP, Linux, macOS
```

**16 tests, todos sobre las capas que no dependen de egui.**

### Del Paint de Windows 10

Herramientas: lápiz, relleno, texto, borrador, selector de color, lupa, los
9 pinceles, las 23 formas, selección rectangular y de forma libre.

Detalles que suelen pasarse por alto y sí están:

- **Borrador selectivo con clic derecho** — sólo reemplaza el Color 1 por el 2
- **Relleno con tolerancia cero**, y por eso lápiz y contornos sin antialiasing
- **Mover la selección deja el Color 2 atrás**, no blanco
- **Selección transparente** como toggle global, sólo composición (Paint no
  guarda canal alfa, y este tampoco)
- **La curva en 3 fases** y el **polígono con doble clic para cerrar**
- **Shift** para círculo/cuadrado perfecto y líneas a 45°
- Los 11 niveles de zoom exactos, los 20 colores de la paleta al hex, 1152×648
  de lienzo por defecto

Archivo: abrir y guardar PNG, JPEG, BMP, GIF, TIFF (y abrir ICO).
Portapapeles: copiar y **pegar imágenes desde otras aplicaciones**.

### Las tres decisiones que sostienen el diseño

**El historial guarda rectángulos, no lienzos.** Un trazo típico cuesta ~50 KB
en vez de 2,9 MB. Ese mismo rectángulo alimenta la subida parcial a la GPU: una
sola cuenta sirve para las dos cosas. Es obligatorio, no una optimización — una
subida completa de textura **asigna una textura de GPU nueva en cada llamada**.

**Ni un archivo de iconos.** Cada forma dibuja su propio icono con la misma
lista de puntos que usa en el lienzo, y los iconos de herramienta son trazos
vectoriales que toman el color del tema.

**Seis temas, tres layouts.** El tema es un JSON con ~30 tokens y elige uno de
tres chromes. Win10/Win11/Win7 comparten `ribbon`, XP usa `palette`, macOS y
Linux usan `unified`.

---

## Lo que falta o está a medias

| Qué | Estado |
|---|---|
| **Herramienta de texto** | El cuadro flotante funciona y va sobre un `TextEdit` de verdad (obligatorio: sin eso macOS no entrega teclas muertas y mueren las tildes). Pero **el volcado al lienzo dibuja puntos, no letras** — falta medir las galerías de egui y volcar la máscara. Es lo primero que arreglaría. |
| **Imprimir** | Nada. El plan es armar un PDF con `printpdf` y entregárselo al sistema. |
| **Reglas** | El comando existe, el dibujo no. |
| **Web (WASM)** | Compila en teoría; abrir/guardar y pegar están sin implementar. |
| **Empaquetado** | Sin hacer. `cargo-packager` cuando toque. Ojo: notarizar en macOS cuesta 99 USD al año y no es opcional. |
| **Selección de forma libre** | Recorta bien al levantar, pero el marco que se dibuja es el rectángulo que la contiene. |

---

## Lo más probable que necesite arreglo

En orden de sospecha. Todo esto compila *en mi lectura*, pero no lo vio un
compilador:

1. **`rfd` y `arboard`** — sus APIs no las pude verificar contra fuente, no
   estaban en el clon. Si algo falla ahí, está aislado en `main.rs` entre
   `open_file`, `save_to`, `copy_to_system` y `paste_from_system`.
2. **La cinta de `ui.rs`** — la geometría es fija a propósito (ver abajo), pero
   los grupos pueden quedar mal espaciados hasta ajustar números a ojo.
3. **`theme.rs::to_style()`** — verifiqué campo por campo contra
   `egui/src/style.rs`, pero es donde más nombres hay juntos.

## Por qué la cinta tiene la geometría hardcodeada

No es pereza. El [issue #4378 de egui](https://github.com/emilk/egui/issues/4378)
lo abrió el propio autor en 2024 y sigue abierto: en modo inmediato no se sabe
el tamaño de un grupo antes de dibujarlo, así que todo lo que se acomoda solo
parpadea un frame.

**Nadie construyó nunca una cinta tipo Office en egui.** Búsqueda en todo el
repositorio: cero resultados. La única en modo inmediato que existe en el mundo
es [MeshLib](https://github.com/MeshInspector/MeshLib), en C++, y su código
hardcodea `currentTopPanelHeight_ = 113` y precalcula la configuración de cada
grupo. Copiamos ese molde. La línea `ui.set_min_height(banda)` al principio de
cada fila arregla sola tres de los seis problemas clásicos de layout.

## Prueba pendiente de la fase 0

Sigue sin hacerse, y son 5 minutos:

```sh
cd ~/Desktop/egui && cargo run -p egui_demo_app
```

En cualquier caja de texto, teclado español: `ñ ¿ ¡` y después **dos tildes
seguidas** (`áé`). Que entre una sola es falso positivo — la firma del bug era
que la segunda desaparecía. Lo arregló el PR #7983, que está en la 0.35, pero
nadie lo confirmó públicamente sobre una versión publicada.

---

## Antes de publicarlo

Poné tu nombre en `LICENSE`, donde dice `<TU NOMBRE AQUÍ>`.
