fn main() {
    #[cfg(windows)]
    winresource::WindowsResource::new()
        .set_icon("packaging/windows/lienzo.ico")
        .compile()
        .expect("no se pudo incrustar el icono de Windows");
}
