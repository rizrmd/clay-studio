pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_tables;
mod m20250824_225443_create_sessions_table;
mod m20250824_create_file_uploads_table;
mod m20250825_000001_add_forgotten_after;
mod m20250824_234033_add_forgotten_message_field;
mod m20250825_085605_add_client_id_to_projects;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_tables::Migration),
            Box::new(m20250824_225443_create_sessions_table::Migration),
            Box::new(m20250825_000001_add_forgotten_after::Migration),
            Box::new(m20250824_234033_add_forgotten_message_field::Migration),
            Box::new(m20250825_085605_add_client_id_to_projects::Migration),
        ]
    }
}