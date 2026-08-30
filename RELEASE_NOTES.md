# Lienzo 0.1.2

Esta versión corrige el zoom por rueda, mejora la presentación pública del
proyecto y renueva la instalación en Windows.

## Novedades

- `Ctrl + rueda` funciona en toda el área de trabajo de Windows, Linux y macOS;
  `Cmd + rueda` también funciona en macOS.
- Cada muesca cambia exactamente un nivel, sin repeticiones causadas por el
  suavizado del desplazamiento.
- El instalador de Windows estrena interfaz MUI2 nítida en pantallas HiDPI.
- El instalador detecta una versión anterior y ofrece actualizarla conservando
  las preferencias; una instalación de la misma versión se puede reparar.
- El ejecutable y el instalador muestran nombre, versión y editor en sus
  propiedades de Windows.
- Nuevo logo oficial en el README y en los paquetes de los tres sistemas; el
  icono de Windows incluye siete resoluciones nativas, de 16 a 256 px.
- La interfaz reemplaza la antigua paleta simplificada por la marca oficial sin
  fondo; la ventana nativa también muestra el logo de Lienzo.
- El README muestra por separado los diez temas claros y los diez oscuros.

## Descargas

- macOS Apple Silicon: ZIP portable o DMG.
- Windows x86_64: ZIP portable o instalador `Setup.exe` actualizable.
- Linux x86_64: AppImage portable o paquete DEB para Debian y Ubuntu.

Los binarios no tienen firma comercial, por lo que macOS Gatekeeper o Windows
SmartScreen pueden mostrar una advertencia. Use `SHA256SUMS.txt` para comprobar
la integridad de cada descarga.
