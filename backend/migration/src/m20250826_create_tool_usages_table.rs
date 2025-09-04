use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create tool_usages table
        manager
            .create_table(
                Table::create()
                    .table(ToolUsages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ToolUsages::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(ToolUsages::MessageId).string().not_null())
                    .col(ColumnDef::new(ToolUsages::ToolName).string().not_null())
                    .col(ColumnDef::new(ToolUsages::Parameters).json())
                    .col(ColumnDef::new(ToolUsages::Output).json())
                    .col(ColumnDef::new(ToolUsages::ExecutionTimeMs).big_integer())
                    .col(
                        ColumnDef::new(ToolUsages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_tool_usages_message")
                            .from(ToolUsages::Table, ToolUsages::MessageId)
                            .to(Messages::Table, Messages::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on message_id for efficient queries
        manager
            .create_index(
                Index::create()
                    .name("idx_tool_usages_message")
                    .table(ToolUsages::Table)
                    .col(ToolUsages::MessageId)
                    .to_owned(),
            )
            .await?;

        // Create index on tool_name for filtering by tool
        manager
            .create_index(
                Index::create()
                    .name("idx_tool_usages_tool_name")
                    .table(ToolUsages::Table)
                    .col(ToolUsages::ToolName)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(ToolUsages::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum ToolUsages {
    Table,
    Id,
    MessageId,
    ToolName,
    Parameters,
    Output,
    ExecutionTimeMs,
    CreatedAt,
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
}
