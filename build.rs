fn main() {
    #[cfg(windows)]
    {
        let version = env!("CARGO_PKG_VERSION");
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("CompanyName", "AigoBored");
        res.set(
            "FileDescription",
            "Windows system tray battery monitor for the Attack Shark R11 Ultra.",
        );
        res.set("ProductName", "R11 Ultra Battery Tracker");
        res.set("FileVersion", version);
        res.set("ProductVersion", version);
        res.set("LegalCopyright", "Copyright (C) AigoBored");
        res.set("OriginalFilename", "R11UltraBattery.exe");
        if let Err(err) = res.compile() {
            panic!("failed to embed Windows resources: {err}");
        }
    }
}
