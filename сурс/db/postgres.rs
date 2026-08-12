use sqlx::{postgres::PgPool, Row};
use std::sync::Arc;
use tokio::runtime::Runtime;
use crate::db::traits::{DbHandler, TableData};

pub struct PostgresHandler {
    pub pool: PgPool,
}

impl DbHandler for PostgresHandler {
    fn get_tables(&self, runtime: &Arc<Runtime>) -> Result<Vec<String>, String> {
        runtime.block_on(async {
            let rows = sqlx::query("SELECT table_name FROM information_schema.tables WHERE table_schema='public'")
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            Ok(rows.iter().map(|r| r.get(0)).collect())
        })
    }

    fn get_table_data(&self, table_name: &str, runtime: &Arc<Runtime>) -> Result<TableData, String> {
        runtime.block_on(async {
            // Получаем список колонок (параметр $1 работает корректно)
            let cols = sqlx::query("SELECT column_name FROM information_schema.columns WHERE table_name=$1")
                .bind(table_name)
                .fetch_all(&self.pool)
                .await
                .map_err(|e| e.to_string())?;
            let columns: Vec<String> = cols.iter().map(|r| r.get(0)).collect();

            // ✅ ИСПРАВЛЕНИЕ: Имя таблицы подставляем через format! напрямую в строку
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