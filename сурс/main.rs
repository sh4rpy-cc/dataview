use eframe::egui;
use std::sync::Arc;
use tokio::runtime::Runtime;
use std::fs::File;
use std::io::Write;

mod db;
use db::{DbType, DbHandler, TableData};
use db::sqlite::SqliteHandler;
use db::postgres::PostgresHandler;
use db::mysql::MysqlHandler;

struct DbBrowserApp {
    db_path: String,
    db_type: DbType,
    handler: Option<Box<dyn DbHandler>>,
    tables: Vec<String>,
    selected_table: Option<String>,
    table_data: TableData,
    sorted_col: Option<String>,
    sort_asc: bool,
    error_message: Option<String>,
    runtime: Arc<Runtime>,
}

impl DbBrowserApp {
    fn new() -> Self {
        Self {
            db_path: String::new(),
            db_type: DbType::Sqlite,
            handler: None,
            tables: Vec::new(),
            selected_table: None,
            table_data: TableData { columns: Vec::new(), rows: Vec::new() },
            sorted_col: None,
            sort_asc: true,
            error_message: None,
            runtime: Arc::new(Runtime::new().unwrap()),
        }
    }

    fn connect(&mut self) {
        let result = self.runtime.block_on(async {
            match self.db_type {
                DbType::Sqlite => {
                    let pool = sqlx::sqlite::SqlitePool::connect(&self.db_path)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(Box::new(SqliteHandler { pool }) as Box<dyn DbHandler>)
                }
                DbType::Postgres => {
                    let pool = sqlx::postgres::PgPool::connect(&self.db_path)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(Box::new(PostgresHandler { pool }) as Box<dyn DbHandler>)
                }
                DbType::Mysql => {
                    let pool = sqlx::mysql::MySqlPool::connect(&self.db_path)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(Box::new(MysqlHandler { pool }) as Box<dyn DbHandler>)
                }
            }
        });

        match result {
            Ok(handler) => {
                self.handler = Some(handler);
                self.error_message = None;
                self.load_tables(); // Автоматическая загрузка списка таблиц
            }
            Err(e) => {
                self.error_message = Some(e);
                self.handler = None;
                self.tables.clear();
            }
        }
    }

    fn load_tables(&mut self) {
        if let Some(handler) = &self.handler {
            match handler.get_tables(&self.runtime) {
                Ok(tables) => {
                    self.tables = tables;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e);
                }
            }
        }
    }

    fn load_table_data(&mut self, table_name: &str, sort_col: Option<String>, sort_asc: bool) {
        if let Some(handler) = &self.handler {
            match handler.get_table_data(table_name, &self.runtime) {
                Ok(mut data) => {
                    if let Some(sort_col_name) = &sort_col {
                        if let Some(col_index) = data.columns.iter().position(|c| c == sort_col_name) {
                            data.rows.sort_by(|a, b| {
                                let a_val = &a[col_index];
                                let b_val = &b[col_index];
                                if sort_asc { a_val.cmp(b_val) } else { b_val.cmp(a_val) }
                            });
                        }
                    }
                    self.table_data = data;
                    self.sorted_col = sort_col;
                    self.sort_asc = sort_asc;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(e);
                }
            }
        }
    }

    fn export_to_csv(&self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("export.csv")
            .add_filter("CSV", &["csv"])
            .save_file()
        {
            let mut file = File::create(path).unwrap();
            let header = self.table_data.columns.join(",");
            writeln!(file, "{}", header).unwrap();
            for row in &self.table_data.rows {
                let line = row.join(",");
                writeln!(file, "{}", line).unwrap();
            }
        }
    }
}

