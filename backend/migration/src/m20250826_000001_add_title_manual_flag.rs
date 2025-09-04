use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add is_title_manually_set column to conversations table
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .add_column(
                        ColumnDef::new(Conversations::IsTitleManuallySet)
                            .boolean()
                            .not_null()
                            .default(false),
                    )
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the is_title_manually_set column
        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::IsTitleManuallySet)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(Iden)]
enum Conversations {
    Table,
    IsTitleManuallySet,
}
