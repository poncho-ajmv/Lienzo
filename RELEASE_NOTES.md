# Lienzo 0.1.4

Esta versión completa la selección libre y conecta impresión y vista previa
con los servicios nativos de cada sistema operativo.

## Novedades

- El marco animado de la selección libre sigue el lazo dibujado, incluso
  después de moverlo o cambiar su tamaño.
- La caja de la selección libre abarca todo el recorrido y no sólo sus puntos
  inicial y final.
- Imprimir envía el lienzo a la impresora predeterminada del sistema.
- Vista previa abre una copia exacta en el visor nativo; si no hay servicio de
  impresión, ese visor funciona también como alternativa segura.
- Lienzo queda definido exclusivamente como aplicación nativa descargable para
  macOS, Windows y Linux; se eliminó el soporte web incompleto.
- README principal en inglés, traducción española separada y política de
  seguridad con reportes privados habilitados.
- El sitio oficial, <https://lienzo.surge.sh/>, queda enlazado desde los README,
  el diálogo Acerca de y los metadatos de los instaladores.

## Descargas

- macOS Apple Silicon: ZIP portable o DMG.
- Windows x86_64: ZIP portable o instalador `Setup.exe` actualizable.
- Linux x86_64: AppImage portable o paquete DEB para Debian y Ubuntu.

Los binarios no tienen firma comercial, por lo que macOS Gatekeeper o Windows
SmartScreen pueden mostrar una advertencia. Use `SHA256SUMS.txt` para comprobar
la integridad de cada descarga.
