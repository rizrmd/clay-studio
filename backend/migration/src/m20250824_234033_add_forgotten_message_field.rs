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
                    .add_column_if_not_exists(
                        ColumnDef::new(Conversations::ForgottenAfterMessageId)
                            .string()
                            .null()
                    )
                    .to_owned(),
            )
            .await?;
            
        // Add foreign key constraint
        manager
            .create_foreign_key(
                ForeignKey::create()
                    .name("fk_conversations_forgotten_message")
                    .from(Conversations::Table, Conversations::ForgottenAfterMessageId)
                    .to(Messages::Table, Messages::Id)
                    .on_delete(ForeignKeyAction::SetNull)
                    .to_owned(),
            )
            .await?;
            
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop foreign key first
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_conversations_forgotten_message")
                    .table(Conversations::Table)
                    .to_owned(),
            )
            .await?;
            
        // Drop the column
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

#[derive(DeriveIden)]
enum Conversations {
    Table,
    ForgottenAfterMessageId,
}

#[derive(DeriveIden)]
enum Messages {
    Table,
    Id,
}