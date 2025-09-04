use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(FileUploads::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(FileUploads::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(FileUploads::ClientId).uuid().not_null())
                    .col(ColumnDef::new(FileUploads::ProjectId).string().not_null())
                    .col(ColumnDef::new(FileUploads::ConversationId).string())
                    .col(ColumnDef::new(FileUploads::FileName).string().not_null())
                    .col(
                        ColumnDef::new(FileUploads::OriginalName)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FileUploads::FilePath).string().not_null())
                    .col(
                        ColumnDef::new(FileUploads::FileSize)
                            .big_integer()
                            .not_null(),
                    )
                    .col(ColumnDef::new(FileUploads::MimeType).string())
                    .col(ColumnDef::new(FileUploads::Description).text())
                    .col(ColumnDef::new(FileUploads::AutoDescription).text())
                    .col(ColumnDef::new(FileUploads::FileContent).text()) // For text files, store content
                    .col(ColumnDef::new(FileUploads::Metadata).json())
                    .col(ColumnDef::new(FileUploads::UploadedBy).uuid())
                    .col(
                        ColumnDef::new(FileUploads::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(FileUploads::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_file_uploads_client_project")
                    .table(FileUploads::Table)
                    .col(FileUploads::ClientId)
                    .col(FileUploads::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_file_uploads_conversation")
                    .table(FileUploads::Table)
                    .col(FileUploads::ConversationId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(FileUploads::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum FileUploads {
    Table,
    Id,
    ClientId,
    ProjectId,
    ConversationId,
    FileName,
    OriginalName,
    FilePath,
    FileSize,
    MimeType,
    Description,
    AutoDescription,
    FileContent,
    Metadata,
    UploadedBy,
    CreatedAt,
    UpdatedAt,
}
