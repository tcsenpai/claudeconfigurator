mod appconfig;
mod backup;
mod bundle;
mod create;
mod frontmatter;
mod fs_cmds;
mod graph;
mod index;
mod jail;
mod mcp;
mod plugins;
mod refs;
mod scope;
mod settings;
#[cfg(test)]
mod testutil;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            backup::backup_list,
            backup::backup_read,
            backup::backup_restore,
            fs_cmds::list_root_md,
            fs_cmds::read_file,
            fs_cmds::write_file,
            fs_cmds::write_raw,
            fs_cmds::catalog,
            fs_cmds::scan_refs,
            settings::settings_get,
            settings::settings_set,
            settings::settings_schema,
            plugins::plugins_list,
            plugins::plugin_set_enabled,
            plugins::plugin_install,
            plugins::plugin_remove,
            plugins::marketplace_add,
            create::create_entry,
            create::import_file,
            create::import_skill_dir,
            create::delete_entry,
            graph::graph_data,
            appconfig::app_config_get,
            appconfig::app_config_set,
            mcp::mcp_list,
            mcp::mcp_upsert,
            mcp::mcp_remove,
            mcp::mcp_set_enabled,
            bundle::bundle_scan_secrets,
            bundle::bundle_export,
            bundle::bundle_preview,
            bundle::bundle_restore,
            scope::scope_get,
            scope::scope_set_global,
            scope::scope_open_project,
            scope::scope_create_claude,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
