# Lienzo 0.1.8

Esta versión corrige dos casos que podían alterar el dibujo durante el uso
normal de selección y portapapeles.

## Correcciones

- Pegar en otra aplicación ya no pega también dentro de Lienzo cuando su ventana
  perdió el foco.
- Mover una selección hasta el borde mantiene todos sus píxeles dentro del
  lienzo y evita recortarlos al confirmar.
- Se agregaron pruebas de regresión para ambos casos; las 36 pruebas pasan.

## Paquetes

GitHub Actions valida el código y genera los paquetes de Windows x86_64, macOS
ARM64 y Linux x86_64, junto con el tag `v0.1.8`, el release y
`SHA256SUMS.txt`. Los binarios no tienen firma comercial, por lo que macOS
Gatekeeper o Windows SmartScreen pueden mostrar una advertencia. Use
`SHA256SUMS.txt` para comprobar la integridad de cada descarga.
