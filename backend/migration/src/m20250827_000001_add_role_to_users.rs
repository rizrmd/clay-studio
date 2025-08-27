use sea_orm_migration::prelude::*;
use sea_orm::{ConnectionTrait, Statement};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Add role column with default value 'user'
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .add_column(
                        ColumnDef::new(Users::Role)
                            .string()
                            .not_null()
                            .default("user")
                    )
                    .to_owned(),
            )
            .await?;
            
        // Create index on role for better query performance
        manager
            .create_index(
                Index::create()
                    .name("idx_users_role")
                    .table(Users::Table)
                    .col(Users::Role)
                    .to_owned(),
            )
            .await?;
            
        // Set the first user of each client as admin
        let conn = manager.get_connection();
        
        // For each client, find the first user and make them admin
        conn.execute(Statement::from_string(
            manager.get_database_backend(),
            r#"
            UPDATE users 
            SET role = 'admin' 
            WHERE id IN (
                SELECT DISTINCT ON (client_id) id 
                FROM users 
                ORDER BY client_id, id
            )
            "#.to_string(),
        ))
        .await?;
            
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the index first
        manager
            .drop_index(
                Index::drop()
                    .name("idx_users_role")
                    .table(Users::Table)
                    .to_owned(),
            )
            .await?;
            
        // Drop the column
        manager
            .alter_table(
                Table::alter()
                    .table(Users::Table)
                    .drop_column(Users::Role)
                    .to_owned(),
            )
            .await?;
            
        Ok(())
    }
}

#[derive(DeriveIden)]
enum Users {
    Table,
    Role,
}