impl eframe::App for DbBrowserApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("DB Browser Lite");

            ui.horizontal(|ui| {
                ui.label("Тип БД:");
                egui::ComboBox::from_label("")
                    .selected_text(match self.db_type {
                        DbType::Sqlite => "SQLite",
                        DbType::Postgres => "PostgreSQL",
                        DbType::Mysql => "MySQL",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.db_type, DbType::Sqlite, "SQLite");
                        ui.selectable_value(&mut self.db_type, DbType::Postgres, "PostgreSQL");
                        ui.selectable_value(&mut self.db_type, DbType::Mysql, "MySQL");
                    });
            });

            ui.horizontal(|ui| {
                ui.label("Путь/строка подключения:");
                ui.text_edit_singleline(&mut self.db_path);
                if ui.button("Подключиться").clicked() {
                    if !self.db_path.is_empty() {
                        self.connect();
                    }
                }
            });

            if let Some(msg) = &self.error_message {
                ui.colored_label(egui::Color32::RED, msg);
            }

            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Таблицы:");
                if ui.button("Обновить").clicked() && self.handler.is_some() {
                    self.load_tables();
                }
            });

            let current_table = self.selected_table.clone().unwrap_or_default();
            let mut selected_table_name = current_table.clone();

            egui::ComboBox::from_label("Выберите таблицу")
                .selected_text(if self.tables.is_empty() {
                    "Таблицы не найдены".to_string()
                } else {
                    selected_table_name.clone()
                })
                .show_ui(ui, |ui| {
                    if self.tables.is_empty() {
                        ui.label("(база пуста или ошибка)");
                    } else {
                        for table in &self.tables {
                            ui.selectable_value(&mut selected_table_name, table.clone(), table);
                        }
                    }
                });

            if selected_table_name != current_table && !selected_table_name.is_empty() {
                self.selected_table = Some(selected_table_name.clone());
                self.load_table_data(&selected_table_name, None, true);
            }

            ui.separator();

            // ===== ИСПРАВЛЕННЫЙ БЛОК ОТРИСОВКИ ДАННЫХ =====
            if let Some(table_name) = self.selected_table.clone() {
                ui.label(format!("Таблица: {}", table_name));

                if ui.button("📥 Экспорт в CSV").clicked() {
                    self.export_to_csv();
                }

                if !self.table_data.columns.is_empty() {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("table_grid").show(ui, |ui| {
                            let columns = self.table_data.columns.clone();
                            let current_sorted_col = self.sorted_col.clone();
                            let current_sort_asc = self.sort_asc;
                            
                            for col in columns {
                                let is_sorted = Some(col.clone()) == current_sorted_col;
                                let sort_label = if is_sorted && current_sort_asc {
                                    format!("{} ▲", col)
                                } else if is_sorted && !current_sort_asc {
                                    format!("{} ▼", col)
                                } else {
                                    col.clone()
                                };
                                let btn = ui.button(egui::RichText::new(sort_label).strong().color(egui::Color32::GOLD));
                                if btn.clicked() {
                                    let new_sort_asc = if Some(col.clone()) == current_sorted_col {
                                        !current_sort_asc
                                    } else {
                                        true
                                    };
                                    self.load_table_data(&table_name, Some(col.clone()), new_sort_asc);
                                }
                                ui.end_row();
                            }

                            for row in &self.table_data.rows {
                                for value in row {
                                    ui.label(value);
                                }
                                ui.end_row();
                            }
                        });
                    });
                } else {
                    ui.label("Таблица пуста или не загружена");
                }
            }

            ui.separator();

            ui.horizontal(|ui| {
                if self.handler.is_some() {
                    ui.colored_label(egui::Color32::GREEN, "✅ Подключено");
                } else {
                    ui.colored_label(egui::Color32::RED, "❌ Не подключено");
                }
            });
        });
    }
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([900.0, 700.0])
            .with_title("DB Browser Lite / sh4rpy"),
        ..Default::default()
    };

    eframe::run_native(
        "DB Browser Lite / sh4rpy",
        options,
        Box::new(|_cc| Ok(Box::new(DbBrowserApp::new()))),
    )
}