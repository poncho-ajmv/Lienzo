# Lienzo 0.1.5

Esta versión corrige el bloqueo de la interfaz que podía aparecer al cambiar a
un tema Windows o al abrir el diálogo Acerca de.

## Novedades

- El icono de Lienzo se carga fuera del acceso exclusivo a la memoria de egui,
  evitando el bloqueo reentrante que dejaba la ventana sin responder.
- La memoria visual transitoria de egui ya no se conserva entre actualizaciones;
  tema, idioma y colores siguen guardándose normalmente.
- Una prueba de regresión detecta si la carga o reutilización del icono vuelve a
  bloquear la interfaz.
- El instalador de Windows se genera con NSIS 3.12.0 y se instala en modo
  silencioso como prueba antes de publicarse.
- macOS verifica el `Info.plist`, la firma ad hoc, el ZIP y el DMG.
- Linux valida el archivo de escritorio y la estructura del AppImage y del DEB.

## Descargas

- macOS Apple Silicon: ZIP portable o DMG.
- Windows x86_64: ZIP portable o instalador `Setup.exe` actualizable.
- Linux x86_64: AppImage portable o paquete DEB para Debian y Ubuntu.

Los binarios no tienen firma comercial, por lo que macOS Gatekeeper o Windows
SmartScreen pueden mostrar una advertencia. Use `SHA256SUMS.txt` para comprobar
la integridad de cada descarga.
