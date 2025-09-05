use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add deleted_at column to data_sources table for soft deletion
        manager
            .alter_table(
                Table::alter()
                    .table(DataSources::Table)
                    .add_column(
                        ColumnDef::new(DataSources::DeletedAt)
                            .timestamp_with_time_zone()
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Remove deleted_at column from data_sources table
        manager
            .alter_table(
                Table::alter()
                    .table(DataSources::Table)
                    .drop_column(DataSources::DeletedAt)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum DataSources {
    Table,
    DeletedAt,
}