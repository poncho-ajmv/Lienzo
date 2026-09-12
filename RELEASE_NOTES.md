# Lienzo 0.1.6 — preparada localmente

Esta versión mejora la interacción al cambiar el tamaño del lienzo y ofrece una
vista de trabajo más limpia sin sacrificar los controles habituales. Está
validada localmente en Windows x86_64 y lista para publicar; 0.1.5 sigue siendo
la versión descargable hasta que se haga push a `main`.

## Novedades

- Los tiradores de tamaño del lienzo muestran cursores horizontal, vertical o
  diagonal según el eje que se va a modificar.
- La nueva vista **Solo lienzo** oculta temporalmente la cinta, barras y
  miniatura. `Esc` devuelve la interfaz completa; funciona junto a pantalla
  completa sin obligarla.
- La miniatura adapta su tarjeta a la proporción de cada imagen, elimina las
  bandas vacías y emplea los colores y bordes del tema activo.
- El porcentaje de zoom se puede editar directamente entre 12,5 % y 800 %; el
  engranaje define y guarda el salto que usan `− / +`, la lupa y los atajos.
- Se añadieron traducciones de Solo lienzo, Paso de zoom y Restablecer para las
  nueve interfaces traducidas.
- Las pruebas cubren los tres cursores de los tiradores y los límites del zoom
  configurable para evitar regresiones.
- **Acerca de Lienzo** muestra de forma destacada la versión compilada, tomada
  directamente de `Cargo.toml` para que siempre coincida con el ejecutable.

## Publicación prevista

- macOS Apple Silicon: ZIP portable y DMG.
- Windows x86_64: ZIP portable e instalador `Setup.exe` actualizable.
- Linux x86_64: AppImage portable y paquete DEB para Debian y Ubuntu.

Al publicar el cambio de versión en `main`, GitHub Actions validará el código,
creará estos paquetes, el tag `v0.1.6`, el release y `SHA256SUMS.txt`. Los
binarios no tienen firma comercial, por lo que macOS Gatekeeper o Windows
SmartScreen pueden mostrar una advertencia. Use `SHA256SUMS.txt` para comprobar
la integridad de cada descarga.
