namespace TechJobsNL.Persistence.Sqlite;

/// <summary>Represents the exhaustive result of opening and migrating a local SQLite database.</summary>
public abstract record SqliteOpenResult
{
    /// <summary>Represents a successfully opened compatible database.</summary>
    public sealed record Opened : SqliteOpenResult
    {
        /// <summary>Initializes a successful open result.</summary>
        public Opened(SqliteStore store, bool migrationApplied, string? backupPath)
        {
            Store = store;
            MigrationApplied = migrationApplied;
            BackupPath = backupPath;
        }

        /// <summary>Gets the opened store.</summary>
        public SqliteStore Store { get; }

        /// <summary>Gets whether the Rust-compatible schema migration was applied.</summary>
        public bool MigrationApplied { get; }

        /// <summary>Gets the recovery backup path when an existing database was migrated.</summary>
        public string? BackupPath { get; }
    }

    /// <summary>Represents a migration failure after the original database was restored from its backup.</summary>
    public sealed record Recovered : SqliteOpenResult
    {
        /// <summary>Initializes a recovered migration failure result.</summary>
        public Recovered(string databasePath, string backupPath, string diagnostic)
        {
            DatabasePath = databasePath;
            BackupPath = backupPath;
            Diagnostic = diagnostic;
        }

        /// <summary>Gets the restored database path.</summary>
        public string DatabasePath { get; }

        /// <summary>Gets the preserved backup path.</summary>
        public string BackupPath { get; }

        /// <summary>Gets the migration failure diagnostic.</summary>
        public string Diagnostic { get; }
    }

    /// <summary>Represents a failure before a database could be safely migrated.</summary>
    public sealed record Failed : SqliteOpenResult
    {
        /// <summary>Initializes an unrecovered open failure result.</summary>
        public Failed(string databasePath, string diagnostic)
        {
            DatabasePath = databasePath;
            Diagnostic = diagnostic;
        }

        /// <summary>Gets the requested database path.</summary>
        public string DatabasePath { get; }

        /// <summary>Gets the failure diagnostic.</summary>
        public string Diagnostic { get; }
    }
}
