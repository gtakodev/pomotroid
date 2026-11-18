fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=build/icons/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("build/icons/icon.ico")
            .set("ProductName", "Pomotroid")
            .set("FileDescription", "Pomotroid")
            .compile()
            .expect("embed the Pomotroid icon in the Windows executable");
    }
}
