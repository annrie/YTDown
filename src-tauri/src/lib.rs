mod commands;
mod db;
mod state;
mod ytdlp;
mod images;
mod scheduler;

use state::AppState;
use tauri::{Listener, Manager};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data dir");
            let conn = db::init_db(&app_data_dir)
                .expect("Failed to initialize database");

            let mut sched = tauri::async_runtime::block_on(
                tokio_cron_scheduler::JobScheduler::new()
            )
            .expect("Failed to create job scheduler");

            // スケジューラをセットアップ完了前に起動して競合を防ぐ
            tauri::async_runtime::block_on(sched.start())
                .expect("Failed to start scheduler");

            app.manage(AppState::new(conn, sched));

            // 既存ジョブ登録・スキップ判定
            let app_handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                // 全ジョブ登録
                scheduler::register_all_jobs(&app_handle).await;
                // 起動時スキップ判定
                scheduler::check_overdue_schedules(&app_handle).await;

                // スリープ復帰時のスキップ判定: window focus イベントをリッスン
                let app_focus = app_handle.clone();
                app_handle.listen("tauri://focus", move |_| {
                    let app = app_focus.clone();
                    tauri::async_runtime::spawn(async move {
                        scheduler::check_overdue_schedules(&app).await;
                    });
                });
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // yt-dlp management
            commands::ytdlp_mgmt::get_ytdlp_info,
            commands::ytdlp_mgmt::check_ytdlp_update,
            commands::ytdlp_mgmt::update_ytdlp,
            commands::ytdlp_mgmt::install_ytdlp,
            // Formats
            commands::formats::fetch_formats,
            // Download engine
            commands::download::start_download,
            commands::download::fetch_playlist_items,
            commands::download::cancel_download,
            commands::download::pause_download,
            commands::download::resume_download,
            // File operations
            commands::file_ops::move_file,
            commands::file_ops::delete_file,
            commands::file_ops::reveal_in_finder,
            // Cookies
            commands::cookies::import_cookies_from_browser,
            commands::cookies::set_cookie_file,
            commands::cookies::check_safari_access,
            // Library
            commands::library::list_library,
            commands::library::search_library,
            commands::library::toggle_favorite,
            commands::library::get_download,
            // Settings
            commands::settings::set_ytdlp_path,
            commands::settings::get_all_settings,
            commands::settings::get_setting,
            commands::settings::set_setting,
            // Browser
            commands::browser::get_browser_url,
            // URL History
            commands::history::save_url_history,
            commands::history::get_url_history,
            commands::history::clear_url_history,
            // Images
            commands::images::scrape_images,
            commands::images::download_images,
            commands::images::list_image_sessions,
            commands::images::list_session_images,
            commands::images::delete_image_session,
            // Schedules
            commands::schedules::create_schedule,
            commands::schedules::update_schedule,
            commands::schedules::delete_schedule,
            commands::schedules::toggle_schedule,
            commands::schedules::list_schedules,
            commands::schedules::get_schedule,
            commands::schedules::run_schedule_now,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
