use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add forgotten_after_message_id column to conversations table
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::ForgottenAfterMessageId)
                            .string()
                            .null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add forgotten_count column to conversations table
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::ForgottenCount)
                            .integer()
                            .default(0)
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Add index for forgotten_after_message_id for faster lookups
        manager
            .create_index(
                Index::create()
                    .name("idx_conversations_forgotten_after")
                    .table(Conversations::Table)
                    .col(Conversations::ForgottenAfterMessageId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the index first (ignore if it doesn't exist)
        let _ = manager
            .drop_index(
                Index::drop()
                    .name("idx_conversations_forgotten_after")
                    .to_owned(),
            )
            .await;

        // Drop the forgotten_count column
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::ForgottenCount)
                    .to_owned(),
            )
            .await?;

        // Drop the forgotten_after_message_id column
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::ForgottenAfterMessageId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Conversations {
    Table,
    ForgottenAfterMessageId,
    ForgottenCount,
}
