using System.Text.Json.Nodes;
using Dapper;
using TechJobsNL.Core.Domain;

namespace TechJobsNL.Persistence.Sqlite;

public sealed partial class SqliteStore
{
    public async Task<SavedVacancyToggleResult> ToggleSavedVacancyAsync(VacancyKey key, CancellationToken cancellationToken)
    {
        var transaction = await _connection.BeginTransactionAsync(cancellationToken).ConfigureAwait(false);
        try
        {
            const string existsSql = "select exists(select 1 from jobs where company_id=@CompanyId and source_id=@SourceId);";
            var values = new { CompanyId = key.CompanyId.Value, SourceId = key.SourceId.Value };
            if (await _connection.ExecuteScalarAsync<long>(new CommandDefinition(existsSql, values, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false) == 0)
                throw new KeyNotFoundException($"Vacancy {key.CompanyId.Value}/{key.SourceId.Value} does not exist.");

            var libraryJson = await _connection.ExecuteScalarAsync<string?>(new CommandDefinition("select library_json from analytics_state where id=1;", transaction: transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            var library = libraryJson is null ? new JsonObject() : JsonNode.Parse(libraryJson)?.AsObject() ?? throw new InvalidDataException("Library JSON has no root object.");
            var jobs = library["jobs"] as JsonArray ?? [];
            library["jobs"] = jobs;
            var existing = jobs.Select((node, index) => (node, index)).FirstOrDefault(item => IsKey(item.node, key));
            var saved = existing.node is null;
            if (saved) jobs.Add(new JsonObject { ["company_id"] = key.CompanyId.Value, ["source_id"] = key.SourceId.Value });
            else jobs.RemoveAt(existing.index);

            const string save = "insert into analytics_state (id, filters_json, library_json) values (1, '{}', @Library) on conflict(id) do update set library_json=excluded.library_json;";
            await _connection.ExecuteAsync(new CommandDefinition(save, new { Library = library.ToJsonString() }, transaction, cancellationToken: cancellationToken)).ConfigureAwait(false);
            await transaction.CommitAsync(cancellationToken).ConfigureAwait(false);
            return new SavedVacancyToggleResult(key, saved);
        }
        finally { await transaction.DisposeAsync().ConfigureAwait(false); }
    }

    public async Task<string?> GetLibraryJsonAsync(CancellationToken cancellationToken) =>
        await _connection.ExecuteScalarAsync<string?>(new CommandDefinition("select library_json from analytics_state where id=1;", cancellationToken: cancellationToken)).ConfigureAwait(false);

    private static bool IsKey(JsonNode? node, VacancyKey key) => node is JsonObject value &&
        string.Equals(value["company_id"]?.GetValue<string>(), key.CompanyId.Value, StringComparison.Ordinal) &&
        string.Equals(value["source_id"]?.GetValue<string>(), key.SourceId.Value, StringComparison.Ordinal);
}
