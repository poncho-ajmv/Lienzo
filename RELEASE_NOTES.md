# Lienzo 0.1.7

Esta versión refuerza las acciones cotidianas de pegar imágenes y ajustar el
zoom. Está publicada para Windows x86_64, macOS ARM64 y Linux x86_64, y fue
validada localmente en Windows x86_64.

## Novedades

- `Ctrl + V` pega imágenes de forma fiable en Windows, incluso desde botones de
  mouse que envían el atajo rápidamente; los otros sistemas conservan el evento
  de teclado nativo.
- Una imagen pegada agranda el lienzo cuando hace falta, pero nunca reduce el
  trabajo existente; queda flotante desde la esquina superior izquierda.
- Al arrastrar una manija de selección a través del borde opuesto, la imagen se
  refleja automáticamente sobre ese eje. Cruzarla de regreso restaura la
  orientación original.
- Voltear horizontal o verticalmente transforma la selección activa; sólo actúa
  sobre el lienzo completo cuando no hay una selección.
- El botón `±25 %` muestra el paso de zoom actual. Al abrirlo, el valor ya queda
  enfocado y seleccionado para reemplazarlo escribiendo; los clics internos no
  cierran el panel y Backspace o `Ctrl + A` editan normalmente. Sólo `Esc` o un
  clic fuera lo cierran.
- Lienzo ignora geometrías de ventana inválidas guardadas por versiones
  anteriores y vuelve a abrir con un tamaño útil mínimo.
- Las pruebas cubren el crecimiento del lienzo, el reflejo reversible de
  selecciones, la entrada del paso de zoom y los límites configurables para
  evitar regresiones.

## Paquetes

- macOS Apple Silicon: ZIP portable y DMG.
- Windows x86_64: ZIP portable e instalador `Setup.exe` actualizable.
- Linux x86_64: AppImage portable y paquete DEB para Debian y Ubuntu.

GitHub Actions valida el código y genera estos paquetes, el tag `v0.1.7`, el
release y `SHA256SUMS.txt`. Los binarios no tienen firma comercial, por lo que
macOS Gatekeeper o Windows SmartScreen pueden mostrar una advertencia. Use
`SHA256SUMS.txt` para comprobar la integridad de cada descarga.
