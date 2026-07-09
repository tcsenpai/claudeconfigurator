mod backup;
mod frontmatter;
mod fs_cmds;
mod index;
mod jail;
mod refs;
#[cfg(test)]
mod testutil;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            fs_cmds::list_root_md,
            fs_cmds::read_file,
            fs_cmds::write_file,
            fs_cmds::write_raw,
            fs_cmds::catalog,
            fs_cmds::scan_refs,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
