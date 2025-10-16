use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add file-specific columns to data_sources table
        manager
            .alter_table(
                Table::alter()
                    .table(DataSources::Table)
                    .add_column(
                        ColumnDef::new(DataSources::FilePath)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(DataSources::FileSize)
                            .big_integer()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(DataSources::FileType)
                            .text()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(DataSources::FileMetadata)
                            .json()
                            .null(),
                    )
                    .add_column(
                        ColumnDef::new(DataSources::ParsingOptions)
                            .json()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for file-based datasources
        manager
            .create_index(
                Index::create()
                    .name("idx_data_sources_file_path")
                    .table(DataSources::Table)
                    .col(DataSources::FilePath)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_data_source_type")
                    .table(DataSources::Table)
                    .col(DataSources::SourceType)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_data_sources_file_path")
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_data_source_type")
                    .to_owned(),
            )
            .await?;

        // Remove columns
        manager
            .alter_table(
                Table::alter()
                    .table(DataSources::Table)
                    .drop_column(DataSources::FilePath)
                    .drop_column(DataSources::FileSize)
                    .drop_column(DataSources::FileType)
                    .drop_column(DataSources::FileMetadata)
                    .drop_column(DataSources::ParsingOptions)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum DataSources {
    Table,
    Id,
    ProjectId,
    Name,
    SourceType,
    ConnectionConfig,
    SchemaInfo,
    PreviewData,
    TableList,
    LastTestedAt,
    IsActive,
    CreatedAt,
    // New columns for file support
    FilePath,
    FileSize,
    FileType,
    FileMetadata,
    ParsingOptions,
}