use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the tools table if it exists
        manager
            .drop_table(Table::drop().table(Tools::Table).if_exists().to_owned())
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Re-create the tools table if rolling back
        manager
            .create_table(
                Table::create()
                    .table(Tools::Table)
                    .if_not_exists()
                    .col(ColumnDef::new(Tools::Id).string().not_null().primary_key())
                    .col(ColumnDef::new(Tools::Name).string().not_null())
                    .col(ColumnDef::new(Tools::Category).string().not_null())
                    .col(ColumnDef::new(Tools::Description).text())
                    .col(ColumnDef::new(Tools::Parameters).json())
                    .col(ColumnDef::new(Tools::UsageExamples).json())
                    .col(
                        ColumnDef::new(Tools::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .to_owned(),
            )
            .await
    }
}

#[derive(DeriveIden)]
enum Tools {
    Table,
    Id,
    Name,
    Category,
    Description,
    Parameters,
    UsageExamples,
    IsActive,
}
