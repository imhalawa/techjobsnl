using Microsoft.Data.Sqlite;

namespace TechJobsNL.Persistence.Sqlite;

/// <summary>Owns an open SQLite connection after its compatible schema has been verified.</summary>
public sealed class SqliteStore : IAsyncDisposable
{
    private readonly SqliteConnection _connection;

    internal SqliteStore(SqliteConnection connection)
    {
        _connection = connection;
    }

    /// <inheritdoc />
    public ValueTask DisposeAsync() => _connection.DisposeAsync();
}
