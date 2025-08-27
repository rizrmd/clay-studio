pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_tables;
mod m20250824_225443_create_sessions_table;
mod m20250824_create_file_uploads_table;
mod m20250825_000001_add_forgotten_after;
mod m20250824_234033_add_forgotten_message_field;
mod m20250825_085605_add_client_id_to_projects;
mod m20250825_create_message_files_table;
mod m20250826_000001_add_title_manual_flag;
mod m20250826_000001_use_is_forgotten_flag;
mod m20250826_000002_drop_tools_table;
mod m20250826_create_tool_usages_table;
mod m20250827_000001_add_role_to_users;
mod m20250827_add_domains_to_clients;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_tables::Migration),
            Box::new(m20250824_225443_create_sessions_table::Migration),
            Box::new(m20250824_create_file_uploads_table::Migration),
            Box::new(m20250825_000001_add_forgotten_after::Migration),
            Box::new(m20250824_234033_add_forgotten_message_field::Migration),
            Box::new(m20250825_085605_add_client_id_to_projects::Migration),
            Box::new(m20250825_create_message_files_table::Migration),
            Box::new(m20250826_000001_add_title_manual_flag::Migration),
            Box::new(m20250826_000001_use_is_forgotten_flag::Migration),
            Box::new(m20250826_000002_drop_tools_table::Migration),
            Box::new(m20250826_create_tool_usages_table::Migration),
            Box::new(m20250827_000001_add_role_to_users::Migration),
            Box::new(m20250827_add_domains_to_clients::Migration),
        ]
    }
}