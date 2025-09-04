use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create clients table first (no foreign keys)
        manager
            .create_table(
                Table::create()
                    .table(Clients::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Clients::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(Clients::Name).text().not_null())
                    .col(ColumnDef::new(Clients::Description).text())
                    .col(
                        ColumnDef::new(Clients::Status)
                            .text()
                            .not_null()
                            .default("pending"),
                    )
                    .col(ColumnDef::new(Clients::InstallPath).text().not_null())
                    .col(
                        ColumnDef::new(Clients::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Clients::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(ColumnDef::new(Clients::DeletedAt).timestamp_with_time_zone())
                    .col(ColumnDef::new(Clients::ClaudeToken).text())
                    .col(ColumnDef::new(Clients::Config).json_binary().not_null())
                    .to_owned(),
            )
            .await?;

        // Create users table with foreign key to clients
        manager
            .create_table(
                Table::create()
                    .table(Users::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Users::Id)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .default(Expr::cust("gen_random_uuid()")),
                    )
                    .col(ColumnDef::new(Users::ClientId).uuid().not_null())
                    .col(ColumnDef::new(Users::Username).text().not_null())
                    .col(ColumnDef::new(Users::Password).text().not_null())
                    .foreign_key(
                        ForeignKey::create()
                            .name("user_client_id_fkey")
                            .from(Users::Table, Users::ClientId)
                            .to(Clients::Table, Clients::Id),
                    )
                    .to_owned(),
            )
            .await?;

        // Create projects table
        manager
            .create_table(
                Table::create()
                    .table(Projects::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Projects::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Projects::Name).string().not_null())
                    .col(ColumnDef::new(Projects::Settings).json())
                    .col(ColumnDef::new(Projects::OrganizationSettings).json())
                    .col(
                        ColumnDef::new(Projects::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Projects::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .to_owned(),
            )
            .await?;

        // Create conversations table
        manager
            .create_table(
                Table::create()
                    .table(Conversations::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Conversations::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Conversations::ProjectId).string().not_null())
                    .col(ColumnDef::new(Conversations::Title).string())
                    .col(
                        ColumnDef::new(Conversations::MessageCount)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(
                        ColumnDef::new(Conversations::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .col(
                        ColumnDef::new(Conversations::UpdatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_conversations_project")
                            .from(Conversations::Table, Conversations::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create messages table
        manager
            .create_table(
                Table::create()
                    .table(Messages::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Messages::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Messages::ConversationId).string().not_null())
                    .col(ColumnDef::new(Messages::Content).text().not_null())
                    .col(ColumnDef::new(Messages::Role).string().not_null())
                    .col(ColumnDef::new(Messages::ClayToolsUsed).json())
                    .col(ColumnDef::new(Messages::ProcessingTimeMs).big_integer())
                    .col(
                        ColumnDef::new(Messages::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_messages_conversation")
                            .from(Messages::Table, Messages::ConversationId)
                            .to(Conversations::Table, Conversations::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create data_sources table
        manager
            .create_table(
                Table::create()
                    .table(DataSources::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(DataSources::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(DataSources::ProjectId).string().not_null())
                    .col(ColumnDef::new(DataSources::Name).string().not_null())
                    .col(ColumnDef::new(DataSources::SourceType).string().not_null())
                    .col(
                        ColumnDef::new(DataSources::ConnectionConfig)
                            .json()
                            .not_null(),
                    )
                    .col(ColumnDef::new(DataSources::SchemaInfo).json())
                    .col(ColumnDef::new(DataSources::PreviewData).json())
                    .col(ColumnDef::new(DataSources::TableList).json())
                    .col(ColumnDef::new(DataSources::LastTestedAt).timestamp_with_time_zone())
                    .col(
                        ColumnDef::new(DataSources::IsActive)
                            .boolean()
                            .not_null()
                            .default(true),
                    )
                    .col(
                        ColumnDef::new(DataSources::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::current_timestamp()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_data_sources_project")
                            .from(DataSources::Table, DataSources::ProjectId)
                            .to(Projects::Table, Projects::Id)
                            .on_delete(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes
        manager
            .create_index(
                Index::create()
                    .name("idx_conversations_project")
                    .table(Conversations::Table)
                    .col(Conversations::ProjectId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_messages_conversation")
                    .table(Messages::Table)
                    .col(Messages::ConversationId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_data_sources_project")
                    .table(DataSources::Table)
                    .col(DataSources::ProjectId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(Messages::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(DataSources::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Conversations::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Projects::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Users::Table).to_owned())
            .await?;
        manager
            .drop_table(Table::drop().table(Clients::Table).to_owned())
            .await?;
        Ok(())
    }
}

#[derive(Iden)]
enum Users {
    Table,
    Id,
    ClientId,
    Username,
    Password,
}

#[derive(Iden)]
enum Clients {
    Table,
    Id,
    Name,
    Description,
    Status,
    InstallPath,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    ClaudeToken,
    Config,
}

#[derive(Iden)]
enum Projects {
    Table,
    Id,
    Name,
    Settings,
    OrganizationSettings,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Conversations {
    Table,
    Id,
    ProjectId,
    Title,
    MessageCount,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    ConversationId,
    Content,
    Role,
    ClayToolsUsed,
    ProcessingTimeMs,
    CreatedAt,
}

#[derive(Iden)]
enum DataSources {
    Table,
    Id,
    ProjectId,
    Name,
    SourceType,
    ConnectionConfig,
    SchemaInfo,
    PreviewData,
    TableList,
    LastTestedAt,
    IsActive,
    CreatedAt,
}
