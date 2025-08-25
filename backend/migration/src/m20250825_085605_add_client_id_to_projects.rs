use sea_orm_migration::prelude::*;
use sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, add client_id column as nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .add_column(
                        ColumnDef::new(Projects::ClientId)
                            .uuid()
                            .null()  // Initially nullable
                    )
                    .to_owned(),
            )
            .await?;
            
        // Get the first active client to use as default
        let conn = manager.get_connection();
        let result = conn
            .query_one(Statement::from_string(
                manager.get_database_backend(),
                "SELECT id::text FROM clients WHERE status = 'active' LIMIT 1".to_string(),
            ))
            .await?;
            
        if let Some(row) = result {
            // Update existing projects to use this client_id
            let client_id: String = row.try_get("", "id")?;
            conn.execute(Statement::from_string(
                manager.get_database_backend(),
                format!("UPDATE projects SET client_id = '{}'::uuid", client_id),
            ))
            .await?;
        }
        
        // Now make the column NOT NULL
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .modify_column(
                        ColumnDef::new(Projects::ClientId)
                            .uuid()
                            .not_null()
                    )
                    .to_owned(),
            )
            .await?;
            
        // Add foreign key constraint
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .add_foreign_key(
                        TableForeignKey::new()
                            .name("fk_projects_client_id")
                            .from_tbl(Projects::Table)
                            .from_col(Projects::ClientId)
                            .to_tbl(Clients::Table)
                            .to_col(Clients::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade)
                    )
                    .to_owned(),
            )
            .await?;
            
        // Create index on client_id for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_client_id")
                    .table(Projects::Table)
                    .col(Projects::ClientId)
                    .to_owned(),
            )
            .await?;
            
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the index first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_projects_client_id")
                    .table(Projects::Table)
                    .to_owned(),
            )
            .await?;
            
        // Drop the foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_projects_client_id")
                    .table(Projects::Table)
                    .to_owned(),
            )
            .await?;
            
        // Drop the column
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .drop_column(Projects::ClientId)
                    .to_owned(),
            )
            .await?;
            
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    ClientId,
}

#[derive(DeriveIden)]
enum Clients {
    Table,
    Id,
}