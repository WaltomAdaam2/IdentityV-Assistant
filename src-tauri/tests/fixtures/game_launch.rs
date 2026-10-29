fn main() {
    let executable = std::env::current_exe().unwrap();
    let evidence = format!("{}\n{}\n{}\n{}\n{}\n",
        executable.display(), std::env::current_dir().unwrap().display(),
        std::env::var("HTTP_PROXY").unwrap_or_default(),
        std::env::var("HTTPS_PROXY").unwrap_or_default(),
        std::env::args().skip(1).collect::<Vec<_>>().join("\n"));
    let pending = executable.with_file_name("launch-evidence.pending");
    std::fs::write(&pending, evidence).unwrap();
    std::fs::rename(pending, executable.with_file_name("launch-evidence.txt")).unwrap();
}
