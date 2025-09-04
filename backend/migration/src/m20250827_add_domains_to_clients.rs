use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Clients::Table)
                    .add_column(
                        ColumnDef::new(Clients::Domains)
                            .array(ColumnType::Text)
                            .null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Clients::Table)
                    .drop_column(Clients::Domains)
                    .to_owned(),
            )
            .await
    }
}

#[derive(Iden)]
enum Clients {
    Table,
    Domains,
}
