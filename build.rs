fn main() {
    #[cfg(windows)]
    {
        let mut r = winresource::WindowsResource::new();
        r.set_icon("assets/icon.ico");
        r.set("ProductName", "PDF Reader CP");
        r.set("FileDescription", "PDF Reader CP");
        r.compile().unwrap();
    }
}
