use sqlx::{sqlite::SqlitePool, Row};
use std::sync::Arc;
use tokio::runtime::Runtime;
use crate::db::traits::{DbHandler, TableData};

pub struct SqliteHandler {
    pub pool: SqlitePool,
}

impl DbHandler for SqliteHandler {
    fn get_tables(&self, runtime: &Arc<Runtime>) -> Result<Vec<String>, String> {
        runtime.block_on(async {
            let rows = sqlx::query("SELECT name FROM sqlite_master WHERE type='table'")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(rows.iter().map(|r| r.get(0)).collect())
        })
    }

    fn get_table_data(&self, table_name: &str, runtime: &Arc<Runtime>) -> Result<TableData, String> {
        runtime.block_on(async {
            // Формируем строку для PRAGMA
            let pragma_query = format!("PRAGMA table_info('{}')", table_name);
            let cols = sqlx::query(&pragma_query)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            
            // ✅ ИСПРАВЛЕНИЕ: Берем колонку с индексом 1 (name), а не 0 (cid)
            let columns: Vec<String> = cols.iter().map(|r| r.get(1)).collect();

            // Основной запрос данных
            let query_str = format!("SELECT * FROM {}", table_name);
            let rows = sqlx::query(&query_str)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

            let mut data = Vec::new();
            for row in rows {
                let mut row_data = Vec::new();
                for col in &columns {
                    let value: String = row.try_get(col.as_str()).unwrap_or_else(|_| "NULL".to_string());
                    row_data.push(value);
                }
                data.push(row_data);
            }

            Ok(TableData { columns, rows: data })
        })
    }
}