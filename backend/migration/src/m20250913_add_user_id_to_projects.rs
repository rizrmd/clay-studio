use sea_orm::{ConnectionTrait, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // First, add user_id column as nullable
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .add_column(
                        ColumnDef::new(Projects::UserId).uuid().null(), // Initially nullable
                    )
                    .to_owned(),
            )
            .await?;

        // Assign existing projects to the first user in each client
        let conn = manager.get_connection();
        
        // Get all clients and their first users
        let clients = conn
            .query_all(Statement::from_string(
                manager.get_database_backend(),
                "SELECT DISTINCT p.client_id, u.id as first_user_id 
                 FROM projects p 
                 JOIN users u ON u.client_id = p.client_id 
                 WHERE p.user_id IS NULL 
                 ORDER BY p.client_id, u.id ASC"
                    .to_string(),
            ))
            .await?;

        // For each client, update projects to use the first user
        for client_row in clients {
            let client_id_str: String = client_row.try_get("", "client_id")?;
            let first_user_id_str: String = client_row.try_get("", "first_user_id")?;
            
            // Update all projects for this client to be owned by the first user
            conn.execute(Statement::from_string(
                manager.get_database_backend(),
                format!(
                    "UPDATE projects SET user_id = '{}' WHERE client_id = '{}' AND user_id IS NULL",
                    first_user_id_str, client_id_str
                ),
            ))
            .await?;
        }

        // Now make the column NOT NULL since all projects should have a user_id
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .modify_column(ColumnDef::new(Projects::UserId).uuid().not_null())
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
                            .name("fk_projects_user_id")
                            .from_tbl(Projects::Table)
                            .from_col(Projects::UserId)
                            .to_tbl(Users::Table)
                            .to_col(Users::Id)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create index on user_id for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_projects_user_id")
                    .table(Projects::Table)
                    .col(Projects::UserId)
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
                    .name("idx_projects_user_id")
                    .table(Projects::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the foreign key constraint
        manager
            .drop_foreign_key(
                ForeignKey::drop()
                    .name("fk_projects_user_id")
                    .table(Projects::Table)
                    .to_owned(),
            )
            .await?;

        // Drop the column
        manager
            .alter_table(
                Table::alter()
                    .table(Projects::Table)
                    .drop_column(Projects::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Projects {
    Table,
    UserId,
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Id,
}