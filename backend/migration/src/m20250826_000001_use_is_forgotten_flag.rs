use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add is_forgotten column to messages table
        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .add_column(
                        ColumnDef::new(Messages::IsForgotten)
                            .boolean()
                            .default(false)
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        // Add index for is_forgotten for faster filtering
        manager
            .create_index(
                Index::create()
                    .name("idx_messages_is_forgotten")
                    .table(Messages::Table)
                    .col(Messages::ConversationId)
                    .col(Messages::IsForgotten)
                    .to_owned(),
            )
            .await?;

        // Drop the old forgotten_after_message_id column from conversations
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::ForgottenAfterMessageId)
                    .to_owned(),
            )
            .await?;

        // Drop the old forgotten_count column if it exists
        let _ = manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::ForgottenCount)
                    .to_owned(),
            )
            .await;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the index
        manager
            .drop_index(
                Index::drop()
                    .name("idx_messages_is_forgotten")
                    .to_owned(),
            )
            .await?;

        // Remove is_forgotten column from messages
        manager
            .alter_table(
                Table::alter()
                    .table(Messages::Table)
                    .drop_column(Messages::IsForgotten)
                    .to_owned(),
            )
            .await?;

        // Restore the old columns
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::ForgottenAfterMessageId)
                            .string()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::ForgottenCount)
                            .integer()
                            .default(0)
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Messages {
    Table,
    ConversationId,
    IsForgotten,
}

#[derive(Iden)]
enum Conversations {
    Table,
    ForgottenAfterMessageId,
    ForgottenCount,
}