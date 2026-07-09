mod backup;
mod create;
mod frontmatter;
mod fs_cmds;
mod index;
mod jail;
mod plugins;
mod refs;
mod settings;
#[cfg(test)]
mod testutil;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            fs_cmds::list_root_md,
            fs_cmds::read_file,
            fs_cmds::write_file,
            fs_cmds::write_raw,
            fs_cmds::catalog,
            fs_cmds::scan_refs,
            settings::settings_get,
            settings::settings_set,
            plugins::plugins_list,
            plugins::plugin_set_enabled,
            plugins::plugin_install,
            plugins::plugin_remove,
            plugins::marketplace_add,
            create::create_entry,
            create::import_file,
            create::import_skill_dir,
            create::delete_entry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
