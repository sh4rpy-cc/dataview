// Объявляем публичные модули
pub mod sqlite;
pub mod postgres;
pub mod mysql;

// Импортируем трейты и типы в общее пространство
pub use crate::db::traits::{DbType, DbHandler, TableData};

// Модуль с общими типами (встроен прямо сюда для упрощения)
pub mod traits {
    use std::sync::Arc;
    use tokio::runtime::Runtime;

    #[derive(Clone, Debug, PartialEq)]
    pub enum DbType {
        Sqlite,
        Postgres,
        Mysql,
    }

    pub struct TableData {
        pub columns: Vec<String>,
        pub rows: Vec<Vec<String>>,
    }

    pub trait DbHandler: Send + Sync {
        fn get_tables(&self, runtime: &Arc<Runtime>) -> Result<Vec<String>, String>;
        fn get_table_data(&self, table_name: &str, runtime: &Arc<Runtime>) -> Result<TableData, String>;
    }
}