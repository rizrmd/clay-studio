use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create message_files junction table
        manager
            .create_table(
                Table::create()
                    .table(MessageFiles::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(MessageFiles::Id)
                            .uuid()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(MessageFiles::MessageId).string().not_null())
                    .col(ColumnDef::new(MessageFiles::FileId).uuid().not_null())
                    .col(
                        ColumnDef::new(MessageFiles::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_message_files_file_id")
                            .from(MessageFiles::Table, MessageFiles::FileId)
                            .to(FileUploads::Table, FileUploads::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_message_files_message_id")
                    .table(MessageFiles::Table)
                    .col(MessageFiles::MessageId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_message_files_file_id")
                    .table(MessageFiles::Table)
                    .col(MessageFiles::FileId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(MessageFiles::Table).to_owned())
            .await
    }
}

#[derive(Iden)]
enum MessageFiles {
    Table,
    Id,
    MessageId,
    FileId,
    CreatedAt,
}

#[derive(Iden)]
enum FileUploads {
    Table,
    Id,
}
